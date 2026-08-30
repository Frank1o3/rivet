// rivet-core library root

pub mod checksum;
pub mod database;
pub mod error;
pub mod feature;
pub mod package_name;
pub mod paths;
pub mod scope;
pub mod target;
pub mod version;

// Re-export common domain types for ergonomic usage
pub use checksum::Checksum;
pub use database::{InstalledDatabase, InstalledRecord, RecordedDependency};
pub use error::{CoreError, Result};
pub use feature::{Feature, FeatureSet};
pub use package_name::PackageName;
pub use paths::{
    absolute_path, default_data_dir, default_local_packages_dir, default_packages_dir,
    default_path, default_prefix, default_repo_mirror_cache, default_repositories_dir,
    default_source_cache,
};
pub use scope::InstallScope;
pub use target::{Target, TargetArch, TargetOs};
pub use version::{Version, VersionReq};
