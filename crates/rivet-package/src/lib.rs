pub mod context;
pub mod dependency;
pub mod error;
pub mod fetch;
pub mod installer;
pub mod loader;
pub mod manifest;
pub mod source;

pub use context::BuildContext;
pub use dependency::{Dependency, DependencyKind};
pub use error::{PackageError, Result};
pub use installer::{install, uninstall};
pub use loader::PackageLoader;
pub use manifest::PackageManifest;
pub use source::{GitRef, Source};
