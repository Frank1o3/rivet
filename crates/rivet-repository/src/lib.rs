// rivet-repository library root

pub mod config;
pub mod error;
pub mod index;
pub mod local;
pub mod multi;

// Re-export common types
pub use config::RepositoryConfig;
pub use error::{RepositoryError, Result};
pub use index::RepositoryIndex;
pub use local::LocalRepository;
pub use multi::{MultiRepositoryManager, RepositoryEntry};
