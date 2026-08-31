mod args;
mod commands;

use clap::Parser;
use rivet_core::InstalledDatabase;
use rivet_repository::load_repositories;

fn main() -> anyhow::Result<()> {
    let cli = args::Cli::parse();

    match cli.command {
        args::Commands::Init => {
            commands::init::execute()?;
        }
        args::Commands::Install {
            packages,
            dry_run,
            feature,
        } => {
            let repos = load_repositories(cli.repo.as_deref())?;

            let scope = if cli.system {
                rivet_core::InstallScope::System
            } else {
                rivet_core::InstallScope::User
            };
            scope.check_permitted()?;

            let db_path = match cli.db {
                Some(path) => rivet_core::absolute_path(path)?,
                None => scope.default_db_path()?,
            };
            let prefix = match cli.prefix {
                Some(path) => rivet_core::absolute_path(path)?,
                None => scope.default_prefix()?,
            };
            let cache_dir = match cli.cache {
                Some(path) => rivet_core::absolute_path(path)?,
                None => rivet_core::default_source_cache()?,
            };

            let mut db = InstalledDatabase::open(db_path)?;
            commands::install::execute(
                &repos, &packages, dry_run, &feature, &mut db, &prefix, &cache_dir,
            )?;
        }
        args::Commands::Search { query } => {
            let repos = load_repositories(cli.repo.as_deref())?;
            commands::search::execute(&repos, &query)?;
        }
        args::Commands::Info { package } => {
            let repos = load_repositories(cli.repo.as_deref())?;

            let scope = if cli.system {
                rivet_core::InstallScope::System
            } else {
                rivet_core::InstallScope::User
            };
            let db_path = match cli.db {
                Some(path) => rivet_core::absolute_path(path)?,
                None => scope.default_db_path()?,
            };
            let db = InstalledDatabase::open(db_path)?;

            commands::info::execute(&repos, &db, &package)?;
        }
        args::Commands::Sync => {
            let mut repos = load_repositories(cli.repo.as_deref())?;
            commands::sync::execute(&mut repos)?;
        }
        args::Commands::Build { recipe, check_only } => {
            commands::build::execute(&recipe, check_only)?;
        }
        args::Commands::Remove { package, force } => {
            let scope = if cli.system {
                rivet_core::InstallScope::System
            } else {
                rivet_core::InstallScope::User
            };
            scope.check_permitted()?;

            let db_path = match cli.db {
                Some(path) => rivet_core::absolute_path(path)?,
                None => scope.default_db_path()?,
            };
            let mut db = InstalledDatabase::open(db_path)?;
            let prefix = match cli.prefix {
                Some(path) => rivet_core::absolute_path(path)?,
                None => scope.default_prefix()?,
            };
            let cache_dir = match cli.cache {
                Some(path) => rivet_core::absolute_path(path)?,
                None => rivet_core::default_source_cache()?,
            };
            commands::remove::execute(&mut db, &package, &cache_dir, &prefix, force)?;
        }
        args::Commands::Autoremove { dry_run } => {
            let scope = if cli.system {
                rivet_core::InstallScope::System
            } else {
                rivet_core::InstallScope::User
            };
            scope.check_permitted()?;

            let db_path = match cli.db {
                Some(path) => rivet_core::absolute_path(path)?,
                None => scope.default_db_path()?,
            };
            let mut db = InstalledDatabase::open(db_path)?;
            let prefix = match cli.prefix {
                Some(path) => rivet_core::absolute_path(path)?,
                None => scope.default_prefix()?,
            };
            let cache_dir = match cli.cache {
                Some(path) => rivet_core::absolute_path(path)?,
                None => rivet_core::default_source_cache()?,
            };
            commands::autoremove::execute(&mut db, &cache_dir, &prefix, dry_run)?;
        }
        args::Commands::List => {
            let scope = if cli.system {
                rivet_core::InstallScope::System
            } else {
                rivet_core::InstallScope::User
            };
            let db_path = match cli.db {
                Some(path) => rivet_core::absolute_path(path)?,
                None => scope.default_db_path()?,
            };
            let db = InstalledDatabase::open(db_path)?;
            commands::list::execute(&db)?;
        }
        args::Commands::Update => {
            let mut repos = load_repositories(cli.repo.as_deref())?;

            let scope = if cli.system {
                rivet_core::InstallScope::System
            } else {
                rivet_core::InstallScope::User
            };
            let db_path = match cli.db {
                Some(path) => rivet_core::absolute_path(path)?,
                None => scope.default_db_path()?,
            };
            let db = InstalledDatabase::open(db_path)?;

            commands::update::execute(&mut repos, &db)?;
        }
        args::Commands::Upgrade { packages, dry_run } => {
            let repos = load_repositories(cli.repo.as_deref())?;

            let scope = if cli.system {
                rivet_core::InstallScope::System
            } else {
                rivet_core::InstallScope::User
            };
            scope.check_permitted()?;

            let db_path = match cli.db {
                Some(path) => rivet_core::absolute_path(path)?,
                None => scope.default_db_path()?,
            };
            let prefix = match cli.prefix {
                Some(path) => rivet_core::absolute_path(path)?,
                None => scope.default_prefix()?,
            };
            let cache_dir = match cli.cache {
                Some(path) => rivet_core::absolute_path(path)?,
                None => rivet_core::default_source_cache()?,
            };

            let mut db = InstalledDatabase::open(db_path)?;
            commands::upgrade::execute(&repos, &packages, &mut db, &prefix, &cache_dir, dry_run)?;
        }
        args::Commands::Clean => {
            let cache_dir = match cli.cache {
                Some(path) => rivet_core::absolute_path(path)?,
                None => rivet_core::default_source_cache()?,
            };
            commands::clean::execute(&cache_dir)?;
        }
        args::Commands::Verify { packages } => {
            let scope = if cli.system {
                rivet_core::InstallScope::System
            } else {
                rivet_core::InstallScope::User
            };
            let db_path = match cli.db {
                Some(path) => rivet_core::absolute_path(path)?,
                None => scope.default_db_path()?,
            };
            let db = InstalledDatabase::open(db_path)?;
            commands::verify::execute(&db, &packages)?;
        }
        args::Commands::Repo { command } => {
            commands::repo::execute(command)?;
        }
    }

    Ok(())
}
