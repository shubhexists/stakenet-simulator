use crate::error::CliError;
use futures::future::try_join_all;
use jito_steward::{
    Config,
    constants::TVC_ACTIVATION_EPOCH,
    score::{instant_unstake_validator, validator_score},
};
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use sqlx::{Pool, Postgres};
use stakenet_simulator_db::{
    cluster_history::ClusterHistory, cluster_history_entry::ClusterHistoryEntry,
    epoch_rewards::EpochRewards, validator_history::ValidatorHistory,
    validator_history_entry::ValidatorHistoryEntry,
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

#[derive(Clone, Debug)]
pub struct ValidatorWithScore {
    pub vote_account: String,
    pub score: f64,
}

// Helper function to log all validator balances for an epoch
// Just for debug purposes
fn log_validator_balances_for_epoch(
    validator_balances: &HashMap<String, u64>,
    epoch: u16,
    context: &str,
) {
    let total_balance: u64 = validator_balances.values().sum();

    info!("=== EPOCH {} VALIDATOR BALANCES ({}) ===", epoch, context);
    info!(
        "Total staked across all validators: {:.6} SOL",
        total_balance as f64 / LAMPORTS_PER_SOL as f64
    );
    info!("Number of active validators: {}", validator_balances.len());

    let mut sorted_validators: Vec<_> = validator_balances.iter().collect();
    sorted_validators.sort_by(|a, b| b.1.cmp(a.1));

    for (i, (vote_account, balance)) in sorted_validators.iter().enumerate() {
        info!(
            "  #{:2} {} - {:.6} SOL ({} lamports)",
            i + 1,
            &vote_account[..8],
            **balance as f64 / LAMPORTS_PER_SOL as f64,
            balance
        );
    }
    info!("=== END EPOCH {} BALANCES ===", epoch);
}

pub async fn rebalancing_simulation(
    db_connection: &Pool<Postgres>,
    steward_config: &Config,
    simulation_start_epoch: u16,
    simulation_end_epoch: u16,
    steward_cycle_rate: u16,
    number_of_validator_delegations: usize,
    instant_unstake_cap_bps: u32,
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
    let all_entries =
        ValidatorHistoryEntry::fetch_all_validator_history_entries(db_connection).await?;
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

        if is_rebalancing_epoch {
            let current_cycle_end = std::cmp::min(
                current_cycle_start + steward_cycle_rate,
                simulation_end_epoch,
            );

            if current_cycle_start > simulation_start_epoch {
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
                total_lamports_staked = cycle_ending_lamports;
            }

            top_validators = top_validators_for_epoch(
                &histories,
                &entries_by_validator,
                &jito_cluster_history,
                steward_config,
                current_epoch,
                number_of_validator_delegations,
            )
            .await?;

            let stake_per_validator: u64 = total_lamports_staked / top_validators.len() as u64;

            validator_balances.clear();
            validator_scores.clear();
            for validator in &top_validators {
                validator_balances.insert(validator.vote_account.clone(), stake_per_validator);
                validator_scores.insert(validator.vote_account.clone(), validator.score);
            }

            cycle_starting_lamports = total_lamports_staked;

            info!(
                "Initialized {} validators with {:.3} SOL each, total: {:.3} SOL",
                top_validators.len(),
                stake_per_validator as f64 / LAMPORTS_PER_SOL as f64,
                total_lamports_staked as f64 / LAMPORTS_PER_SOL as f64
            );

            current_cycle_start = current_cycle_end;
        }

        if !top_validators.is_empty() {
            if !is_rebalancing_epoch {
                /////////////////////// Regular Epoch checks and rebalances ///////////////////////
                let current_validator_list: Vec<String> = top_validators
                    .iter()
                    .map(|v| v.vote_account.clone())
                    .collect();

                let validators_to_unstake = calculate_unstake_per_epoch(
                    &current_validator_list,
                    &histories,
                    &entries_by_validator,
                    &jito_cluster_history,
                    steward_config,
                    current_epoch,
                )
                .await?;

                if !validators_to_unstake.is_empty() {
                    handle_instant_unstaking(
                        &validators_to_unstake,
                        &mut validator_balances,
                        &top_validators,
                        instant_unstake_cap_bps,
                        number_of_validator_delegations,
                        &mut total_lamports_staked,
                    )?;
                }
            }

            let total_before_rewards = validator_balances.values().sum::<u64>();
            simulate_epoch_returns(db_connection, &mut validator_balances, current_epoch).await?;
            let total_after_rewards = validator_balances.values().sum::<u64>();
            total_lamports_staked = total_after_rewards;

            info!(
                "Epoch {} returns: {:.6} SOL -> {:.6} SOL (gain: {:.6} SOL)",
                current_epoch,
                total_before_rewards as f64 / LAMPORTS_PER_SOL as f64,
                total_after_rewards as f64 / LAMPORTS_PER_SOL as f64,
                (total_after_rewards - total_before_rewards) as f64 / LAMPORTS_PER_SOL as f64
            );

            log_validator_balances_for_epoch(
                &validator_balances,
                current_epoch,
                "AFTER EPOCH REWARDS",
            );
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

fn handle_instant_unstaking(
    validators_to_unstake: &[String],
    validator_balances: &mut HashMap<String, u64>,
    top_validators: &[ValidatorWithScore],
    instant_unstake_cap_bps: u32,
    number_of_validator_delegations: usize,
    total_lamports_staked: &mut u64,
) -> Result<(), CliError> {
    let mut total_unstaked_amount = 0u64;

    for validator in validators_to_unstake {
        if let Some(current_balance) = validator_balances.get_mut(validator) {
            let unstake_amount = (*current_balance as u128 * instant_unstake_cap_bps as u128
                / 10000)
                .min(u64::MAX as u128) as u64;

            *current_balance = current_balance
                .checked_sub(unstake_amount)
                .ok_or(CliError::ArithmeticError)?;

            total_unstaked_amount = total_unstaked_amount
                .checked_add(unstake_amount)
                .ok_or(CliError::ArithmeticError)?;
        }
    }

    if total_unstaked_amount > 0 {
        let remaining_validator_count = top_validators.len() - validators_to_unstake.len();

        if remaining_validator_count > 0 {
            let new_target_balance_per_validator =
                *total_lamports_staked / remaining_validator_count as u64;

            let mut validators_by_score: Vec<_> = top_validators
                .iter()
                .filter(|v| !validators_to_unstake.contains(&v.vote_account))
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

                        info!(
                            "Redistributed {:.3} SOL to validator {} (balance now: {:.3} SOL)",
                            amount_to_add as f64 / LAMPORTS_PER_SOL as f64,
                            validator.vote_account,
                            *current_balance as f64 / LAMPORTS_PER_SOL as f64
                        );
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
            "Unstaked total: {:.3} SOL from {} validators, redistributed to remaining {} validators",
            total_unstaked_amount as f64 / LAMPORTS_PER_SOL as f64,
            validators_to_unstake.len(),
            number_of_validator_delegations - validators_to_unstake.len()
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
            let old_balance = *current_balance;
            let new_balance = reward.stake_after_epoch(*current_balance);
            *current_balance = new_balance;

            let reward_amount = new_balance.saturating_sub(old_balance);
            if reward_amount > 0 {
                info!(
                    "Validator {} earned {:.6} SOL in epoch {} ({} -> {} lamports)",
                    &reward.vote_pubkey[..8],
                    reward_amount as f64 / LAMPORTS_PER_SOL as f64,
                    current_epoch,
                    old_balance,
                    new_balance
                );
            }
        }
    }

    Ok(())
}

/// Determiens which, if any, of the _selected_validators_ should be unstaked. Returns a vec of 
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
    info!(
        "Checking instant unstake for epoch {} on {} validators",
        epoch,
        selected_validators.len()
    );
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
            error!(
                "Erroring scoring validator {}",
                jito_validator_history.vote_account
            );
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
