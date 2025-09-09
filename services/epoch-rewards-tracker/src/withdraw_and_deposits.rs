use crate::{
    dune::{
        WITHDRAW_DEPOSIT_TRANSACTIONS_QUERY, WithdrawAndDepositsRow, execute_dune_query,
        fetch_dune_query, wait_for_query_execution,
    },
    errors::EpochRewardsTrackerError,
};
use sqlx::{Pool, Postgres};

pub async fn withdraw_and_deposits(db: &Pool<Postgres>) -> Result<(), EpochRewardsTrackerError> {
    let execute_client = execute_dune_query(WITHDRAW_DEPOSIT_TRANSACTIONS_QUERY)
        .await
        .map_err(|_| EpochRewardsTrackerError::DuneApiError)?;

    wait_for_query_execution(&execute_client.execution_id).await?;
    let results: Vec<WithdrawAndDepositsRow> = fetch_dune_query(execute_client.execution_id)
        .await
        .map_err(|_| EpochRewardsTrackerError::DuneApiError)?;

    for row in results {
        println!("{:?}", row)
    }

    Ok(())
}
