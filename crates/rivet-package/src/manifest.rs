use std::collections::HashMap;
use std::path::PathBuf;

use rivet_core::{Feature, PackageName, Target, TargetArch, TargetOs, Version};
use serde::{Deserialize, Serialize};

use crate::dependency::Dependency;
use crate::provider::ProviderCheck;
use crate::source::Source;

/// The complete declarative metadata extracted from a package recipe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageManifest {
    pub name: PackageName,
    pub version: Version,
    pub description: Option<String>,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub source: Option<Source>,
    pub dependencies: Vec<Dependency>,
    pub features: HashMap<Feature, Vec<Dependency>>,
    pub default_features: Vec<Feature>,
    pub supported_architectures: Vec<TargetArch>,
    pub supported_os: Vec<TargetOs>,
    #[serde(default)]
    pub recipe_path: PathBuf,
    #[serde(default)]
    pub provider_check: Option<ProviderCheck>,
    #[serde(default)]
    pub cleanup: Vec<PackageName>,
    #[serde(default)]
    pub source_repository: Option<String>,
}

impl PackageManifest {
    /// Checks if this package can be installed/built on the specified target platform.
    pub fn supports_target(&self, target: &Target) -> bool {
        target.matches(&self.supported_architectures, &self.supported_os)
    }

    /// Returns build dependencies.
    pub fn build_dependencies(&self) -> impl Iterator<Item = &Dependency> {
        self.dependencies
            .iter()
            .filter(|d| d.kind == crate::dependency::DependencyKind::Build)
    }

    /// Returns runtime dependencies.
    pub fn runtime_dependencies(&self) -> impl Iterator<Item = &Dependency> {
        self.dependencies
            .iter()
            .filter(|d| d.kind == crate::dependency::DependencyKind::Runtime)
    }
}
