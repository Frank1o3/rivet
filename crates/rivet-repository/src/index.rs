use std::collections::HashMap;
use std::fs;
use std::path::Path;

use rivet_core::PackageName;
use rivet_package::PackageManifest;
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// A serialized or in-memory index of all package manifests in a repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryIndex {
    pub name: String,
    pub schema_version: u32,
    pub packages: HashMap<PackageName, Vec<PackageManifest>>,
}

impl RepositoryIndex {
    pub const CURRENT_SCHEMA_VERSION: u32 = 1;

    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            schema_version: Self::CURRENT_SCHEMA_VERSION,
            packages: HashMap::new(),
        }
    }

    /// Adds a package manifest to the index.
    pub fn add(&mut self, manifest: PackageManifest) {
        let entry = self.packages.entry(manifest.name.clone()).or_default();
        // Replace existing same version if present, otherwise append
        if let Some(pos) = entry.iter().position(|m| m.version == manifest.version) {
            entry[pos] = manifest;
        } else {
            entry.push(manifest);
        }
    }

    /// Gets all candidate versions of a package by name.
    pub fn get(&self, name: &PackageName) -> Option<&Vec<PackageManifest>> {
        self.packages.get(name)
    }

    /// Searches for packages whose name or description contains the search query (case-insensitive).
    pub fn search(&self, query: &str) -> Vec<&PackageManifest> {
        let query = query.to_lowercase();
        let mut results = Vec::new();

        for candidates in self.packages.values() {
            for manifest in candidates {
                let matches_name = manifest.name.as_str().to_lowercase().contains(&query);
                let matches_desc = manifest
                    .description
                    .as_ref()
                    .map(|d| d.to_lowercase().contains(&query))
                    .unwrap_or(false);

                if matches_name || matches_desc {
                    results.push(manifest);
                }
            }
        }

        results
    }

    /// Total unique package names in index.
    pub fn len(&self) -> usize {
        self.packages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }

    /// Total package version manifests across all packages.
    pub fn total_versions(&self) -> usize {
        self.packages.values().map(|v| v.len()).sum()
    }

    /// Saves the index to a JSON file.
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Loads the index from a JSON file.
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let index: Self = serde_json::from_str(&content)?;
        Ok(index)
    }
}
