use crate::utils::wait_for_query_execution;
use crate::{
    error::CliError,
    utils::{execute_dune_query, fetch_dune_query},
};
use sqlx::{Pool, Postgres, types::BigDecimal};
use stakenet_simulator_db::inactive_stake_jito_sol::InactiveStakeJitoSol;
use std::collections::HashSet;
use std::str::FromStr;
use tracing::info;

pub async fn fetch_inactive_stake(db: &Pool<Postgres>) -> Result<(), CliError> {
    let execute_client = execute_dune_query(5571499)
        .await
        .map_err(|_| CliError::DuneApiError)?;

    if execute_client.execution_id.is_empty() {
        return Err(CliError::EmptyExecutionId);
    }

    wait_for_query_execution(&execute_client.execution_id).await?;

    let results = fetch_dune_query(execute_client.execution_id)
        .await
        .map_err(|_| CliError::DuneApiError)?;

    let existing_records = InactiveStakeJitoSol::fetch_all_records(db).await?;
    let existing_epochs: HashSet<u64> = existing_records.into_iter().map(|r| r.epoch).collect();

    let new_records: Vec<InactiveStakeJitoSol> = results
        .into_iter()
        .filter(|row| !existing_epochs.contains(&row.approx_epoch))
        .map(|row| {
            let day_date = row.day.chars().take(10).collect::<String>();
            InactiveStakeJitoSol {
                id: format!("{}-{}", row.approx_epoch, day_date),
                epoch: row.approx_epoch,
                day: day_date,
                balance: BigDecimal::from_str(&row.total_sol_balance.to_string()).unwrap(),
            }
        })
        .collect();

    if new_records.is_empty() {
        info!("No new epochs to insert.");
    } else {
        info!("Inserting {} new epoch records...", new_records.len());
        InactiveStakeJitoSol::bulk_insert(db, new_records).await?;
        info!("Insertion complete.");
    }
    Ok(())
}
