use rivet_core::error::CoreError;
use thiserror::Error;

/// Errors that can occur while parsing, validating, or executing package recipes.
#[derive(Debug, Error)]
pub enum PackageError {
    #[error("core error: {0}")]
    Core(#[from] CoreError),

    #[error("Lua runtime error: {0}")]
    Lua(String),

    #[error("package definition is missing required field '{0}'")]
    MissingField(&'static str),

    #[error("invalid field '{field}': {reason}")]
    InvalidField { field: &'static str, reason: String },

    #[error("no package definition found in recipe (did you call `package({{ ... }})`?)")]
    NoPackageDefined,

    #[error("multiple `package({{ ... }})` calls found in a single recipe file")]
    MultiplePackagesDefined,

    #[error("failed to read recipe file: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to fetch package source: {0}")]
    SourceFetch(String),
}

impl From<mlua::Error> for PackageError {
    fn from(err: mlua::Error) -> Self {
        PackageError::Lua(err.to_string())
    }
}

pub type Result<T, E = PackageError> = std::result::Result<T, E>;
