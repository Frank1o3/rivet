use std::path::{Path, PathBuf};

use rivet_core::Target;
use rivet_package::{Dependency, DependencyKind, PackageManifest};
use rivet_repository::MultiRepositoryManager;
use rivet_resolver::{DependencySolver, PackageProvider, ResolutionPlan};

#[derive(Debug, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
}

fn explicit_repo_path() -> Option<PathBuf> {
    ["packages", "recipes"]
        .iter()
        .map(Path::new)
        .find(|dir| dir.exists())
        .map(|dir| dir.to_path_buf())
}

pub struct App {
    pub repos: MultiRepositoryManager,
    pub all_packages: Vec<PackageManifest>,
    pub filtered_packages: Vec<PackageManifest>,
    pub selected_index: usize,
    pub search_query: String,
    pub input_mode: InputMode,
    pub status_message: String,
    pub resolution_plan: Option<ResolutionPlan>,
    pub show_modal: bool,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        let repos = rivet_repository::load_repositories(explicit_repo_path().as_deref())
            .unwrap_or_default();

        let mut app = Self {
            repos,
            all_packages: Vec::new(),
            filtered_packages: Vec::new(),
            selected_index: 0,
            search_query: String::new(),
            input_mode: InputMode::Normal,
            status_message:
                "Ready. Press [/] to search, [Enter] to resolve, [s] to sync, [q] to quit."
                    .to_string(),
            resolution_plan: None,
            show_modal: false,
            should_quit: false,
        };

        app.reload_packages();
        app
    }

    pub fn reload_packages(&mut self) {
        let results = self.repos.search("");
        let mut pkgs: Vec<PackageManifest> = Vec::new();
        for summary in results {
            if let Ok(name) = rivet_core::PackageName::new(&summary.name) {
                let candidates = self.repos.get_candidates(&name);
                if let Some(first) = candidates.into_iter().next() {
                    pkgs.push(first);
                }
            }
        }
        pkgs.sort_by(|a, b| a.name.cmp(&b.name));
        self.all_packages = pkgs;
        self.apply_filter();
    }

    pub fn apply_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_packages = self.all_packages.clone();
        } else {
            let query = self.search_query.to_lowercase();
            self.filtered_packages = self
                .all_packages
                .iter()
                .filter(|p| {
                    p.name.as_str().to_lowercase().contains(&query)
                        || p.description
                            .as_ref()
                            .map(|d| d.to_lowercase().contains(&query))
                            .unwrap_or(false)
                })
                .cloned()
                .collect();
        }

        if self.selected_index >= self.filtered_packages.len() && !self.filtered_packages.is_empty()
        {
            self.selected_index = self.filtered_packages.len() - 1;
        }
    }

    pub fn selected_package(&self) -> Option<&PackageManifest> {
        self.filtered_packages.get(self.selected_index)
    }

    pub fn next(&mut self) {
        if !self.filtered_packages.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.filtered_packages.len();
        }
    }

    pub fn previous(&mut self) {
        if !self.filtered_packages.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.filtered_packages.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    pub fn sync_repositories(&mut self) {
        match self.repos.scan_all() {
            Ok(count) => {
                self.reload_packages();
                self.status_message = format!("Indexed {} packages successfully.", count);
            }
            Err(e) => {
                self.status_message = format!("Sync failed: {}", e);
            }
        }
    }

    pub fn resolve_selected(&mut self) {
        if let Some(pkg) = self.selected_package() {
            let target = Target::host();
            let solver = DependencySolver::new(&self.repos, &target);
            let root = Dependency::new(
                pkg.name.clone(),
                rivet_core::VersionReq::STAR,
                DependencyKind::Runtime,
            );

            match solver.resolve(&[root]) {
                Ok(plan) => {
                    self.status_message =
                        format!("Resolved {} package(s) for {}", plan.len(), pkg.name);
                    self.resolution_plan = Some(plan);
                    self.show_modal = true;
                }
                Err(e) => {
                    self.status_message = format!("Resolution error: {}", e);
                    self.resolution_plan = None;
                    self.show_modal = false;
                }
            }
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
