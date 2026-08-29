use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256 as Sha256Hasher, Sha512 as Sha512Hasher};

use crate::error::{CoreError, Result};

/// A cryptographic checksum for source archive and binary package verification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Checksum {
    Sha256(String),
    Sha512(String),
}

impl Checksum {
    /// Creates a SHA-256 checksum after verifying that the input is a valid 64-character hex string.
    pub fn sha256(hex_str: impl AsRef<str>) -> Result<Self> {
        let hex_str = hex_str.as_ref().trim().to_ascii_lowercase();
        if hex_str.len() != 64 || hex::decode(&hex_str).is_err() {
            return Err(CoreError::InvalidChecksum(
                hex_str,
                "SHA-256 checksum must be a 64-character hexadecimal string",
            ));
        }
        Ok(Checksum::Sha256(hex_str))
    }

    /// Creates a SHA-512 checksum after verifying that the input is a valid 128-character hex string.
    pub fn sha512(hex_str: impl AsRef<str>) -> Result<Self> {
        let hex_str = hex_str.as_ref().trim().to_ascii_lowercase();
        if hex_str.len() != 128 || hex::decode(&hex_str).is_err() {
            return Err(CoreError::InvalidChecksum(
                hex_str,
                "SHA-512 checksum must be a 128-character hexadecimal string",
            ));
        }
        Ok(Checksum::Sha512(hex_str))
    }

    /// Computes the SHA-256 checksum of a byte buffer.
    pub fn compute_sha256(bytes: &[u8]) -> Self {
        let mut hasher = Sha256Hasher::new();
        hasher.update(bytes);
        let result = hasher.finalize();
        Checksum::Sha256(hex::encode(result))
    }

    /// Computes the SHA-512 checksum of a byte buffer.
    pub fn compute_sha512(bytes: &[u8]) -> Self {
        let mut hasher = Sha512Hasher::new();
        hasher.update(bytes);
        let result = hasher.finalize();
        Checksum::Sha512(hex::encode(result))
    }

    /// Verifies whether the provided byte buffer matches this checksum.
    pub fn verify(&self, bytes: &[u8]) -> Result<()> {
        let actual = match self {
            Checksum::Sha256(_) => Self::compute_sha256(bytes),
            Checksum::Sha512(_) => Self::compute_sha512(bytes),
        };

        if self == &actual {
            Ok(())
        } else {
            Err(CoreError::ChecksumMismatch {
                expected: self.hex_value().to_string(),
                actual: actual.hex_value().to_string(),
            })
        }
    }

    /// Returns the raw hexadecimal string of the checksum.
    pub fn hex_value(&self) -> &str {
        match self {
            Checksum::Sha256(h) | Checksum::Sha512(h) => h.as_str(),
        }
    }
}

impl fmt::Display for Checksum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Checksum::Sha256(h) => write!(f, "sha256:{}", h),
            Checksum::Sha512(h) => write!(f, "sha512:{}", h),
        }
    }
}

impl FromStr for Checksum {
    type Err = CoreError;

    /// Parses a checksum string in format `sha256:<hex>` or `sha512:<hex>` or auto-detects 64/128 hex chars.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        if let Some(rest) = trimmed.strip_prefix("sha256:") {
            Self::sha256(rest)
        } else if let Some(rest) = trimmed.strip_prefix("sha512:") {
            Self::sha512(rest)
        } else if trimmed.len() == 64 {
            Self::sha256(trimmed)
        } else if trimmed.len() == 128 {
            Self::sha512(trimmed)
        } else {
            Err(CoreError::InvalidChecksum(
                s.to_string(),
                "expected format 'sha256:<hex>', 'sha512:<hex>', or raw 64/128-char hex string",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_and_verify_sha256() {
        let data = b"hello rivet package manager";
        let checksum = Checksum::compute_sha256(data);
        assert!(checksum.verify(data).is_ok());
        assert!(checksum.verify(b"tampered data").is_err());
    }

    #[test]
    fn test_checksum_parsing() {
        let raw = "a591a6d40bf420404a011733cfb7b190d62c65bf0bcda32b57b277d9ad9f146e";
        let parsed = Checksum::from_str(raw).unwrap();
        assert_eq!(parsed, Checksum::Sha256(raw.to_string()));

        let prefixed = format!("sha256:{}", raw);
        assert_eq!(Checksum::from_str(&prefixed).unwrap(), parsed);
    }
}
