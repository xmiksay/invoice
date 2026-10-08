//! Runtime configuration from `INVOICE__*` environment variables.

use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use config::{Config as RawConfig, Environment};
use serde::Deserialize;

use crate::ares::DEFAULT_ARES_URL;
use crate::secret::Secret;

pub const DEFAULT_BIND: &str = "0.0.0.0:3000";

/// Full server configuration (`invoice serve`).
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: Secret<String>,
    pub api_token: Secret<String>,
    pub bind: String,
    /// ARES REST root (`INVOICE__ARES_URL`).
    pub ares_url: String,
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
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn validate(&self) -> Result<()> {
        if self.api_token.expose().trim().is_empty() {
            bail!("INVOICE__API_TOKEN must not be empty");
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
        ]))
        .expect("valid config");
        assert_eq!(cfg.bind, "127.0.0.1:9000");
        assert_eq!(cfg.ares_url, "http://127.0.0.1:9001/rest");
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
