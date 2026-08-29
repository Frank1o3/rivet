use std::fmt;

use rivet_core::{Feature, PackageName, VersionReq};
use serde::{Deserialize, Serialize};

use crate::error::{PackageError, Result};

/// The kind/phase of dependency requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyKind {
    /// Required only during compilation/building.
    Build,
    /// Required at runtime on the target system.
    Runtime,
}

/// A dependency requirement on another package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dependency {
    pub name: PackageName,
    pub req: VersionReq,
    pub kind: DependencyKind,
    /// Optional: only required if this feature is activated on the package.
    pub feature: Option<Feature>,
}

impl Dependency {
    pub fn new(name: PackageName, req: VersionReq, kind: DependencyKind) -> Self {
        Self {
            name,
            req,
            kind,
            feature: None,
        }
    }

    pub fn with_feature(mut self, feature: Feature) -> Self {
        self.feature = Some(feature);
        self
    }

    /// Parses a dependency shorthand string like `"foo >= 1.2.0"`, `"bar ^2.0"`, or `"baz"`.
    pub fn parse_shorthand(s: &str, kind: DependencyKind) -> Result<Self> {
        let s = s.trim();
        if s.is_empty() {
            return Err(PackageError::InvalidField {
                field: "dependency",
                reason: "dependency string cannot be empty".to_string(),
            });
        }

        // Split by first whitespace or comparison operator
        let mut parts = s.split_whitespace();
        let name_str = parts.next().unwrap();
        let req_str = s[name_str.len()..].trim();

        let name = PackageName::new(name_str).map_err(|e| PackageError::InvalidField {
            field: "dependency name",
            reason: e.to_string(),
        })?;

        let req = if req_str.is_empty() {
            VersionReq::STAR
        } else {
            VersionReq::parse(req_str).map_err(|e| PackageError::InvalidField {
                field: "dependency version requirement",
                reason: e.to_string(),
            })?
        };

        Ok(Self {
            name,
            req,
            kind,
            feature: None,
        })
    }
}

impl fmt::Display for Dependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.req == VersionReq::STAR {
            write!(f, "{}", self.name)
        } else {
            write!(f, "{} {}", self.name, self.req)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rivet_core::Version;

    #[test]
    fn test_parse_dependency_shorthand() {
        let dep = Dependency::parse_shorthand("libuv >= 1.40.0", DependencyKind::Runtime).unwrap();
        assert_eq!(dep.name.as_str(), "libuv");
        assert_eq!(dep.kind, DependencyKind::Runtime);
        assert!(dep.req.matches(&Version::parse("1.40.0").unwrap()));
        assert!(dep.req.matches(&Version::parse("1.42.0").unwrap()));
        assert!(!dep.req.matches(&Version::parse("1.39.0").unwrap()));

        let star_dep = Dependency::parse_shorthand("zlib", DependencyKind::Build).unwrap();
        assert_eq!(star_dep.name.as_str(), "zlib");
        assert_eq!(star_dep.req, VersionReq::STAR);
        assert_eq!(star_dep.kind, DependencyKind::Build);
    }
}
