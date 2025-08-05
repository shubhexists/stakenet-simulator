use crate::error::CliError;
use dotenvy::dotenv;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;
use std::collections::HashSet;
use std::env;
use tokio::time::{Duration, sleep};
use tracing::{info, warn};

#[derive(Debug, Deserialize, PartialEq)]
pub struct Row {
    pub day: String,
    pub total_sol_balance: f64,
    pub approx_epoch: u64,
}

#[derive(Debug, Deserialize)]
pub struct ExecuteResponse {
    pub execution_id: String,
    pub state: String,
}

#[derive(Debug, Deserialize)]
struct ResultData {
    rows: Vec<Row>,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    result: ResultData,
}

#[derive(Debug, Deserialize)]
pub struct ExecutionStatus {
    pub execution_id: String,
    pub query_id: u64,
    pub is_execution_finished: bool,
    pub state: String,
    pub submitted_at: String,

    // "QUERY_STATE_COMPLETED" state
    pub expires_at: Option<String>,
    pub execution_started_at: Option<String>,
    pub execution_ended_at: Option<String>,
    pub result_metadata: Option<ResultMetadata>,

    // "QUERY_STATE_PENDING" state
    pub queue_position: Option<u32>,
    pub max_inflight_interactive_executions: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct ResultMetadata {
    pub column_names: Vec<String>,
    pub column_types: Vec<String>,
    pub row_count: u32,
    pub result_set_bytes: u32,
    pub total_row_count: u32,
    pub total_result_set_bytes: u32,
    pub datapoint_count: u32,
    pub pending_time_millis: Option<u64>,
    pub execution_time_millis: Option<u64>,
}

pub async fn execute_dune_query(
    query_id: u64,
) -> Result<ExecuteResponse, Box<dyn std::error::Error>> {
    dotenv().ok();

    let api_key = env::var("DUNE_API_KEY")?;
    let url = format!("https://api.dune.com/api/v1/query/{}/execute", query_id);

    let client = reqwest::Client::new();

    let mut headers = HeaderMap::new();
    headers.insert("X-Dune-API-Key", HeaderValue::from_str(&api_key)?);
    headers.insert("Content-Type", HeaderValue::from_static("application/json"));

    let response = client
        .post(&url)
        .headers(headers)
        .send()
        .await?
        .error_for_status()?
        .json::<ExecuteResponse>()
        .await?;

    Ok(response)
}

pub async fn fetch_dune_query(
    execution_id: String,
) -> Result<Vec<Row>, Box<dyn std::error::Error>> {
    dotenv().ok();

    let api_key = env::var("DUNE_API_KEY")?;
    let url = format!("https://api.dune.com/api/v1/execution/{execution_id}/results");
    let client = reqwest::Client::new();

    let mut headers = HeaderMap::new();
    headers.insert("X-Dune-API-Key", HeaderValue::from_str(&api_key)?);

    let response = client
        .get(&url)
        .headers(headers)
        .send()
        .await?
        .error_for_status()?
        .json::<ApiResponse>()
        .await?;

    Ok(response.result.rows)
}

pub async fn fetch_dune_execution_status(
    execution_id: &str,
) -> Result<ExecutionStatus, Box<dyn std::error::Error>> {
    dotenv().ok();

    let api_key = env::var("DUNE_API_KEY")?;
    let url = format!(
        "https://api.dune.com/api/v1/execution/{}/status",
        execution_id
    );

    let client = reqwest::Client::new();

    let mut headers = HeaderMap::new();
    headers.insert("X-Dune-API-Key", HeaderValue::from_str(&api_key)?);

    let response = client
        .get(&url)
        .headers(headers)
        .send()
        .await?
        .error_for_status()?
        .json::<ExecutionStatus>()
        .await?;

    Ok(response)
}

pub async fn wait_for_query_execution(execution_id: &str) -> Result<(), CliError> {
    if execution_id.is_empty() {
        return Err(CliError::EmptyExecutionId);
    }
    info!("Submitted Dune query. Execution ID: {}", execution_id);
    let mut seen_states = HashSet::new();
    loop {
        let status = fetch_dune_execution_status(execution_id)
            .await
            .map_err(|_| CliError::DuneApiError)?;
        let state_str = status.state.as_str();
        if !seen_states.contains(state_str) {
            match state_str {
                "QUERY_STATE_COMPLETED" => {
                    info!("Query execution completed!");
                    break;
                }
                "QUERY_STATE_PENDING" => {
                    info!(
                        "Query pending... Queue position: {:?}",
                        status.queue_position
                    );
                }
                "QUERY_STATE_EXECUTING" => {
                    info!("Query executing... hang tight.");
                }
                other => {
                    warn!("Unexpected query state: {}", other);
                }
            }
            seen_states.insert(state_str.to_string());
        }
        if state_str == "QUERY_STATE_COMPLETED" {
            break;
        }
        sleep(Duration::from_secs(10)).await;
    }
    Ok(())
}
