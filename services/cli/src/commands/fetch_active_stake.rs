use crate::error::CliError;
use crate::utils::wait_for_query_execution;
use crate::utils::{execute_dune_query, fetch_dune_query};
use sqlx::types::BigDecimal;
use sqlx::{Pool, Postgres};
use stakenet_simulator_db::active_stake_jito_sol::ActiveStakeJitoSol;
use std::str::FromStr;
use tracing::info;

pub async fn fetch_active_stake(db: &Pool<Postgres>) -> Result<(), CliError> {
    let execute_client = execute_dune_query(5571504)
        .await
        .map_err(|_| CliError::DuneApiError)?;

    wait_for_query_execution(&execute_client.execution_id).await?;

    let results = fetch_dune_query(execute_client.execution_id)
        .await
        .map_err(|_| CliError::DuneApiError)?;

    let records: Vec<ActiveStakeJitoSol> = results
        .into_iter()
        .map(|row| {
            let day_date = row.day.chars().take(10).collect::<String>();
            ActiveStakeJitoSol {
                id: format!("{}-{}", row.approx_epoch, day_date),
                epoch: row.approx_epoch,
                day: day_date,
                balance: BigDecimal::from_str(&row.total_sol_balance.to_string()).unwrap(),
            }
        })
        .collect();

    if records.is_empty() {
        info!("No records to process.");
    } else {
        info!("Processing records...");
        ActiveStakeJitoSol::bulk_insert(db, records).await?;
        info!("Processing complete. New records inserted, duplicates ignored.");
    }

    Ok(())
}
