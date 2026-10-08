use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use invoice::app::{self, AppState};
use invoice::ares::AresClient;
use invoice::cnb::CnbClient;
use invoice::config::{Config, DbConfig};
use invoice::migration::{Migrator, MigratorTrait};
use invoice::pdf::PdfService;
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

    let state = AppState {
        db,
        api_token: cfg.api_token,
        ares: AresClient::new(&cfg.ares_url)?,
        cnb: CnbClient::new(&cfg.cnb_url)?,
        pdf: PdfService::new(
            &cfg.mdcast_url,
            cfg.mdcast_token.as_ref().map(|t| t.expose().as_str()),
            cfg.design_dir.clone(),
            cfg.storage_dir.clone(),
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
    }
}
