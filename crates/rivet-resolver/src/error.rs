use rivet_core::{Feature, PackageName, Target, Version, VersionReq};
use thiserror::Error;

/// Errors that occur during dependency resolution.
#[derive(Debug, Error)]
pub enum ResolverError {
    #[error("package '{name}' not found in repository{}", .requested_by.as_ref().map(|r| format!(" (required by '{r}')")).unwrap_or_default())]
    PackageNotFound {
        name: PackageName,
        requested_by: Option<PackageName>,
    },

    #[error("no matching version found for '{name}' matching '{req}' (available: {available:?})")]
    NoMatchingVersion {
        name: PackageName,
        req: VersionReq,
        available: Vec<Version>,
    },

    #[error("version conflict for package '{name}': {reason}")]
    VersionConflict { name: PackageName, reason: String },

    #[error("package '{name}' (version {version}) is not supported on target platform '{target}'")]
    UnsupportedPlatform {
        name: PackageName,
        version: Version,
        target: Target,
    },

    #[error("cyclic dependency detected: {}", cycle.iter().map(|p| p.as_str()).collect::<Vec<_>>().join(" -> "))]
    CyclicDependency { cycle: Vec<PackageName> },

    #[error("package '{package}' requires missing feature '{feature}'")]
    MissingFeature {
        package: PackageName,
        feature: Feature,
    },
}

pub type Result<T, E = ResolverError> = std::result::Result<T, E>;
