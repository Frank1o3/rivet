use serde::{Deserialize, Serialize};

/// Describes how to detect that a package's functionality is already
/// available on the host system, so Rivet can skip building/installing
/// it and just use what's there — e.g. a `rust` package that's happy to
/// defer to a `rustc` already put on PATH by rustup, homebrew, the
/// system package manager, or anything else Rivet didn't install.
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
    /// Runs the check against the real system. Returns the detected
    /// version if the command exists, ran successfully, and its output
    /// contained something that parses as a semantic version.
    pub fn detect(&self) -> Option<rivet_core::Version> {
        let output = std::process::Command::new(&self.command)
            .arg(&self.version_flag)
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let text = String::from_utf8_lossy(&output.stdout);
        extract_version(&text)
    }
}

/// Scans whitespace/punctuation-delimited tokens in `text` for the first
/// one that parses as a semver version, e.g. pulls "1.75.0" out of
/// "rustc 1.75.0 (82e1608df 2023-12-21)".
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
