use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration entry for a package repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryConfig {
    /// Unique repository identifier (e.g. "core", "extra", "community").
    pub name: String,

    /// Remote URL for Git or HTTP repository sources.
    pub url: Option<String>,

    /// Local filesystem path for repository files and recipes.
    pub path: Option<PathBuf>,

    /// Priority order (higher numbers take precedence over lower numbers).
    #[serde(default)]
    pub priority: i32,

    /// Whether this repository is currently enabled.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

impl RepositoryConfig {
    pub fn local(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            url: None,
            path: Some(path.into()),
            priority: 0,
            enabled: true,
        }
    }

    pub fn remote(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            url: Some(url.into()),
            path: None,
            priority: 0,
            enabled: true,
        }
    }
}
