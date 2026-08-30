pub mod config;
pub mod definition;
pub mod error;
pub mod index;
pub mod loader;
pub mod local;
pub mod multi;
pub mod remote;
pub mod summary;

pub use config::RepositoryConfig;
pub use definition::{RepositoryDefinition, RepositorySource};
pub use error::{RepositoryError, Result};
pub use index::RepositoryIndex;
pub use loader::RepositoryLoader;
pub use local::LocalRepository;
pub use multi::{MultiRepositoryManager, RemoteUpdateResult, RepositoryBackend, RepositoryEntry};
pub use remote::{RemoteIndexChecksum, RemoteIndexEntry, RemoteRepository};
pub use summary::PackageSummary;
