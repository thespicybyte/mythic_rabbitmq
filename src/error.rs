use thiserror::Error;

#[derive(Debug, Error)]
pub enum MythicError {
    #[error("RabbitMQ connection error: {0}")]
    Connection(#[from] lapin::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("C2 sync failed: {0}")]
    SyncFailed(String),

    #[error("RPC handler error: {0}")]
    RpcError(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, MythicError>;
