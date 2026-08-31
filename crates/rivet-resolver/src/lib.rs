// rivet-resolver library root

pub mod error;
pub mod plan;
pub mod provider;
pub mod solver;

// Re-export common types
pub use error::{ResolverError, Result};
pub use plan::{ExecutionStage, ResolutionPlan, ResolvedPackage};
pub use provider::{InMemoryPackageProvider, PackageProvider};
pub use solver::DependencySolver;
