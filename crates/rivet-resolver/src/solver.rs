use std::collections::{HashMap, HashSet, VecDeque};

use rivet_core::{FeatureSet, PackageName, Target, VersionReq};
use rivet_package::{Dependency, DependencyKind, PackageManifest};

use crate::error::{ResolverError, Result};
use crate::plan::{ResolutionPlan, ResolvedPackage};
use crate::provider::PackageProvider;

/// Pure dependency graph solver.
pub struct DependencySolver<'a, P: PackageProvider> {
    provider: &'a P,
    target: &'a Target,
}

impl<'a, P: PackageProvider> DependencySolver<'a, P> {
    /// Creates a new solver with a given package provider and target platform.
    pub fn new(provider: &'a P, target: &'a Target) -> Self {
        Self { provider, target }
    }

    /// Solves the dependency graph for a list of root dependencies.
    pub fn resolve(&self, root_deps: &[Dependency]) -> Result<ResolutionPlan> {
        if root_deps.is_empty() {
            return Ok(ResolutionPlan::default());
        }

        // 1. Map of package name -> list of (VersionReq, requested_by)
        let mut constraints: HashMap<PackageName, Vec<(VersionReq, Option<PackageName>)>> =
            HashMap::new();

        // 2. Map of package name -> (PackageManifest, FeatureSet)
        let mut selected: HashMap<PackageName, (PackageManifest, FeatureSet)> = HashMap::new();

        // Names of packages whose requirement was satisfied by a
        // system-detected capability rather than by Rivet itself.
        let mut system_provided: HashSet<PackageName> = HashSet::new();

        // 3. Work queue of dependencies to resolve: (Dependency, requested_by)
        let mut queue: VecDeque<(Dependency, Option<PackageName>)> = VecDeque::new();

        for dep in root_deps {
            queue.push_back((dep.clone(), None));
        }

        while let Some((dep, requested_by)) = queue.pop_front() {
            let pkg_name = &dep.name;
            constraints
                .entry(pkg_name.clone())
                .or_default()
                .push((dep.req.clone(), requested_by.clone()));

            // If already selected, check if current selection satisfies the new requirement
            if let Some((manifest, enabled_features)) = selected.get_mut(pkg_name) {
                if !dep.req.matches(&manifest.version) {
                    let reasons = constraints[pkg_name]
                        .iter()
                        .map(|(req, req_by)| {
                            format!(
                                "{} wanted by {}",
                                req,
                                req_by.as_ref().map(|p| p.as_str()).unwrap_or("root")
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", ");

                    return Err(ResolverError::VersionConflict {
                        name: pkg_name.clone(),
                        reason: format!(
                            "selected version {} does not satisfy all requirements ({})",
                            manifest.version, reasons
                        ),
                    });
                }

                // If this dependency enables a feature, activate it
                if let Some(feat) = dep.feature {
                    if !enabled_features.contains(&feat) {
                        enabled_features.insert(feat.clone());
                        // Enqueue dependencies belonging to this newly activated feature
                        if let Some(feat_deps) = manifest.features.get(&feat) {
                            for f_dep in feat_deps {
                                queue.push_back((f_dep.clone(), Some(pkg_name.clone())));
                            }
                        }
                    }
                }
                continue;
            }

            // Fetch candidates from provider
            let mut candidates = self.provider.get_candidates(pkg_name);
            if candidates.is_empty() {
                return Err(ResolverError::PackageNotFound {
                    name: pkg_name.clone(),
                    requested_by,
                });
            }

            // Filter by target platform
            let platform_supported = candidates.iter().any(|c| c.supports_target(self.target));

            if !platform_supported {
                let first_v = candidates[0].version.clone();
                return Err(ResolverError::UnsupportedPlatform {
                    name: pkg_name.clone(),
                    version: first_v,
                    target: self.target.clone(),
                });
            }

            candidates.retain(|c| c.supports_target(self.target));

            // Sort candidates by version descending (newest first)
            candidates.sort_by(|a, b| b.version.cmp(&a.version));

            // Find best candidate satisfying all constraints on this package
            let current_reqs = &constraints[pkg_name];
            let matching_candidate = candidates.into_iter().find(|cand| {
                current_reqs
                    .iter()
                    .all(|(req, _)| req.matches(&cand.version))
            });

            let chosen = match matching_candidate {
                Some(cand) => cand,
                None => {
                    let available_versions = self
                        .provider
                        .get_candidates(pkg_name)
                        .into_iter()
                        .map(|c| c.version)
                        .collect();

                    return Err(ResolverError::NoMatchingVersion {
                        name: pkg_name.clone(),
                        req: dep.req.clone(),
                        available: available_versions,
                    });
                }
            };

            let is_system_provided = chosen
                .provider_check
                .as_ref()
                .and_then(|check| check.detect())
                .map(|detected| current_reqs.iter().all(|(req, _)| req.matches(&detected)))
                .unwrap_or(false);

            if is_system_provided {
                system_provided.insert(pkg_name.clone());
            }

            // Initialize enabled features (defaults + requested)
            let mut enabled_features = FeatureSet::new();
            for default_feat in &chosen.default_features {
                enabled_features.insert(default_feat.clone());
            }
            if let Some(feat) = dep.feature {
                enabled_features.insert(feat);
            }

            // Queue all dependencies of the chosen package
            if !is_system_provided {
                // Queue all dependencies of the chosen package
                for sub_dep in &chosen.dependencies {
                    queue.push_back((sub_dep.clone(), Some(pkg_name.clone())));
                }

                // Queue feature-specific dependencies for all enabled features
                for feat in enabled_features.iter() {
                    if let Some(feat_deps) = chosen.features.get(feat) {
                        for f_dep in feat_deps {
                            queue.push_back((f_dep.clone(), Some(pkg_name.clone())));
                        }
                    }
                }
            }

            selected.insert(pkg_name.clone(), (chosen, enabled_features));
        }

        // 4. Topological Sort with Cycle Detection
        self.topological_sort(selected, &system_provided)
    }

    /// Performs topological sorting on the resolved packages.
    fn topological_sort(
        &self,
        selected: HashMap<PackageName, (PackageManifest, FeatureSet)>,
        system_provided: &HashSet<PackageName>,
    ) -> Result<ResolutionPlan> {
        let mut adj_list: HashMap<PackageName, Vec<PackageName>> = HashMap::new();
        let mut in_degree: HashMap<PackageName, usize> = HashMap::new();

        for name in selected.keys() {
            adj_list.entry(name.clone()).or_default();
            in_degree.entry(name.clone()).or_insert(0);
        }

        // Build edges: dependency -> dependent (so dependency has in_degree 0 initially)
        for (pkg_name, (manifest, enabled_features)) in &selected {
            let mut all_deps = Vec::new();
            for dep in &manifest.dependencies {
                if selected.contains_key(&dep.name) {
                    all_deps.push(&dep.name);
                }
            }
            for feat in enabled_features.iter() {
                if let Some(f_deps) = manifest.features.get(feat) {
                    for dep in f_deps {
                        if selected.contains_key(&dep.name) {
                            all_deps.push(&dep.name);
                        }
                    }
                }
            }

            for dep_name in all_deps {
                // Edge: dep_name -> pkg_name (dep must be installed before pkg)
                adj_list
                    .entry(dep_name.clone())
                    .or_default()
                    .push(pkg_name.clone());
                *in_degree.entry(pkg_name.clone()).or_insert(0) += 1;
            }
        }

        // Kahn's algorithm
        let mut ready: VecDeque<PackageName> = in_degree
            .iter()
            .filter(|&(_, &deg)| deg == 0)
            .map(|(name, _)| name.clone())
            .collect();

        let mut ordered = Vec::new();

        while let Some(current) = ready.pop_front() {
            ordered.push(current.clone());

            if let Some(neighbors) = adj_list.get(&current) {
                for neighbor in neighbors {
                    let deg = in_degree.get_mut(neighbor).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        ready.push_back(neighbor.clone());
                    }
                }
            }
        }

        if ordered.len() != selected.len() {
            // Find cycle
            let unvisited: Vec<PackageName> = in_degree
                .into_iter()
                .filter(|(_, deg)| *deg > 0)
                .map(|(name, _)| name)
                .collect();

            return Err(ResolverError::CyclicDependency { cycle: unvisited });
        }

        let mut final_packages = Vec::new();
        for name in ordered {
            let (manifest, enabled_features) = selected.get(&name).unwrap().clone();
            let mut build_deps = Vec::new();
            let mut runtime_deps = Vec::new();

            for dep in &manifest.dependencies {
                match dep.kind {
                    DependencyKind::Build => build_deps.push(dep.name.clone()),
                    DependencyKind::Runtime => runtime_deps.push(dep.name.clone()),
                }
            }

            final_packages.push(ResolvedPackage {
                manifest,
                enabled_features,
                build_dependencies: build_deps,
                runtime_dependencies: runtime_deps,
                is_system_provided: system_provided.contains(&name),
            });
        }

        Ok(ResolutionPlan::new(final_packages))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::InMemoryPackageProvider;
    use rivet_core::Target;
    use rivet_package::{Dependency, DependencyKind, PackageLoader};

    #[test]
    fn test_solver_transitive_resolution_and_ordering() {
        let loader = PackageLoader::new().unwrap();

        let zlib_script = r#"
            package({
                name = "zlib",
                version = "1.3.1",
            })
        "#;

        let libpng_script = r#"
            package({
                name = "libpng",
                version = "1.6.43",
                dependencies = { "zlib >= 1.2.0" },
            })
        "#;

        let neovim_script = r#"
            package({
                name = "neovim",
                version = "0.10.0",
                dependencies = { "libpng >= 1.6.0" },
            })
        "#;

        let mut provider = InMemoryPackageProvider::new();
        provider.add(loader.load_from_str(zlib_script).unwrap());
        provider.add(loader.load_from_str(libpng_script).unwrap());
        provider.add(loader.load_from_str(neovim_script).unwrap());

        let target = Target::host();
        let solver = DependencySolver::new(&provider, &target);

        let root_dep = Dependency::parse_shorthand("neovim", DependencyKind::Runtime).unwrap();
        let plan = solver.resolve(&[root_dep]).unwrap();

        assert_eq!(plan.len(), 3);
        let names: Vec<&str> = plan.iter().map(|p| p.manifest.name.as_str()).collect();
        // zlib must come first, then libpng, then neovim
        assert_eq!(names, vec!["zlib", "libpng", "neovim"]);
    }

    #[test]
    fn test_solver_detects_version_conflict() {
        let loader = PackageLoader::new().unwrap();

        let openssl1_script = r#"package({ name = "openssl", version = "1.1.1" })"#;
        let openssl3_script = r#"package({ name = "openssl", version = "3.0.0" })"#;

        let pkg_a_script = r#"
            package({
                name = "pkg-a",
                version = "1.0.0",
                dependencies = { "openssl >= 3.0.0" },
            })
        "#;

        let pkg_b_script = r#"
            package({
                name = "pkg-b",
                version = "1.0.0",
                dependencies = { "openssl < 2.0.0" },
            })
        "#;

        let root_script = r#"
            package({
                name = "my-app",
                version = "1.0.0",
                dependencies = { "pkg-a", "pkg-b" },
            })
        "#;

        let mut provider = InMemoryPackageProvider::new();
        provider.add(loader.load_from_str(openssl1_script).unwrap());
        provider.add(loader.load_from_str(openssl3_script).unwrap());
        provider.add(loader.load_from_str(pkg_a_script).unwrap());
        provider.add(loader.load_from_str(pkg_b_script).unwrap());
        provider.add(loader.load_from_str(root_script).unwrap());

        let target = Target::host();
        let solver = DependencySolver::new(&provider, &target);

        let root_dep = Dependency::parse_shorthand("my-app", DependencyKind::Runtime).unwrap();
        let result = solver.resolve(&[root_dep]);

        assert!(result.is_err());
    }
}
