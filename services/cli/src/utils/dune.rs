use dotenvy::dotenv;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize, PartialEq)]
pub struct Row {
    pub day: String,
    pub total_sol_balance: f64,
    pub approx_epoch: u64,
}

#[derive(Debug, Deserialize)]
struct ResultData {
    rows: Vec<Row>,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    result: ResultData,
}

pub async fn fetch_inactive_sol_stake_jito(
    epoch: u64,
) -> Result<Option<Row>, Box<dyn std::error::Error>> {
    dotenv().ok();

    let api_key = env::var("DUNE_API_KEY")?;
    let query_id = 5571499;
    let url = format!("https://api.dune.com/api/v1/query/{query_id}/results?limit=1000");

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

    let maybe_row = response
        .result
        .rows
        .into_iter()
        .find(|row| row.approx_epoch == epoch);

    Ok(maybe_row)
}

pub async fn fetch_active_sol_stake_jito(
    epoch: u64,
) -> Result<Option<Row>, Box<dyn std::error::Error>> {
    dotenv().ok();

    let api_key = env::var("DUNE_API_KEY")?;
    let query_id = 5571504;
    let url = format!("https://api.dune.com/api/v1/query/{query_id}/results?limit=1000");

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

    let maybe_row = response
        .result
        .rows
        .into_iter()
        .find(|row| row.approx_epoch == epoch);

    Ok(maybe_row)
}
