use rivet_core::Checksum;
use serde::{Deserialize, Serialize};

/// Git reference type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum GitRef {
    Tag(String),
    Branch(String),
    Commit(String),
}

/// Upstream source definition for retrieving package code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Source {
    /// Remote downloadable archive (tar.gz, tar.xz, zip, etc.) with a cryptographic checksum.
    Archive { url: String, checksum: Checksum },
    /// Git repository.
    Git {
        url: String,
        #[serde(default)]
        reference: Option<GitRef>,
        #[serde(default)]
        checksum: Option<Checksum>,
    },
    /// Local filesystem path (useful for testing or local development).
    Local { path: String },
    /// Virtual or meta package with no upstream source files.
    Virtual,
}
