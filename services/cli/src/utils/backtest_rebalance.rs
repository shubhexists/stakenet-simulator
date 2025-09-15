use crate::error::CliError;
use futures::future::try_join_all;
use jito_steward::{
    Config,
    constants::TVC_ACTIVATION_EPOCH,
    score::{instant_unstake_validator, validator_score},
};
use num_traits::ToPrimitive;
use rand::prelude::IndexedRandom;
use rand::rng;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use sqlx::{Pool, Postgres};
use stakenet_simulator_db::{
    active_stake_jito_sol::ActiveStakeJitoSol, cluster_history::ClusterHistory,
    cluster_history_entry::ClusterHistoryEntry, epoch_rewards::EpochRewards,
    validator_history::ValidatorHistory, validator_history_entry::ValidatorHistoryEntry,
    withdraw_and_deposits::WithdrawsAndDeposits,
};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info};
use validator_history::ClusterHistory as JitoClusterHistory;

#[derive(Clone, Debug)]
pub struct RebalancingCycle {
    pub starting_total_lamports: u64,
    pub ending_total_lamports: u64,
}

#[derive(Debug, Clone)]
pub struct EpochWithdrawDepositStakeData {
    pub withdraw_stake: f64,
    pub deposit_stake: f64,
    pub active_balance: f64,
}

#[derive(Clone, Debug)]
pub struct ValidatorWithScore {
    pub vote_account: String,
    pub score: f64,
}

pub async fn rebalancing_simulation(
    db_connection: &Pool<Postgres>,
    steward_config: &Config,
    simulation_start_epoch: u16,
    simulation_end_epoch: u16,
    steward_cycle_rate: u16,
    number_of_validator_delegations: usize,
    instant_unstake_cap_bps: u32,
    validator_historical_start_offset: u16,
) -> Result<Vec<RebalancingCycle>, CliError> {
    let mut rebalancing_cycles = Vec::new();
    let mut current_cycle_start = simulation_start_epoch;

    // Tracks each validator's actual balance epoch to epoch
    let mut validator_balances: HashMap<String, u64> = HashMap::new();
    // Tracks the validator's score for the current steward cycle
    let mut validator_scores: HashMap<String, f64> = HashMap::new();

    let histories = ValidatorHistory::fetch_all(db_connection).await?;
    let cluster_history = ClusterHistory::fetch(db_connection).await?;
    let cluster_history_entries = ClusterHistoryEntry::fetch_all(db_connection).await?;
    let jito_cluster_history =
        cluster_history.convert_to_jito_cluster_history(cluster_history_entries);
    let jito_cluster_history = Arc::new(jito_cluster_history);

    info!("Fetching all validator history entries...");
    let all_entries = ValidatorHistoryEntry::fetch_all_records_between_epochs(
        db_connection,
        simulation_start_epoch
            .saturating_sub(validator_historical_start_offset)
            .into(),
        simulation_end_epoch.into(),
    )
    .await?;

    let withdraws_and_deposits_stakes = WithdrawsAndDeposits::get_details_for_epoch_range(
        db_connection,
        simulation_start_epoch.into(),
        simulation_end_epoch.into(),
    )
    .await?;

    let active_stake = ActiveStakeJitoSol::get_active_stakes_for_epoch_range(
        db_connection,
        simulation_start_epoch.into(),
        simulation_end_epoch.into(),
    )
    .await?;

    let epoch_map = build_epoch_map(withdraws_and_deposits_stakes, active_stake);
    let mut entries_by_validator: HashMap<String, Vec<ValidatorHistoryEntry>> = HashMap::new();
    for entry in all_entries {
        entries_by_validator
            .entry(entry.vote_pubkey.clone())
            .or_insert_with(Vec::new)
            .push(entry);
    }
    let entries_by_validator = Arc::new(entries_by_validator);
    info!(
        "Grouped {} validators' history entries",
        entries_by_validator.len()
    );

    let mut top_validators: Vec<ValidatorWithScore> = Vec::new();
    let mut total_lamports_staked = LAMPORTS_PER_SOL
        .checked_mul(number_of_validator_delegations as u64)
        .ok_or(CliError::ArithmeticError)?;

    let mut cycle_starting_lamports = 0u64;

    for current_epoch in simulation_start_epoch..simulation_end_epoch {
        info!("Processing epoch {}", current_epoch);

        let is_rebalancing_epoch =
            (current_epoch - simulation_start_epoch) % steward_cycle_rate == 0;

        let current_epoch_entries: HashMap<String, Vec<ValidatorHistoryEntry>> =
            entries_by_validator
                .iter()
                .map(|(vote_pubkey, entries)| {
                    let mut filtered_entries: Vec<ValidatorHistoryEntry> = entries
                        .iter()
                        .filter(|entry| entry.validator_history_entry.epoch <= current_epoch)
                        .cloned()
                        .collect();

                    filtered_entries.sort_by(|a, b| {
                        b.validator_history_entry
                            .epoch
                            .cmp(&a.validator_history_entry.epoch)
                    });

                    (vote_pubkey.clone(), filtered_entries)
                })
                .collect();

        let current_epoch_entries = Arc::new(current_epoch_entries);

        if is_rebalancing_epoch {
            (top_validators, cycle_starting_lamports) = process_steward_cycle(
                &histories,
                &current_epoch_entries,
                &jito_cluster_history,
                steward_config,
                current_epoch,
                simulation_start_epoch,
                steward_cycle_rate,
                simulation_end_epoch,
                number_of_validator_delegations,
                &mut current_cycle_start,
                &mut rebalancing_cycles,
                &mut validator_balances,
                &mut validator_scores,
                &mut total_lamports_staked,
                cycle_starting_lamports,
            )
            .await?;
        }

        if !top_validators.is_empty() {
            total_lamports_staked = process_epoch_cycle(
                db_connection,
                &top_validators,
                &histories,
                &current_epoch_entries,
                &jito_cluster_history,
                steward_config,
                current_epoch,
                is_rebalancing_epoch,
                instant_unstake_cap_bps,
                number_of_validator_delegations,
                &mut validator_balances,
                &epoch_map,
                total_lamports_staked,
            )
            .await?;
        }
    }

    if !validator_balances.is_empty() {
        let final_cycle_ending_lamports = validator_balances.values().sum::<u64>();
        let final_cycle_result = RebalancingCycle {
            starting_total_lamports: cycle_starting_lamports,
            ending_total_lamports: final_cycle_ending_lamports,
        };

        rebalancing_cycles.push(final_cycle_result);
    }

    Ok(rebalancing_cycles)
}

/// Processes a single steward cycle, selecting top validators and allocating stake.
async fn process_steward_cycle(
    histories: &[ValidatorHistory],
    current_epoch_entries: &Arc<HashMap<String, Vec<ValidatorHistoryEntry>>>,
    jito_cluster_history: &Arc<JitoClusterHistory>,
    steward_config: &Config,
    current_epoch: u16,
    simulation_start_epoch: u16,
    steward_cycle_rate: u16,
    simulation_end_epoch: u16,
    number_of_validator_delegations: usize,
    current_cycle_start: &mut u16,
    rebalancing_cycles: &mut Vec<RebalancingCycle>,
    validator_balances: &mut HashMap<String, u64>,
    validator_scores: &mut HashMap<String, f64>,
    total_lamports_staked: &mut u64,
    cycle_starting_lamports: u64,
) -> Result<(Vec<ValidatorWithScore>, u64), CliError> {
    let current_cycle_end = std::cmp::min(
        *current_cycle_start + steward_cycle_rate,
        simulation_end_epoch,
    );

    if *current_cycle_start > simulation_start_epoch {
        let cycle_ending_lamports = validator_balances.values().sum::<u64>();
        let cycle_result = RebalancingCycle {
            starting_total_lamports: cycle_starting_lamports,
            ending_total_lamports: cycle_ending_lamports,
        };

        info!(
            "Completed cycle: {:.3} SOL -> {:.3} SOL (return: {:.2}%)",
            cycle_starting_lamports as f64 / LAMPORTS_PER_SOL as f64,
            cycle_ending_lamports as f64 / LAMPORTS_PER_SOL as f64,
            ((cycle_ending_lamports as f64 / cycle_starting_lamports as f64) - 1.0) * 100.0
        );

        rebalancing_cycles.push(cycle_result);
        *total_lamports_staked = cycle_ending_lamports;
    }

    let top_validators = top_validators_for_epoch(
        histories,
        current_epoch_entries,
        jito_cluster_history,
        steward_config,
        current_epoch,
        number_of_validator_delegations,
    )
    .await?;

    let stake_per_validator: u64 = *total_lamports_staked / top_validators.len() as u64;

    validator_balances.clear();
    validator_scores.clear();
    for validator in &top_validators {
        validator_balances.insert(validator.vote_account.clone(), stake_per_validator);
        validator_scores.insert(validator.vote_account.clone(), validator.score);
    }

    let new_cycle_starting_lamports = *total_lamports_staked;

    info!(
        "Initialized {} validators with {:.3} SOL each, total: {:.3} SOL",
        top_validators.len(),
        stake_per_validator as f64 / LAMPORTS_PER_SOL as f64,
        *total_lamports_staked as f64 / LAMPORTS_PER_SOL as f64
    );

    *current_cycle_start = current_cycle_end;

    Ok((top_validators, new_cycle_starting_lamports))
}

/// Apply epoch-specific stake changes based on deposit/withdraw ratios
fn apply_epoch_stake_changes(
    validator_balances: &mut HashMap<String, u64>, // This has validator and it's corresponding balances for that epoch
    epoch_map: &HashMap<u64, Vec<EpochWithdrawDepositStakeData>>, // map of epochs to vec of withdraw/deposit stake of that epoch
    current_epoch: u16,                                           // current epoch
) -> Result<(), CliError> {
    let current_epoch_u64 = current_epoch as u64;

    // Get the epoch data for current epoch
    if let Some(epoch_data_vec) = epoch_map.get(&current_epoch_u64) {
        let num_records = epoch_data_vec.len();

        if num_records == 0 {
            return Ok(()); // No data for this epoch so we will skip this
        }

        // Get all validator vote accounts as a vector
        let validator_accounts: Vec<String> = validator_balances.keys().cloned().collect();
        if validator_accounts.is_empty() {
            return Ok(()); // No validators to adjust so we will skip this
        }

        // Randomly select validators (with replacement if needed)
        let mut rng = rng();
        let selected_validators: Vec<String> = (0..num_records)
            .map(|_| {
                validator_accounts
                    .choose(&mut rng)
                    .unwrap_or(&validator_accounts[0])
                    .clone()
            })
            .collect();

        info!(
            "Epoch {}: Applying stake changes to {} randomly selected validators from {} total validators",
            current_epoch,
            num_records,
            validator_accounts.len()
        );

        // Apply stake changes for each selected validator and corresponding epoch data
        for (validator_account, epoch_data) in selected_validators.iter().zip(epoch_data_vec.iter())
        {
            if let Some(current_balance) = validator_balances.get_mut(validator_account) {
                let current_balance_f64 = *current_balance as f64;

                // Skip if active balance is zero to avoid division by zero
                if epoch_data.active_balance == 0.0 {
                    continue;
                }

                // Find net stake change for a given validator this epoch
                let net_stake_change = epoch_data.deposit_stake - epoch_data.withdraw_stake;
                // Calculate the ratio of that stake to the active stake on the jitoSOL pool
                // during that epoch. This is used to normalize the depoist stake or withdraw
                // stake amounts to the pool values in this back test
                let stake_change_ratio = net_stake_change / epoch_data.active_balance;

                // The amount of stake that should be adjusted for the validator
                let stake_adjustment = current_balance_f64 * stake_change_ratio;
                let new_balance = current_balance_f64 + stake_adjustment;

                // Ensure balance doesn't go negative
                let final_balance = new_balance.max(0.0) as u64;

                *current_balance = final_balance;

                info!(
                    "Epoch {}: Adjusted validator {} balance by {:.6} SOL ({:.2}% change) - Balance: {:.6} -> {:.6} SOL",
                    current_epoch,
                    validator_account,
                    stake_adjustment / LAMPORTS_PER_SOL as f64,
                    stake_change_ratio * 100.0,
                    current_balance_f64 / LAMPORTS_PER_SOL as f64,
                    final_balance as f64 / LAMPORTS_PER_SOL as f64
                );
            }
        }
    }

    Ok(())
}

/// Processes a single epoch within the current steward cycle, handling instant unstaking if
/// necessary, and simulating epoch returns.
async fn process_epoch_cycle(
    db_connection: &Pool<Postgres>,
    top_validators: &[ValidatorWithScore],
    histories: &[ValidatorHistory],
    current_epoch_entries: &Arc<HashMap<String, Vec<ValidatorHistoryEntry>>>,
    jito_cluster_history: &Arc<JitoClusterHistory>,
    steward_config: &Config,
    current_epoch: u16,
    is_rebalancing_epoch: bool,
    instant_unstake_cap_bps: u32,
    number_of_validator_delegations: usize,
    validator_balances: &mut HashMap<String, u64>,
    epoch_map: &HashMap<u64, Vec<EpochWithdrawDepositStakeData>>,
    mut total_lamports_staked: u64,
) -> Result<u64, CliError> {
    // Apply epoch-specific stake changes first (before any other processing)
    apply_epoch_stake_changes(validator_balances, epoch_map, current_epoch)?;
    total_lamports_staked = validator_balances.values().sum::<u64>();

    if !is_rebalancing_epoch {
        let current_validator_list: Vec<String> = top_validators
            .iter()
            .map(|v| v.vote_account.clone())
            .collect();

        let validators_to_unstake = calculate_unstake_per_epoch(
            &current_validator_list,
            histories,
            current_epoch_entries,
            jito_cluster_history,
            steward_config,
            current_epoch,
        )
        .await?;

        if !validators_to_unstake.is_empty() {
            handle_instant_unstaking(
                &validators_to_unstake,
                validator_balances,
                top_validators,
                instant_unstake_cap_bps,
                number_of_validator_delegations,
                &mut total_lamports_staked,
            )?;
        }
    }

    let total_before_rewards = validator_balances.values().sum::<u64>();
    simulate_epoch_returns(db_connection, validator_balances, current_epoch).await?;
    let total_after_rewards = validator_balances.values().sum::<u64>();
    total_lamports_staked = total_after_rewards;

    info!(
        "Epoch {} returns: {:.6} SOL -> {:.6} SOL (gain: {:.6} SOL)",
        current_epoch,
        total_before_rewards as f64 / LAMPORTS_PER_SOL as f64,
        total_after_rewards as f64 / LAMPORTS_PER_SOL as f64,
        (total_after_rewards - total_before_rewards) as f64 / LAMPORTS_PER_SOL as f64
    );

    Ok(total_lamports_staked)
}

/// Handles instant unstaking of validators
fn handle_instant_unstaking(
    validators_to_unstake: &[String],
    validator_balances: &mut HashMap<String, u64>,
    top_validators: &[ValidatorWithScore],
    instant_unstake_cap_bps: u32,
    number_of_validator_delegations: usize,
    total_lamports_staked: &mut u64,
) -> Result<(), CliError> {
    let max_unstake_amount = (*total_lamports_staked as u128 * instant_unstake_cap_bps as u128
        / 10000)
        .min(u64::MAX as u128) as u64;

    let mut validators_with_scores: Vec<(String, f64, u64)> = validators_to_unstake
        .iter()
        .filter_map(|vote_account| {
            let score = top_validators
                .iter()
                .find(|v| &v.vote_account == vote_account)
                .map(|v| v.score)
                .unwrap_or(0.0);

            validator_balances
                .get(vote_account)
                .map(|&balance| (vote_account.clone(), score, balance))
        })
        .collect();

    validators_with_scores
        .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut actual_validators_to_unstake = Vec::new();
    let mut total_unstaked_amount = 0u64;

    for (vote_account, _score, balance) in validators_with_scores {
        let potential_unstake = total_unstaked_amount + balance;

        if potential_unstake <= max_unstake_amount {
            actual_validators_to_unstake.push(vote_account);
            total_unstaked_amount += balance;
        } else {
            break;
        }
    }

    for validator in &actual_validators_to_unstake {
        if let Some(current_balance) = validator_balances.get_mut(validator) {
            total_unstaked_amount = total_unstaked_amount
                .checked_sub(*current_balance)
                .ok_or(CliError::ArithmeticError)?;

            let unstaked_amount = *current_balance;
            *current_balance = 0;

            total_unstaked_amount = total_unstaked_amount
                .checked_add(unstaked_amount)
                .ok_or(CliError::ArithmeticError)?;
        }
    }

    if total_unstaked_amount > 0 {
        let remaining_validator_count = top_validators.len() - actual_validators_to_unstake.len();

        if remaining_validator_count > 0 {
            let new_target_balance_per_validator =
                *total_lamports_staked / remaining_validator_count as u64;

            let mut validators_by_score: Vec<_> = top_validators
                .iter()
                .filter(|v| !actual_validators_to_unstake.contains(&v.vote_account))
                .collect();
            validators_by_score.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let mut remaining_to_distribute = total_unstaked_amount;

            for validator in &validators_by_score {
                if remaining_to_distribute == 0 {
                    break;
                }

                if let Some(current_balance) = validator_balances.get_mut(&validator.vote_account) {
                    let deficit = if *current_balance < new_target_balance_per_validator {
                        new_target_balance_per_validator - *current_balance
                    } else {
                        0
                    };

                    let amount_to_add = deficit.min(remaining_to_distribute);
                    if amount_to_add > 0 {
                        *current_balance = current_balance
                            .checked_add(amount_to_add)
                            .ok_or(CliError::ArithmeticError)?;

                        remaining_to_distribute = remaining_to_distribute
                            .checked_sub(amount_to_add)
                            .ok_or(CliError::ArithmeticError)?;
                    }
                }
            }

            info!(
                "New target balance per validator: {:.3} SOL (was {:.3} SOL per validator)",
                new_target_balance_per_validator as f64 / LAMPORTS_PER_SOL as f64,
                (*total_lamports_staked / number_of_validator_delegations as u64) as f64
                    / LAMPORTS_PER_SOL as f64
            );
        }

        info!(
            "Unstaked total: {:.3} SOL from {} validators (max allowed: {:.3} SOL), redistributed to remaining {} validators",
            total_unstaked_amount as f64 / LAMPORTS_PER_SOL as f64,
            actual_validators_to_unstake.len(),
            max_unstake_amount as f64 / LAMPORTS_PER_SOL as f64,
            number_of_validator_delegations - actual_validators_to_unstake.len()
        );
    }

    Ok(())
}

async fn simulate_epoch_returns(
    db_connection: &Pool<Postgres>,
    validator_balances: &mut HashMap<String, u64>,
    current_epoch: u16,
) -> Result<(), CliError> {
    let validator_list: Vec<String> = validator_balances.keys().cloned().collect();

    let rewards =
        EpochRewards::fetch_for_single_epoch(db_connection, &validator_list, current_epoch.into())
            .await?;

    for reward in rewards {
        if let Some(current_balance) = validator_balances.get_mut(&reward.vote_pubkey) {
            let new_balance = reward.stake_after_epoch(*current_balance);
            *current_balance = new_balance;
        }
    }

    Ok(())
}

/// Determines which, if any, of the _selected_validators_ should be unstaked. Returns a vec of
/// their vote account pubkeys.
///
/// # Arguments
/// - `selected_validators`: The orignal cohort of validators that received delegations for this
/// steward cycle
/// - `histories`: ValidatorHistory (metadata) records for all validators
/// - `entries_by_validator`: Mapping of validator to their ValidatorHistoryEntry records
/// - `jito_cluster_history`: The ClusterHistory
/// - `steward_config`: Steward `Config` being used for this back testing
/// - `epoch`: Target epoch we're checking against
async fn calculate_unstake_per_epoch(
    selected_validators: &[String],
    histories: &[ValidatorHistory],
    entries_by_validator: &Arc<HashMap<String, Vec<ValidatorHistoryEntry>>>,
    jito_cluster_history: &Arc<JitoClusterHistory>,
    steward_config: &Config,
    epoch: u16,
) -> Result<Vec<String>, CliError> {
    let epoch_start_slot = epoch as u64 * 432_000;
    let unstake_tasks: Vec<_> = selected_validators
        .iter()
        .filter_map(|validator_vote_account| {
            histories
                .iter()
                .find(|vh| vh.vote_account == *validator_vote_account)
                .map(|validator_history| {
                    let validator_history = validator_history.clone();
                    let entries_by_validator = Arc::clone(entries_by_validator);
                    let jito_cluster_history = Arc::clone(jito_cluster_history);
                    let steward_config = steward_config.clone();
                    let vote_account = validator_vote_account.clone();
                    tokio::task::spawn_blocking(move || {
                        let unstake_result = calculate_instant_unstake(
                            validator_history,
                            &entries_by_validator,
                            &jito_cluster_history,
                            &steward_config,
                            epoch_start_slot,
                            epoch,
                        );
                        (vote_account, unstake_result)
                    })
                })
        })
        .collect();
    let unstake_results = try_join_all(unstake_tasks)
        .await
        .map_err(|e| CliError::TaskJoinError(e))?;

    let mut validators_to_unstake = Vec::new();

    for (vote_account, result) in unstake_results {
        match result {
            Ok(should_unstake) => {
                if should_unstake {
                    validators_to_unstake.push(vote_account);
                }
            }
            Err(e) => {
                error!(
                    "Error checking instant unstake for validator {} at epoch {}: {:?}",
                    vote_account, epoch, e
                );
            }
        }
    }
    Ok(validators_to_unstake)
}

async fn top_validators_for_epoch(
    histories: &[ValidatorHistory],
    entries_by_validator: &Arc<HashMap<String, Vec<ValidatorHistoryEntry>>>,
    jito_cluster_history: &Arc<JitoClusterHistory>,
    steward_config: &Config,
    scoring_epoch: u16,
    number_of_validators: usize,
) -> Result<Vec<ValidatorWithScore>, CliError> {
    info!("Scoring validators for epoch {}", scoring_epoch);

    let scoring_tasks: Vec<_> = histories
        .iter()
        .map(|validator_history| {
            let validator_history = validator_history.clone();
            let entries_by_validator = Arc::clone(entries_by_validator);
            let jito_cluster_history = Arc::clone(jito_cluster_history);
            let steward_config = steward_config.clone();
            tokio::task::spawn_blocking(move || {
                score_validator(
                    validator_history,
                    &entries_by_validator,
                    &jito_cluster_history,
                    &steward_config,
                    scoring_epoch,
                )
            })
        })
        .collect();

    let scoring_results = try_join_all(scoring_tasks)
        .await
        .map_err(|e| CliError::TaskJoinError(e))?;

    let mut scored_validators: Vec<(String, f64)> = scoring_results
        .into_iter()
        .filter_map(|result| result.ok())
        .collect();

    scored_validators.sort_by(|a, b| b.1.total_cmp(&a.1));

    // Get only the validators that have score more than zero, maximum number of validators can be "number_of_validators"
    let top_validators: Vec<ValidatorWithScore> = scored_validators
        .into_iter()
        .filter(|(_, score)| *score > 0.0)
        .take(number_of_validators)
        .map(|(vote_account, score)| ValidatorWithScore {
            vote_account,
            score,
        })
        .collect();

    Ok(top_validators)
}

pub fn score_validator(
    validator_history: ValidatorHistory,
    entries_by_validator: &HashMap<String, Vec<ValidatorHistoryEntry>>,
    jito_cluster_history: &JitoClusterHistory,
    steward_config: &Config,
    current_epoch: u16,
) -> Result<(String, f64), CliError> {
    let vote_account = validator_history.vote_account.clone();

    // Get entries for this validator from the pre-fetched map
    let mut entries = entries_by_validator
        .get(&vote_account)
        .cloned()
        .unwrap_or_default();

    // Convert DB structures into on-chain structures
    let jito_validator_history = validator_history.convert_to_jito_validator_history(&mut entries);

    // Score the validator
    let score_result = validator_score(
        &jito_validator_history,
        jito_cluster_history,
        &steward_config,
        current_epoch,
        TVC_ACTIVATION_EPOCH,
    );
    match score_result {
        Ok(score) => Ok((vote_account, score.score)),
        Err(_) => {
            // TBD: Should we log this error? There are many validators that will error out
            // error!(
            //     "Erroring scoring validator {}: {}",
            //     jito_validator_history.vote_account, q
            // );
            Ok((vote_account, 0.0))
        }
    }
}

fn calculate_instant_unstake(
    validator_history: ValidatorHistory,
    entries_by_validator: &HashMap<String, Vec<ValidatorHistoryEntry>>,
    jito_cluster_history: &JitoClusterHistory,
    config: &Config,
    epoch_start_slot: u64,
    current_epoch: u16,
) -> Result<bool, CliError> {
    let vote_account = validator_history.vote_account.clone();
    let mut entries = entries_by_validator
        .get(&vote_account)
        .cloned()
        .unwrap_or_default();

    let jito_validator_history = validator_history.convert_to_jito_validator_history(&mut entries);

    let unstake_result = instant_unstake_validator(
        &jito_validator_history,
        jito_cluster_history,
        config,
        epoch_start_slot,
        current_epoch,
        TVC_ACTIVATION_EPOCH,
    );

    match unstake_result {
        Ok(unstake) => Ok(unstake.instant_unstake),
        Err(_) => {
            error!(
                "Error calculating instant unstake for validator {}",
                jito_validator_history.vote_account
            );
            // TBD: Is there is an error in calculating, we should NOT unstake the validator?
            Ok(false)
        }
    }
}

pub fn build_epoch_map(
    withdraws_and_deposits: Vec<WithdrawsAndDeposits>,
    active_stake: Vec<ActiveStakeJitoSol>,
) -> HashMap<u64, Vec<EpochWithdrawDepositStakeData>> {
    let mut epoch_map: HashMap<u64, Vec<EpochWithdrawDepositStakeData>> = HashMap::new();
    let mut active_by_epoch: HashMap<u64, f64> = HashMap::new();
    for stake in active_stake {
        let balance = stake.balance.to_f64().unwrap_or(0.0);
        *active_by_epoch.entry(stake.epoch).or_insert(0.0) += balance;
    }

    for wd in withdraws_and_deposits {
        let active_balance = active_by_epoch.get(&wd.epoch).cloned().unwrap_or(0.0);

        epoch_map
            .entry(wd.epoch)
            .or_insert_with(Vec::new)
            .push(EpochWithdrawDepositStakeData {
                withdraw_stake: wd.withdraw_stake.to_f64().unwrap_or(0.0),
                deposit_stake: wd.deposit_stake.to_f64().unwrap_or(0.0),
                active_balance,
            });
    }

    epoch_map
}
