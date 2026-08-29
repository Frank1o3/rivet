use rivet_core::{FeatureSet, PackageName};
use rivet_package::PackageManifest;
use serde::{Deserialize, Serialize};

/// A single resolved package ready for build/installation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedPackage {
    /// Full package manifest.
    pub manifest: PackageManifest,

    /// Set of enabled features.
    pub enabled_features: FeatureSet,

    /// Names of build dependencies that must be available during compilation.
    pub build_dependencies: Vec<PackageName>,

    /// Names of runtime dependencies that must be installed on the system.
    pub runtime_dependencies: Vec<PackageName>,
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
