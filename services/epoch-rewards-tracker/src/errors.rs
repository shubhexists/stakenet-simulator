use crate::{config::ConfigError, rpc_utils::RpcUtilsError};
use solana_client::client_error::ClientError;
use solana_sdk::pubkey::{ParsePubkeyError, Pubkey};
use sqlx::Error as SqlxError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EpochRewardsTrackerError {
    #[error("ConfigError: {0}")]
    ConfigError(#[from] ConfigError),

    #[error("Solana ClientError: {0}")]
    ClientError(#[from] ClientError),

    #[error("ValidatorHistoryNotFound: {0}")]
    ValidatorHistoryNotFound(Pubkey),

    #[error("ClusterHistoryNotFound: {0}")]
    ClusterHistoryNotFound(Pubkey),

    #[error("SqlxError: {0}")]
    SqlxError(#[from] SqlxError),

    #[error("ParsePubkeyError: {0}")]
    ParsePubkeyError(#[from] ParsePubkeyError),

    #[error("RpcUtilsError: {0}")]
    RpcUtilsError(#[from] RpcUtilsError),

    #[error("MissingLeaderSchedule for epoch: {0}")]
    MissingLeaderSchedule(u64),

    #[error("Invalid Pubkey provided")]
    InvalidPubkeyError,

    #[error("Database connection error")]
    DatabaseConnectionError,

    #[error("Execution Id not provided")]
    EmptyExecutionId,

    #[error("Dune API Error")]
    DuneApiError,
}
