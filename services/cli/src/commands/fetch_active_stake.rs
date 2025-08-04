use crate::error::CliError;
use crate::utils::{execute_dune_query, fetch_dune_execution_status, fetch_dune_query};
use sqlx::types::BigDecimal;
use sqlx::{Pool, Postgres};
use stakenet_simulator_db::active_stake_jito_sol::ActiveStakeJitoSol;
use std::collections::HashSet;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::sleep;

pub async fn fetch_active_stake(db: &Pool<Postgres>) -> Result<(), CliError> {
    let execute_client = execute_dune_query(5571504)
        .await
        .map_err(|_| CliError::DuneApiError)?;

    if execute_client.execution_id.is_empty() {
        return Err(CliError::EmptyExecutionId);
    }

    println!(
        "Submitted Dune query. Execution ID: {}",
        execute_client.execution_id
    );

    let mut seen_states = HashSet::new();
    loop {
        let status = fetch_dune_execution_status(&execute_client.execution_id)
            .await
            .map_err(|_| CliError::DuneApiError)?;

        let state_str = status.state.as_str();

        // TODO: USE AN ENUM
        if !seen_states.contains(state_str) {
            match state_str {
                "QUERY_STATE_COMPLETED" => {
                    println!("Query execution completed!");
                    break;
                }
                "QUERY_STATE_PENDING" => {
                    println!(
                        "Query pending... Queue position: {:?}",
                        status.queue_position
                    );
                }
                "QUERY_STATE_EXECUTING" => {
                    println!("Query executing... hang tight.");
                }
                other => {
                    println!("Unexpected query state: {}", other);
                }
            }
            seen_states.insert(state_str.to_string());
        }

        if state_str == "QUERY_STATE_COMPLETED" {
            break;
        }

        sleep(Duration::from_secs(10)).await;
    }

    let results = fetch_dune_query(execute_client.execution_id)
        .await
        .map_err(|_| CliError::DuneApiError)?;

    let existing_records = ActiveStakeJitoSol::fetch_all_records(db).await?;
    let existing_epochs: HashSet<u64> = existing_records.into_iter().map(|r| r.epoch).collect();

    let new_records: Vec<ActiveStakeJitoSol> = results
        .into_iter()
        .filter(|row| !existing_epochs.contains(&row.approx_epoch))
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

    if new_records.is_empty() {
        println!("No new epochs to insert.");
    } else {
        println!("Inserting {} new epoch records...", new_records.len());
        ActiveStakeJitoSol::bulk_insert(db, new_records)
            .await
            .map_err(|err| CliError::SqlxError(err))?;
        println!("Insertion complete.");
    }
    Ok(())
}
