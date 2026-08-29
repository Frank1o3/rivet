use rivet_core::{FeatureSet, PackageName};
use rivet_package::PackageManifest;
use serde::{Deserialize, Serialize};

/// A single resolved package ready for build/installation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedPackage {
    pub manifest: PackageManifest,
    pub enabled_features: FeatureSet,
    pub build_dependencies: Vec<PackageName>,
    pub runtime_dependencies: Vec<PackageName>,
    /// True if this package's requirement was satisfied by something
    /// already present on the system rather than by Rivet building or
    /// installing it. Installers should skip it entirely.
    #[serde(default)]
    pub is_system_provided: bool,
}

/// An ordered execution plan for building and installing resolved packages.
///
/// The list of packages is ordered topologically such that all dependencies
/// appear before the packages that depend on them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolutionPlan {
    pub packages: Vec<ResolvedPackage>,
}

impl ResolutionPlan {
    pub fn new(packages: Vec<ResolvedPackage>) -> Self {
        Self { packages }
    }

    pub fn len(&self) -> usize {
        self.packages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ResolvedPackage> {
        self.packages.iter()
    }
}

impl IntoIterator for ResolutionPlan {
    type Item = ResolvedPackage;
    type IntoIter = std::vec::IntoIter<ResolvedPackage>;

    fn into_iter(self) -> Self::IntoIter {
        self.packages.into_iter()
    }
}

impl<'a> IntoIterator for &'a ResolutionPlan {
    type Item = &'a ResolvedPackage;
    type IntoIter = std::slice::Iter<'a, ResolvedPackage>;

    fn into_iter(self) -> Self::IntoIter {
        self.packages.iter()
    }
}
