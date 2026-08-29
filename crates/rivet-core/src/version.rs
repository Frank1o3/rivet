use std::fmt;
use std::ops::Deref;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{CoreError, Result};

/// A semantic version following SemVer 2.0.0.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version(pub semver::Version);

impl Version {
    /// Creates a new `Version` with major, minor, and patch numbers.
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self(semver::Version::new(major, minor, patch))
    }

    /// Parses a semantic version from a string.
    pub fn parse(s: &str) -> Result<Self> {
        semver::Version::parse(s)
            .map(Self)
            .map_err(|e| CoreError::InvalidVersion(s.to_string(), e))
    }

    /// Returns the major version number.
    #[inline]
    pub fn major(&self) -> u64 {
        self.0.major
    }

    /// Returns the minor version number.
    #[inline]
    pub fn minor(&self) -> u64 {
        self.0.minor
    }

    /// Returns the patch version number.
    #[inline]
    pub fn patch(&self) -> u64 {
        self.0.patch
    }

    /// Returns true if this version is a pre-release (e.g. `1.0.0-alpha`).
    #[inline]
    pub fn is_prerelease(&self) -> bool {
        !self.0.pre.is_empty()
    }
}

impl Deref for Version {
    type Target = semver::Version;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Debug for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl FromStr for Version {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl From<semver::Version> for Version {
    fn from(v: semver::Version) -> Self {
        Self(v)
    }
}

impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        semver::Version::deserialize(deserializer).map(Self)
    }
}

/// A version requirement or constraint (e.g. `>= 1.2.0, < 2.0.0`, `^1.4`, `*`).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct VersionReq(pub semver::VersionReq);

impl VersionReq {
    /// Requirement that matches any version (`*`).
    pub const STAR: Self = Self(semver::VersionReq::STAR);

    /// Parses a version requirement from a string.
    pub fn parse(s: &str) -> Result<Self> {
        semver::VersionReq::parse(s)
            .map(Self)
            .map_err(|e| CoreError::InvalidVersionReq(s.to_string(), e))
    }

    /// Evaluates whether the given `Version` satisfies this requirement.
    #[inline]
    pub fn matches(&self, version: &Version) -> bool {
        self.0.matches(&version.0)
    }
}

impl Deref for VersionReq {
    type Target = semver::VersionReq;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for VersionReq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Debug for VersionReq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl FromStr for VersionReq {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl From<semver::VersionReq> for VersionReq {
    fn from(req: semver::VersionReq) -> Self {
        Self(req)
    }
}

impl Serialize for VersionReq {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for VersionReq {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::parse(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing_and_properties() {
        let v = Version::parse("1.2.3-beta.1").unwrap();
        assert_eq!(v.major(), 1);
        assert_eq!(v.minor(), 2);
        assert_eq!(v.patch(), 3);
        assert!(v.is_prerelease());

        let v_stable = Version::new(2, 0, 1);
        assert_eq!(v_stable.to_string(), "2.0.1");
        assert!(!v_stable.is_prerelease());
    }

    #[test]
    fn test_version_req_matching() {
        let req = VersionReq::parse(">= 1.2.0, < 2.0.0").unwrap();
        let v1 = Version::parse("1.2.0").unwrap();
        let v2 = Version::parse("1.9.5").unwrap();
        let v3 = Version::parse("2.0.0").unwrap();
        let v4 = Version::parse("1.1.9").unwrap();

        assert!(req.matches(&v1));
        assert!(req.matches(&v2));
        assert!(!req.matches(&v3));
        assert!(!req.matches(&v4));
    }

    #[test]
    fn test_version_serde() {
        let v = Version::parse("1.0.0").unwrap();
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "\"1.0.0\"");
        let deserialized: Version = serde_json::from_str(&json).unwrap();
        assert_eq!(v, deserialized);

        let req = VersionReq::parse("^1.2").unwrap();
        let req_json = serde_json::to_string(&req).unwrap();
        assert_eq!(req_json, "\"^1.2\"");
        let req_deser: VersionReq = serde_json::from_str(&req_json).unwrap();
        assert_eq!(req, req_deser);
    }
}
