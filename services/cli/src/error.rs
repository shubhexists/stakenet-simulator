use solana_client::client_error::ClientError;
use sqlx::Error as SqlxError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("Solana ClientError: {0}")]
    ClientError(#[from] ClientError),

    #[error("SqlxError: {0}")]
    SqlxError(#[from] SqlxError),

    #[error("AnchorDeserializeError")]
    AnchorDeserializeError,

    #[error("ArithmeticError")]
    ArithmeticError,

    #[error("Dune API Error")]
    DuneApiError,

    #[error("Execution Id not provided")]
    EmptyExecutionId,

    #[error("RPC Url is required for this command")]
    InvalidRPCUrl,

    #[error("Lookback period can't be larger than current epoch")]
    LookBackPeriodTooBig,

    #[error("Record count mismatch: active stake has {active_count} records, inactive stake has {inactive_count} records")]
    RecordCountMismatch { active_count: i64, inactive_count: i64 },
}
