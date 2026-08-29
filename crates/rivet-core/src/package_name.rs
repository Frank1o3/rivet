use std::fmt;
use std::ops::Deref;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{CoreError, Result};

/// A validated package name.
///
/// Package names must:
/// - Be between 1 and 128 characters in length.
/// - Start with an ASCII lowercase letter or digit `[a-z0-9]`.
/// - Contain only lowercase ASCII letters, digits, hyphens (`-`), underscores (`_`), dots (`.`), or plus (`+`).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageName(String);

impl PackageName {
    /// Validates and constructs a new `PackageName`.
    pub fn new(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        Self::validate(&name)?;
        Ok(Self(name))
    }

    /// Validates the format of a package name string.
    pub fn validate(name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(CoreError::InvalidPackageName(
                name.to_string(),
                "package name cannot be empty",
            ));
        }

        if name.len() > 128 {
            return Err(CoreError::InvalidPackageName(
                name.to_string(),
                "package name cannot exceed 128 characters",
            ));
        }

        let first_char = name.chars().next().unwrap();
        if !first_char.is_ascii_lowercase() && !first_char.is_ascii_digit() {
            return Err(CoreError::InvalidPackageName(
                name.to_string(),
                "package name must start with a lowercase alphanumeric character",
            ));
        }

        for ch in name.chars() {
            let is_valid = ch.is_ascii_lowercase()
                || ch.is_ascii_digit()
                || ch == '-'
                || ch == '_'
                || ch == '.'
                || ch == '+';

            if !is_valid {
                return Err(CoreError::InvalidPackageName(
                    name.to_string(),
                    "package name contains invalid characters (allowed: [a-z0-9._+-])",
                ));
            }
        }

        Ok(())
    }

    /// Returns the string slice representation of the package name.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for PackageName {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for PackageName {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PackageName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for PackageName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PackageName({})", self.0)
    }
}

impl FromStr for PackageName {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl TryFrom<&str> for PackageName {
    type Error = CoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for PackageName {
    type Error = CoreError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl Serialize for PackageName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for PackageName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        PackageName::new(s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_package_names() {
        let valid = [
            "neovim",
            "gcc-libs",
            "lib32-zlib",
            "python3.11",
            "gtk+",
            "a_b-c.d+e",
        ];
        for name in valid {
            assert!(
                PackageName::new(name).is_ok(),
                "expected '{}' to be valid",
                name
            );
        }
    }

    #[test]
    fn test_invalid_package_names() {
        let invalid = [
            "",               // empty
            "-gcc",           // starts with hyphen
            "_lib",           // starts with underscore
            ".hidden",        // starts with dot
            "NeoVim",         // uppercase
            "foo bar",        // whitespace
            "foo/bar",        // slash
            "foo@bar",        // at symbol
            &"a".repeat(129), // exceeds length
        ];
        for name in invalid {
            assert!(
                PackageName::new(name).is_err(),
                "expected '{}' to be invalid",
                name
            );
        }
    }

    #[test]
    fn test_serde_roundtrip() {
        let pkg = PackageName::new("ripgrep").unwrap();
        let json = serde_json::to_string(&pkg).unwrap();
        assert_eq!(json, "\"ripgrep\"");
        let deserialized: PackageName = serde_json::from_str(&json).unwrap();
        assert_eq!(pkg, deserialized);
    }
}
