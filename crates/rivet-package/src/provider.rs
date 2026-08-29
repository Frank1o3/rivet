use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCheck {
    /// Executable to look for on PATH, e.g. "rustc".
    pub command: String,
    /// Flag used to print its version, e.g. "--version".
    #[serde(default = "default_version_flag")]
    pub version_flag: String,
}

fn default_version_flag() -> String {
    "--version".to_string()
}

impl ProviderCheck {
    pub fn detect(&self) -> Option<rivet_core::Version> {
        if let Some(version) = self.detect_via_command(&self.command) {
            return Some(version);
        }

        if let Some(user_bin) = self.sudo_user_cargo_bin() {
            return self.detect_via_command(user_bin);
        }

        None
    }

    /// Runs `<command_path> <version_flag>` and extracts a semver from stdout.
    fn detect_via_command(
        &self,
        command_path: impl AsRef<std::ffi::OsStr>,
    ) -> Option<rivet_core::Version> {
        let output = std::process::Command::new(command_path)
            .arg(&self.version_flag)
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let text = String::from_utf8_lossy(&output.stdout);
        extract_version(&text)
    }

    /// done here to keep this patch minimal.
    fn sudo_user_cargo_bin(&self) -> Option<PathBuf> {
        let invoking_user = std::env::var("SUDO_USER").ok()?;
        if invoking_user == "root" {
            return None;
        }

        let candidate = PathBuf::from("/home")
            .join(invoking_user)
            .join(".cargo/bin")
            .join(&self.command);

        candidate.is_file().then_some(candidate)
    }
}

fn extract_version(text: &str) -> Option<rivet_core::Version> {
    text.split(|c: char| c.is_whitespace() || matches!(c, ',' | '(' | ')'))
        .find_map(|token| {
            let cleaned = token.trim_start_matches('v');
            rivet_core::Version::parse(cleaned).ok()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_version_from_typical_output() {
        assert_eq!(
            extract_version("rustc 1.75.0 (82e1608df 2023-12-21)").unwrap(),
            rivet_core::Version::new(1, 75, 0)
        );
        assert_eq!(
            extract_version("cargo 1.75.0").unwrap(),
            rivet_core::Version::new(1, 75, 0)
        );
        assert!(extract_version("no version here").is_none());
    }
}
