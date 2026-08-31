use std::collections::{HashMap, HashSet};

use rivet_core::{FeatureSet, PackageName};
use rivet_package::PackageManifest;
use serde::{Deserialize, Serialize};

/// A single resolved package ready for build/installation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedPackage {
    pub manifest: PackageManifest,
    pub enabled_features: FeatureSet,
    pub build_dependencies: Vec<PackageName>,
    pub runtime_dependencies: Vec<PackageName>,
    /// True if this package's requirement was satisfied by something
    /// already present on the system rather than by Rivet building or
    /// installing it. Installers should skip it entirely.
    #[serde(default)]
    pub is_system_provided: bool,
}

/// A parallel execution stage containing independent packages that can be built concurrently.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionStage {
    pub stage_index: usize,
    pub packages: Vec<ResolvedPackage>,
}

impl ExecutionStage {
    pub fn new(stage_index: usize, packages: Vec<ResolvedPackage>) -> Self {
        Self {
            stage_index,
            packages,
        }
    }

    pub fn len(&self) -> usize {
        self.packages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }
}

/// An ordered execution plan for building and installing resolved packages.
///
/// The list of packages is ordered topologically such that all dependencies
/// appear before the packages that depend on them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolutionPlan {
    pub packages: Vec<ResolvedPackage>,
}

impl ResolutionPlan {
    pub fn new(packages: Vec<ResolvedPackage>) -> Self {
        Self { packages }
    }

    pub fn len(&self) -> usize {
        self.packages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ResolvedPackage> {
        self.packages.iter()
    }

    /// Stratifies the resolution plan into sequential execution stages where packages within
    /// each stage have no dependencies on each other and can be fetched/built in parallel.
    pub fn stages(&self) -> Vec<ExecutionStage> {
        if self.packages.is_empty() {
            return Vec::new();
        }

        let mut package_stage: HashMap<PackageName, usize> = HashMap::new();
        let plan_package_names: HashSet<PackageName> =
            self.packages.iter().map(|p| p.manifest.name.clone()).collect();

        for item in &self.packages {
            let mut max_dep_stage: Option<usize> = None;

            let all_deps = item
                .build_dependencies
                .iter()
                .chain(item.runtime_dependencies.iter());

            for dep_name in all_deps {
                if plan_package_names.contains(dep_name) {
                    if let Some(&dep_stage) = package_stage.get(dep_name) {
                        max_dep_stage = Some(
                            max_dep_stage
                                .map_or(dep_stage, |m| std::cmp::max(m, dep_stage)),
                        );
                    }
                }
            }

            let my_stage = match max_dep_stage {
                Some(s) => s + 1,
                None => 0,
            };

            package_stage.insert(item.manifest.name.clone(), my_stage);
        }

        let total_stages = package_stage.values().copied().max().map_or(0, |m| m + 1);
        let mut stages_vec: Vec<Vec<ResolvedPackage>> = vec![Vec::new(); total_stages];

        for item in &self.packages {
            let stage_idx = package_stage[&item.manifest.name];
            stages_vec[stage_idx].push(item.clone());
        }

        stages_vec
            .into_iter()
            .enumerate()
            .map(|(stage_index, packages)| ExecutionStage {
                stage_index,
                packages,
            })
            .collect()
    }
}

impl IntoIterator for ResolutionPlan {
    type Item = ResolvedPackage;
    type IntoIter = std::vec::IntoIter<ResolvedPackage>;

    fn into_iter(self) -> Self::IntoIter {
        self.packages.into_iter()
    }
}

impl<'a> IntoIterator for &'a ResolutionPlan {
    type Item = &'a ResolvedPackage;
    type IntoIter = std::slice::Iter<'a, ResolvedPackage>;

    fn into_iter(self) -> Self::IntoIter {
        self.packages.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rivet_core::Version;

    fn make_test_resolved_pkg(
        name: &str,
        build_deps: Vec<&str>,
        runtime_deps: Vec<&str>,
    ) -> ResolvedPackage {
        let manifest = PackageManifest {
            name: PackageName::new(name).unwrap(),
            version: Version::parse("1.0.0").unwrap(),
            description: None,
            license: None,
            homepage: None,
            publisher: None,
            source: None,
            dependencies: vec![],
            features: HashMap::new(),
            default_features: vec![],
            supported_architectures: vec![],
            supported_os: vec![],
            recipe_path: std::path::PathBuf::new(),
            provider_check: None,
            cleanup: vec![],
            source_repository: None,
        };

        ResolvedPackage {
            manifest,
            enabled_features: FeatureSet::new(),
            build_dependencies: build_deps
                .into_iter()
                .map(|s| PackageName::new(s).unwrap())
                .collect(),
            runtime_dependencies: runtime_deps
                .into_iter()
                .map(|s| PackageName::new(s).unwrap())
                .collect(),
            is_system_provided: false,
        }
    }

    #[test]
    fn test_resolution_plan_parallel_stages() {
        // DAG structure:
        // Stage 0: liba, libb
        // Stage 1: libc (depends on liba, libb), libd (depends on liba)
        // Stage 2: app (depends on libc, libd)

        let liba = make_test_resolved_pkg("liba", vec![], vec![]);
        let libb = make_test_resolved_pkg("libb", vec![], vec![]);
        let libc = make_test_resolved_pkg("libc", vec!["liba"], vec!["libb"]);
        let libd = make_test_resolved_pkg("libd", vec![], vec!["liba"]);
        let app = make_test_resolved_pkg("app", vec!["libc"], vec!["libd"]);

        let plan = ResolutionPlan::new(vec![liba, libb, libc, libd, app]);
        let stages = plan.stages();

        assert_eq!(stages.len(), 3);

        // Stage 0: liba and libb
        assert_eq!(stages[0].stage_index, 0);
        let s0_names: Vec<&str> = stages[0]
            .packages
            .iter()
            .map(|p| p.manifest.name.as_str())
            .collect();
        assert_eq!(s0_names, vec!["liba", "libb"]);

        // Stage 1: libc and libd
        assert_eq!(stages[1].stage_index, 1);
        let s1_names: Vec<&str> = stages[1]
            .packages
            .iter()
            .map(|p| p.manifest.name.as_str())
            .collect();
        assert_eq!(s1_names, vec!["libc", "libd"]);

        // Stage 2: app
        assert_eq!(stages[2].stage_index, 2);
        let s2_names: Vec<&str> = stages[2]
            .packages
            .iter()
            .map(|p| p.manifest.name.as_str())
            .collect();
        assert_eq!(s2_names, vec!["app"]);
    }
}
