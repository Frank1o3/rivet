use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Where a repository's package definitions are retrieved from.
///
/// Unlike `rivet_package::Source`, this has no `type` tag: repositories
/// are assumed to be Git-based. If a non-Git transport is added later,
/// tag it then rather than guessing the shape now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositorySource {
    pub url: String,
    pub branch: String,
    /// Subdirectory containing `index.json` and `packages/`. Empty = repo root.
    #[serde(default)]
    pub path: Option<String>,
}

/// The declarative metadata extracted from a `repository.lua` definition.
///
/// `name` is human-readable prose (e.g. "Rivet"), not a filesystem
/// identifier — the on-disk slug under `~/.rivet/packages/<slug>/` comes
/// from the definition file's own filename stem, tracked separately by
/// whatever loads this from disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryDefinition {
    pub name: String,
    pub description: Option<String>,
    pub license: Option<String>,
    pub source: RepositorySource,
    /// Path to the `.lua` file this was loaded from. Empty for
    /// in-memory/test definitions.
    #[serde(default)]
    pub definition_path: PathBuf,
}
