use serde::{Deserialize, Serialize};

/// A lightweight, repository-agnostic search result.
///
/// Deliberately doesn't carry dependency/build data. For local repos
/// that data is already sitting in memory as a full `PackageManifest`,
/// but forcing a remote repo to fetch a package just to answer `search`
/// would defeat the entire point of publishing a lightweight index.
/// `rivet info` fetches and shows the full manifest once a specific
/// package is actually chosen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageSummary {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    /// Name of the repository this result came from.
    pub repository: String,
}
