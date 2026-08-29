use std::collections::BTreeSet;
use std::fmt;
use std::ops::Deref;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{CoreError, Result};

/// A package feature flag (e.g. `wayland`, `x11`, `pipewire`, `systemd`, `lto`).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Feature(String);

impl Feature {
    /// Creates a validated feature identifier.
    pub fn new(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        Self::validate(&name)?;
        Ok(Self(name))
    }

    /// Validates feature name format.
    pub fn validate(name: &str) -> Result<()> {
        if name.is_empty() {
            return Err(CoreError::InvalidFeature(
                name.to_string(),
                "feature name cannot be empty",
            ));
        }

        if name.len() > 64 {
            return Err(CoreError::InvalidFeature(
                name.to_string(),
                "feature name cannot exceed 64 characters",
            ));
        }

        let first = name.chars().next().unwrap();
        if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
            return Err(CoreError::InvalidFeature(
                name.to_string(),
                "feature name must start with a lowercase alphanumeric character",
            ));
        }

        for ch in name.chars() {
            if !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != '-' && ch != '_' {
                return Err(CoreError::InvalidFeature(
                    name.to_string(),
                    "feature name contains invalid characters (allowed: [a-z0-9-_])",
                ));
            }
        }

        Ok(())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for Feature {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for Feature {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Feature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for Feature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Feature({})", self.0)
    }
}

impl FromStr for Feature {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl Serialize for Feature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Feature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Feature::new(s).map_err(serde::de::Error::custom)
    }
}

/// A set of enabled or declared features.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureSet(BTreeSet<Feature>);

impl FeatureSet {
    /// Creates an empty feature set.
    pub fn new() -> Self {
        Self(BTreeSet::new())
    }

    /// Inserts a feature into the set.
    pub fn insert(&mut self, feature: Feature) -> bool {
        self.0.insert(feature)
    }

    /// Checks if a feature is present in the set.
    pub fn contains(&self, feature: &Feature) -> bool {
        self.0.contains(feature)
    }

    /// Checks if a feature string is present in the set.
    pub fn contains_str(&self, feature_name: &str) -> bool {
        self.0.iter().any(|f| f.as_str() == feature_name)
    }

    /// Returns true if `self` is a subset of `other`.
    pub fn is_subset(&self, other: &FeatureSet) -> bool {
        self.0.is_subset(&other.0)
    }

    /// Returns the number of features in the set.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if the set contains no features.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns an iterator over the features.
    pub fn iter(&self) -> impl Iterator<Item = &Feature> {
        self.0.iter()
    }
}

impl IntoIterator for FeatureSet {
    type Item = Feature;
    type IntoIter = std::collections::btree_set::IntoIter<Feature>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a FeatureSet {
    type Item = &'a Feature;
    type IntoIter = std::collections::btree_set::Iter<'a, Feature>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl FromIterator<Feature> for FeatureSet {
    fn from_iter<T: IntoIterator<Item = Feature>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_features() {
        let valid = ["wayland", "x11", "pipewire", "lto_support", "audio-backend"];
        for f in valid {
            assert!(Feature::new(f).is_ok());
        }
    }

    #[test]
    fn test_invalid_features() {
        let invalid = ["", "-wayland", "Wayland", "foo bar", "x11/wayland"];
        for f in invalid {
            assert!(Feature::new(f).is_err());
        }
    }

    #[test]
    fn test_feature_set() {
        let mut set = FeatureSet::new();
        set.insert(Feature::new("wayland").unwrap());
        set.insert(Feature::new("pipewire").unwrap());

        assert!(set.contains_str("wayland"));
        assert!(set.contains_str("pipewire"));
        assert!(!set.contains_str("x11"));
        assert_eq!(set.len(), 2);
    }
}
