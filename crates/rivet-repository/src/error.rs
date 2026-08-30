use rivet_core::CoreError;
use rivet_package::PackageError;
use thiserror::Error;

/// Errors that occur during repository operations.
#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("core error: {0}")]
    Core(#[from] CoreError),

    #[error("package recipe error: {0}")]
    Package(#[from] PackageError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("invalid repository configuration: {0}")]
    InvalidConfig(String),

    #[error("failed to synchronize repository '{name}': {reason}")]
    SyncFailed { name: String, reason: String },

    #[error("Lua runtime error: {0}")]
    Lua(String),

    #[error("invalid repository definition: {0}")]
    Definition(String),
}

pub type Result<T, E = RepositoryError> = std::result::Result<T, E>;

impl From<mlua::Error> for RepositoryError {
    fn from(err: mlua::Error) -> Self {
        RepositoryError::Lua(err.to_string())
    }
}
