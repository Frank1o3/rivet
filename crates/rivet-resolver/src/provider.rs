use std::collections::HashMap;

use rivet_core::PackageName;
use rivet_package::PackageManifest;

/// Abstract interface for querying available package definitions from repositories or indexes.
pub trait PackageProvider {
    /// Returns all available candidate manifests for the given package name.
    fn get_candidates(&self, name: &PackageName) -> Vec<PackageManifest>;
}

/// An in-memory package provider, ideal for unit testing and local resolution.
#[derive(Debug, Default, Clone)]
pub struct InMemoryPackageProvider {
    packages: HashMap<PackageName, Vec<PackageManifest>>,
}

impl InMemoryPackageProvider {
    pub fn new() -> Self {
        Self {
            packages: HashMap::new(),
        }
    }

    /// Adds a package manifest to the provider.
    pub fn add(&mut self, manifest: PackageManifest) {
        self.packages
            .entry(manifest.name.clone())
            .or_default()
            .push(manifest);
    }
}

impl PackageProvider for InMemoryPackageProvider {
    fn get_candidates(&self, name: &PackageName) -> Vec<PackageManifest> {
        self.packages.get(name).cloned().unwrap_or_default()
    }
}
