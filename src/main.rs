use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use invoice::app::{self, AppState};
use invoice::ares::AresClient;
use invoice::cnb::CnbClient;
use invoice::config::{Config, DbConfig};
use invoice::migration::{Migrator, MigratorTrait};
use invoice::pdf::{PdfService, design};
use invoice::storage::transfer::{self, Report};
use invoice::storage::{DESIGN_PREFIX, Storage, StorageConfig};
use sea_orm::{Database, DatabaseConnection};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "invoice", version, about = "Single-user invoice management")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Apply pending migrations, then serve the API and SPA.
    Serve,
    /// Manage database migrations.
    Migrate {
        #[command(subcommand)]
        action: MigrateAction,
    },
    /// File storage maintenance (the configured INVOICE__STORAGE_KIND backend).
    Storage {
        #[command(subcommand)]
        action: StorageAction,
    },
    /// PDF design overrides kept in the storage under `design/`.
    Design {
        #[command(subcommand)]
        action: DesignAction,
    },
}

#[derive(Subcommand)]
enum StorageAction {
    /// Copy a filesystem storage directory (and an old design directory) into
    /// the configured backend. Verifies sha256, skips identical files, never
    /// overwrites different ones, never deletes the source.
    Migrate {
        /// The old INVOICE__STORAGE_DIR (keys = relative paths).
        #[arg(long)]
        from_dir: PathBuf,
        /// The old INVOICE__DESIGN_DIR (copied under `design/`).
        #[arg(long)]
        design_dir: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum DesignAction {
    /// Upload every file of DIR to `design/…` (changed files replaced).
    Push { dir: PathBuf },
    /// Download every `design/…` file into DIR.
    Pull { dir: PathBuf },
    /// List the effective design: custom files and the built-in defaults.
    Ls,
    /// Remove custom design files (the built-in default shows through again).
    Rm {
        #[arg(required = true)]
        paths: Vec<String>,
    },
}

#[derive(Subcommand)]
enum MigrateAction {
    /// Apply all pending migrations.
    Up,
    /// Show applied and pending migrations.
    Status,
    /// Roll back the last N migrations.
    Down {
        #[arg(short = 'n', long, default_value_t = 1)]
        n: u32,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new("invoice=info,tower_http=info,sea_orm_migration=info")
        }))
        .init();

    match Cli::parse().command {
        Command::Serve => serve().await,
        Command::Migrate { action } => migrate(action).await,
        Command::Storage { action } => storage_cmd(action).await,
        Command::Design { action } => design_cmd(action).await,
    }
}

async fn connect(url: &str) -> Result<DatabaseConnection> {
    Database::connect(url)
        .await
        .context("connect to database (INVOICE__DATABASE_URL)")
}

async fn serve() -> Result<()> {
    let cfg = Config::from_env()?;
    let db = connect(cfg.database_url.expose()).await?;
    Migrator::up(&db, None)
        .await
        .context("apply pending migrations")?;

    let storage = Storage::new(&cfg.storage)?;
    tokio::spawn(probe(storage.clone()));
    let state = AppState {
        db,
        api_token: cfg.api_token,
        ares: AresClient::new(&cfg.ares_url)?,
        cnb: CnbClient::new(&cfg.cnb_url)?,
        pdf: PdfService::new(
            &cfg.mdcast_url,
            cfg.mdcast_token.as_ref().map(|t| t.expose().as_str()),
            storage,
        )?,
    };
    let listener = tokio::net::TcpListener::bind(&cfg.bind)
        .await
        .with_context(|| format!("bind {}", cfg.bind))?;
    tracing::info!(bind = %cfg.bind, "listening");
    axum::serve(listener, app::router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("http server")
}

async fn migrate(action: MigrateAction) -> Result<()> {
    let cfg = DbConfig::from_env()?;
    let db = connect(cfg.database_url.expose()).await?;
    match action {
        MigrateAction::Up => Migrator::up(&db, None).await,
        MigrateAction::Status => Migrator::status(&db).await,
        MigrateAction::Down { n } => Migrator::down(&db, Some(n)).await,
    }
    .context("run migrations")
}

/// Startup reachability check. Only logs: requests touching the storage
/// answer 503 `storage_unavailable` while it is down.
async fn probe(storage: Storage) {
    match storage.list(DESIGN_PREFIX).await {
        Ok(design) => tracing::info!(
            kind = storage.kind(),
            design_files = design.len(),
            "storage reachable"
        ),
        Err(e) => tracing::error!(kind = storage.kind(), error = %e, "storage unreachable"),
    }
}

fn open_storage() -> Result<Storage> {
    Storage::new(&StorageConfig::from_env()?)
}

fn finish(report: &Report) -> Result<()> {
    for key in &report.copied {
        println!("copied     {key}");
    }
    for problem in &report.problems {
        eprintln!("problem    {problem}");
    }
    println!("{}", report.summary());
    if !report.problems.is_empty() {
        bail!("{} file(s) not copied", report.problems.len());
    }
    Ok(())
}

async fn storage_cmd(action: StorageAction) -> Result<()> {
    let storage = open_storage()?;
    match action {
        StorageAction::Migrate {
            from_dir,
            design_dir,
        } => finish(&transfer::migrate(&storage, &from_dir, design_dir.as_deref()).await?),
    }
}

async fn design_cmd(action: DesignAction) -> Result<()> {
    let storage = open_storage()?;
    match action {
        DesignAction::Push { dir } => {
            let mut report = Report::default();
            transfer::copy_tree(&storage, &dir, Some(DESIGN_PREFIX), true, &mut report).await?;
            finish(&report)
        }
        DesignAction::Pull { dir } => {
            for rel in transfer::pull(&storage, DESIGN_PREFIX, &dir).await? {
                println!("{rel}");
            }
            Ok(())
        }
        DesignAction::Ls => {
            for f in design::list(&storage).await? {
                let source = match f.source {
                    design::Source::Custom => "custom",
                    design::Source::Default => "default",
                };
                println!("{source:<8} {:>10}  {}", f.size, f.path);
            }
            Ok(())
        }
        DesignAction::Rm { paths } => {
            let mut missing = 0;
            for path in paths {
                if design::remove(&storage, &path).await? {
                    println!("removed    {path}");
                } else {
                    eprintln!("not found  {path}");
                    missing += 1;
                }
            }
            if missing > 0 {
                bail!("{missing} design file(s) not found");
            }
            Ok(())
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!(error = %e, "install Ctrl+C handler");
            std::future::pending::<()>().await;
        }
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(e) => {
                tracing::error!(error = %e, "install SIGTERM handler");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("shutting down");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parses_subcommands() {
        assert!(matches!(
            Cli::try_parse_from(["invoice", "serve"]).map(|c| c.command),
            Ok(Command::Serve)
        ));
        assert!(matches!(
            Cli::try_parse_from(["invoice", "migrate", "down", "-n", "3"]).map(|c| c.command),
            Ok(Command::Migrate {
                action: MigrateAction::Down { n: 3 }
            })
        ));
        assert!(matches!(
            Cli::try_parse_from(["invoice", "migrate", "down"]).map(|c| c.command),
            Ok(Command::Migrate {
                action: MigrateAction::Down { n: 1 }
            })
        ));
        assert!(Cli::try_parse_from(["invoice", "bogus"]).is_err());
        assert!(matches!(
            Cli::try_parse_from([
                "invoice", "storage", "migrate", "--from-dir", "/data", "--design-dir", "/design"
            ])
            .map(|c| c.command),
            Ok(Command::Storage {
                action: StorageAction::Migrate { from_dir, design_dir: Some(d) }
            }) if from_dir.as_path() == std::path::Path::new("/data") && d.as_path() == std::path::Path::new("/design")
        ));
        assert!(Cli::try_parse_from(["invoice", "storage", "migrate"]).is_err());
        assert!(matches!(
            Cli::try_parse_from(["invoice", "design", "push", "./my-design"]).map(|c| c.command),
            Ok(Command::Design {
                action: DesignAction::Push { .. }
            })
        ));
        assert!(matches!(
            Cli::try_parse_from(["invoice", "design", "ls"]).map(|c| c.command),
            Ok(Command::Design {
                action: DesignAction::Ls
            })
        ));
        assert!(Cli::try_parse_from(["invoice", "design", "rm"]).is_err());
    }
}
