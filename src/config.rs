//! Runtime configuration from `INVOICE__*` environment variables.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use config::{Config as RawConfig, Environment};
use serde::Deserialize;

use crate::ares::DEFAULT_ARES_URL;
use crate::cnb::DEFAULT_CNB_URL;
use crate::pdf::DEFAULT_MDCAST_URL;
use crate::secret::Secret;

pub const DEFAULT_BIND: &str = "0.0.0.0:3000";
pub const DEFAULT_STORAGE_DIR: &str = "./data";

/// Full server configuration (`invoice serve`).
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: Secret<String>,
    pub api_token: Secret<String>,
    pub bind: String,
    /// ARES REST root (`INVOICE__ARES_URL`).
    pub ares_url: String,
    /// ČNB `denni_kurz.txt` URL (`INVOICE__CNB_URL`).
    pub cnb_url: String,
    /// mdcast render service root (`INVOICE__MDCAST_URL`; empty → default).
    pub mdcast_url: String,
    /// Optional Bearer token for mdcast (`INVOICE__MDCAST_TOKEN`).
    pub mdcast_token: Option<Secret<String>>,
    /// Optional design override directory (`INVOICE__DESIGN_DIR`; empty → unset).
    pub design_dir: Option<PathBuf>,
    /// Archive root (`INVOICE__STORAGE_DIR`), created at start.
    pub storage_dir: PathBuf,
}

/// The subset `invoice migrate` needs — migrations must not require the API token.
#[derive(Debug, Clone, Deserialize)]
pub struct DbConfig {
    pub database_url: Secret<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Self::from_source(None)
    }

    /// `env` overrides the process environment (used by tests).
    pub fn from_source(env: Option<HashMap<String, String>>) -> Result<Self> {
        let cfg: Self = build(env)?.try_deserialize().context(
            "invalid configuration (INVOICE__DATABASE_URL and INVOICE__API_TOKEN are required)",
        )?;
        let cfg = cfg.normalized();
        cfg.validate()?;
        Ok(cfg)
    }

    /// CI and container runtimes pass unset variables through as empty
    /// strings; those mean "use the default", not "the empty value".
    fn normalized(mut self) -> Self {
        if self.mdcast_url.trim().is_empty() {
            self.mdcast_url = DEFAULT_MDCAST_URL.to_string();
        }
        if self
            .mdcast_token
            .as_ref()
            .is_some_and(|t| t.expose().trim().is_empty())
        {
            self.mdcast_token = None;
        }
        if self
            .design_dir
            .as_ref()
            .is_some_and(|d| d.as_os_str().is_empty())
        {
            self.design_dir = None;
        }
        if self.storage_dir.as_os_str().is_empty() {
            self.storage_dir = PathBuf::from(DEFAULT_STORAGE_DIR);
        }
        self
    }

    pub fn validate(&self) -> Result<()> {
        if self.api_token.expose().trim().is_empty() {
            bail!("INVOICE__API_TOKEN must not be empty");
        }
        if let Some(dir) = &self.design_dir
            && !dir.is_dir()
        {
            bail!(
                "INVOICE__DESIGN_DIR {} is not an existing directory",
                dir.display()
            );
        }
        validate_database_url(&self.database_url)
    }
}

impl DbConfig {
    pub fn from_env() -> Result<Self> {
        Self::from_source(None)
    }

    pub fn from_source(env: Option<HashMap<String, String>>) -> Result<Self> {
        let cfg: Self = build(env)?
            .try_deserialize()
            .context("invalid configuration (INVOICE__DATABASE_URL is required)")?;
        validate_database_url(&cfg.database_url)?;
        Ok(cfg)
    }
}

fn validate_database_url(url: &Secret<String>) -> Result<()> {
    if url.expose().trim().is_empty() {
        bail!("INVOICE__DATABASE_URL must not be empty");
    }
    Ok(())
}

fn build(env: Option<HashMap<String, String>>) -> Result<RawConfig> {
    RawConfig::builder()
        .set_default("bind", DEFAULT_BIND)
        .and_then(|b| b.set_default("ares_url", DEFAULT_ARES_URL))
        .and_then(|b| b.set_default("cnb_url", DEFAULT_CNB_URL))
        .and_then(|b| b.set_default("mdcast_url", DEFAULT_MDCAST_URL))
        .and_then(|b| b.set_default("storage_dir", DEFAULT_STORAGE_DIR))
        .context("set config defaults")?
        .add_source(
            Environment::with_prefix("INVOICE")
                .prefix_separator("__")
                .separator("__")
                .source(env),
        )
        .build()
        .context("read configuration from environment")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &[(&str, &str)]) -> Option<HashMap<String, String>> {
        Some(
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        )
    }

    #[test]
    fn loads_with_default_bind() {
        let cfg = Config::from_source(env(&[
            ("INVOICE__DATABASE_URL", "postgres://x"),
            ("INVOICE__API_TOKEN", "tok"),
        ]))
        .expect("valid config");
        assert_eq!(cfg.bind, DEFAULT_BIND);
        assert_eq!(cfg.ares_url, DEFAULT_ARES_URL);
        assert_eq!(cfg.cnb_url, DEFAULT_CNB_URL);
        assert_eq!(cfg.mdcast_url, DEFAULT_MDCAST_URL);
        assert!(cfg.mdcast_token.is_none());
        assert!(cfg.design_dir.is_none());
        assert_eq!(cfg.storage_dir, PathBuf::from(DEFAULT_STORAGE_DIR));
        assert_eq!(cfg.api_token.expose(), "tok");
        assert_eq!(cfg.database_url.expose(), "postgres://x");
    }

    #[test]
    fn bind_is_overridable() {
        let cfg = Config::from_source(env(&[
            ("INVOICE__DATABASE_URL", "postgres://x"),
            ("INVOICE__API_TOKEN", "tok"),
            ("INVOICE__BIND", "127.0.0.1:9000"),
            ("INVOICE__ARES_URL", "http://127.0.0.1:9001/rest"),
            ("INVOICE__CNB_URL", "http://127.0.0.1:9002/kurz.txt"),
        ]))
        .expect("valid config");
        assert_eq!(cfg.bind, "127.0.0.1:9000");
        assert_eq!(cfg.ares_url, "http://127.0.0.1:9001/rest");
        assert_eq!(cfg.cnb_url, "http://127.0.0.1:9002/kurz.txt");
    }

    #[test]
    fn pdf_settings_are_read() {
        let dir = tempfile::tempdir().expect("temp dir");
        let design = dir.path().to_string_lossy().to_string();
        let cfg = Config::from_source(env(&[
            ("INVOICE__DATABASE_URL", "postgres://x"),
            ("INVOICE__API_TOKEN", "tok"),
            ("INVOICE__MDCAST_URL", "http://127.0.0.1:9003"),
            ("INVOICE__MDCAST_TOKEN", "mdtok"),
            ("INVOICE__DESIGN_DIR", &design),
            ("INVOICE__STORAGE_DIR", "/srv/data"),
        ]))
        .expect("valid config");
        assert_eq!(cfg.mdcast_url, "http://127.0.0.1:9003");
        assert_eq!(
            cfg.mdcast_token.as_ref().map(|t| t.expose().as_str()),
            Some("mdtok")
        );
        assert_eq!(cfg.design_dir, Some(dir.path().to_path_buf()));
        assert_eq!(cfg.storage_dir, PathBuf::from("/srv/data"));
        assert!(!format!("{cfg:?}").contains("mdtok"));
    }

    #[test]
    fn empty_pdf_settings_mean_defaults() {
        let cfg = Config::from_source(env(&[
            ("INVOICE__DATABASE_URL", "postgres://x"),
            ("INVOICE__API_TOKEN", "tok"),
            ("INVOICE__MDCAST_URL", " "),
            ("INVOICE__MDCAST_TOKEN", ""),
            ("INVOICE__DESIGN_DIR", ""),
            ("INVOICE__STORAGE_DIR", ""),
        ]))
        .expect("valid config");
        assert_eq!(cfg.storage_dir, PathBuf::from(DEFAULT_STORAGE_DIR));
        assert_eq!(cfg.mdcast_url, DEFAULT_MDCAST_URL);
        assert!(cfg.mdcast_token.is_none());
        assert!(cfg.design_dir.is_none());
    }

    #[test]
    fn missing_design_dir_is_rejected() {
        let dir = tempfile::tempdir().expect("temp dir");
        let file = dir.path().join("not-a-dir.txt");
        std::fs::write(&file, "x").expect("write file");
        for path in [dir.path().join("missing"), file] {
            let err = Config::from_source(env(&[
                ("INVOICE__DATABASE_URL", "postgres://x"),
                ("INVOICE__API_TOKEN", "tok"),
                ("INVOICE__DESIGN_DIR", &path.to_string_lossy()),
            ]))
            .expect_err("design dir must exist");
            assert!(err.to_string().contains("INVOICE__DESIGN_DIR"), "{err}");
        }
    }

    #[test]
    fn empty_token_is_rejected() {
        let err = Config::from_source(env(&[
            ("INVOICE__DATABASE_URL", "postgres://x"),
            ("INVOICE__API_TOKEN", "   "),
        ]))
        .expect_err("empty token must fail");
        assert!(err.to_string().contains("INVOICE__API_TOKEN"));
    }

    #[test]
    fn missing_token_is_rejected() {
        assert!(Config::from_source(env(&[("INVOICE__DATABASE_URL", "postgres://x")])).is_err());
    }

    #[test]
    fn missing_database_url_is_rejected() {
        assert!(Config::from_source(env(&[("INVOICE__API_TOKEN", "tok")])).is_err());
        assert!(DbConfig::from_source(env(&[])).is_err());
    }

    #[test]
    fn db_config_does_not_need_the_token() {
        let cfg = DbConfig::from_source(env(&[("INVOICE__DATABASE_URL", "postgres://x")]))
            .expect("valid db config");
        assert_eq!(cfg.database_url.expose(), "postgres://x");
    }

    #[test]
    fn debug_never_prints_secrets() {
        let cfg = Config::from_source(env(&[
            ("INVOICE__DATABASE_URL", "postgres://u:pw@h/db"),
            ("INVOICE__API_TOKEN", "supersecret"),
        ]))
        .expect("valid config");
        let dbg = format!("{cfg:?}");
        assert!(!dbg.contains("supersecret"));
        assert!(!dbg.contains("pw@"));
    }
}
