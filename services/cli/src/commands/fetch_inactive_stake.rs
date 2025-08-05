use crate::utils::wait_for_query_execution;
use crate::{
    error::CliError,
    utils::{execute_dune_query, fetch_dune_query},
};
use sqlx::{Pool, Postgres, types::BigDecimal};
use stakenet_simulator_db::inactive_stake_jito_sol::InactiveStakeJitoSol;
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

    let records: Vec<InactiveStakeJitoSol> = results
        .into_iter()
        .map(|row| {
            let day_date = row.day.chars().take(10).collect::<String>();
            InactiveStakeJitoSol::new(
                row.approx_epoch,
                day_date,
                BigDecimal::from_str(&row.total_sol_balance.to_string()).unwrap(),
            )
        })
        .collect();

    if records.is_empty() {
        info!("No records to process.");
    } else {
        info!("Processing records...");
        InactiveStakeJitoSol::bulk_insert(db, records).await?;
        info!("Processing complete. New records inserted, duplicates ignored.");
    }

    Ok(())
}
