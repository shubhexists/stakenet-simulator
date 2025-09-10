use crate::{
    dune::{
        WITHDRAW_DEPOSIT_TRANSACTIONS_QUERY, WithdrawAndDepositsRow, execute_dune_query,
        fetch_dune_query, wait_for_query_execution,
    },
    errors::EpochRewardsTrackerError,
};
use num_traits::FromPrimitive;
use sqlx::{Pool, Postgres, types::BigDecimal};
use stakenet_simulator_db::{
    active_stake_jito_sol::ActiveStakeJitoSol, withdraw_and_deposits::WithdrawsAndDeposits,
};
use std::collections::HashMap;
use tracing::info;

/// This command assumes that the "active_stake_jito_sol" is already populated
pub async fn withdraw_and_deposits(db: &Pool<Postgres>) -> Result<(), EpochRewardsTrackerError> {
    let execute_client = execute_dune_query(WITHDRAW_DEPOSIT_TRANSACTIONS_QUERY)
        .await
        .map_err(|_| EpochRewardsTrackerError::DuneApiError)?;

    wait_for_query_execution(&execute_client.execution_id).await?;
    let results: Vec<WithdrawAndDepositsRow> = fetch_dune_query(execute_client.execution_id)
        .await
        .map_err(|_| EpochRewardsTrackerError::DuneApiError)?;

    let total_stakes: Vec<ActiveStakeJitoSol> =
        ActiveStakeJitoSol::get_all_active_stakes(db).await?;

    let mut stake_map: HashMap<u64, ActiveStakeJitoSol> =
        total_stakes.into_iter().map(|s| (s.epoch, s)).collect();

    let mut merged: Vec<WithdrawsAndDeposits> = Vec::new();
    for row in results {
        if let Some(stake) = stake_map.remove(&row.epoch) {
            merged.push(WithdrawsAndDeposits::new(
                row.epoch,
                BigDecimal::from_f64(row.deposit_sol).unwrap_or_else(|| BigDecimal::from(0)),
                BigDecimal::from_f64(row.withdraw_stake).unwrap_or_else(|| BigDecimal::from(0)),
                BigDecimal::from_f64(row.deposit_stake).unwrap_or_else(|| BigDecimal::from(0)),
                BigDecimal::from_f64(row.withdraw_sol).unwrap_or_else(|| BigDecimal::from(0)),
                stake.balance,
            ))
        }
    }

    if merged.is_empty() {
        info!("No records to process.");
    } else {
        info!("Processing records...");
        WithdrawsAndDeposits::bulk_insert(db, merged).await?;
        info!("Processing complete. Records inserted/updated.");
    }

    Ok(())
}
