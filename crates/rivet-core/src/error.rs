use std::io;
use thiserror::Error;

/// Core error types for the Rivet package manager.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("invalid package name '{0}': {1}")]
    InvalidPackageName(String, &'static str),

    #[error("invalid version string '{0}': {1}")]
    InvalidVersion(String, semver::Error),

    #[error("invalid version requirement '{0}': {1}")]
    InvalidVersionReq(String, semver::Error),

    #[error("invalid checksum '{0}': {1}")]
    InvalidChecksum(String, &'static str),

    #[error("checksum mismatch: expected {expected}, calculated {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("invalid feature name '{0}': {1}")]
    InvalidFeature(String, &'static str),

    #[error("invalid target architecture '{0}'")]
    InvalidTargetArch(String),

    #[error("invalid target OS '{0}'")]
    InvalidTargetOs(String),

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

pub type Result<T, E = CoreError> = std::result::Result<T, E>;
