use miden_client::transactions::TransactionRequestError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Cannot convert Oracle ID: {0}")]
    InvalidOracleId(String),

    #[error("Cannot convert Publisher ID: {0}")]
    InvalidPublisherId(String),

    #[error("Oracle library setup failed: {0}")]
    OracleLibrarySetupFailed(String),

    #[error("Script compilation failed: {0}")]
    ScriptCompilationFailed(String),

    #[error("Failed to build transaction request: {0}")]
    FailedToBuildTxRequest(TransactionRequestError),
}
