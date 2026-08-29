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
pub use database::{InstalledDatabase, InstalledRecord};
pub use error::{CoreError, Result};
pub use feature::{Feature, FeatureSet};
pub use package_name::PackageName;
pub use paths::{absolute_path, default_path, default_prefix, default_source_cache};
pub use target::{Target, TargetArch, TargetOs};
pub use version::{Version, VersionReq};
pub use scope::InstallScope;
