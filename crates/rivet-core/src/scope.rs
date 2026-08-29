use std::path::PathBuf;

use crate::paths::{absolute_path, default_path, default_prefix};

/// Whether an installation targets only the current user, or the whole
/// system (shared by all users, requires elevated privileges).
///
/// This exists because both the install *prefix* and the install
/// *database* need to live somewhere different depending on who the
/// result is for. Mixing user-owned and root-owned installs into one
/// prefix/database would be confusing at best, and a privilege
/// escalation path at worst — a plain `rivet install` should never be
/// able to write outside the invoking user's own directories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallScope {
    /// Installed into (and only visible to) the invoking user. Default;
    /// requires no special privileges.
    User,
    /// Installed system-wide, shared by all users. Requires root (on
    /// Unix) and must be explicitly requested via `--system`.
    System,
}

impl InstallScope {
    /// Default installation prefix for this scope. Still overridable via
    /// `--prefix` at the CLI layer.
    pub fn default_prefix(self) -> anyhow::Result<PathBuf> {
        match self {
            InstallScope::User => default_prefix(),
            InstallScope::System => match std::env::var("RIVET_SYSTEM_PREFIX") {
                Ok(path) => absolute_path(PathBuf::from(path)),
                Err(_) => Ok(PathBuf::from("/usr/local")),
            },
        }
    }

    /// Default installed-package database location for this scope.
    pub fn default_db_path(self) -> anyhow::Result<PathBuf> {
        match self {
            InstallScope::User => default_path(),
            InstallScope::System => match std::env::var("RIVET_SYSTEM_DB") {
                Ok(path) => absolute_path(PathBuf::from(path)),
                Err(_) => Ok(PathBuf::from("/var/lib/rivet/db.json")),
            },
        }
    }

    /// Refuses `System` scope unless the current process has the
    /// privilege to use it. `User` scope is always permitted.
    pub fn check_permitted(self) -> anyhow::Result<()> {
        if self == InstallScope::User {
            return Ok(());
        }

        #[cfg(unix)]
        {
            // SAFETY: geteuid() takes no arguments, touches no memory
            // through pointers, and has no documented failure mode —
            // it's about as safe as an `unsafe fn` gets.
            let euid = unsafe { libc::geteuid() };
            if euid != 0 {
                anyhow::bail!(
                    "--system requires root privileges; re-run with sudo, \
                     or drop --system to install for your user only"
                );
            }
            Ok(())
        }

        #[cfg(not(unix))]
        {
            anyhow::bail!("--system installs are not yet supported on this platform");
        }
    }
}
