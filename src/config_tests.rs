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
        ("INVOICE__PUBLIC_URL", "http://localhost:3000"),
    ]))
    .expect("valid config");
    assert_eq!(cfg.bind, DEFAULT_BIND);
    assert_eq!(cfg.ares_url, DEFAULT_ARES_URL);
    assert_eq!(cfg.cnb_url, DEFAULT_CNB_URL);
    assert_eq!(cfg.mdcast_url, DEFAULT_MDCAST_URL);
    assert!(cfg.mdcast_token.is_none());
    assert_eq!(cfg.storage, StorageConfig::default());
    assert_eq!(cfg.public.base_url(), "http://localhost:3000");
    assert!(!cfg.registration);
    assert!(!cfg.trust_forwarded);
    assert_eq!(cfg.database_url.expose(), "postgres://x");
}

#[test]
fn bind_is_overridable() {
    let cfg = Config::from_source(env(&[
        ("INVOICE__DATABASE_URL", "postgres://x"),
        ("INVOICE__PUBLIC_URL", "http://localhost:3000"),
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
    let cfg = Config::from_source(env(&[
        ("INVOICE__DATABASE_URL", "postgres://x"),
        ("INVOICE__PUBLIC_URL", "http://localhost:3000"),
        ("INVOICE__MDCAST_URL", "http://127.0.0.1:9003"),
        ("INVOICE__MDCAST_TOKEN", "mdtok"),
        ("INVOICE__STORAGE_DIR", "/srv/data"),
    ]))
    .expect("valid config");
    assert_eq!(cfg.mdcast_url, "http://127.0.0.1:9003");
    assert_eq!(
        cfg.mdcast_token.as_ref().map(|t| t.expose().as_str()),
        Some("mdtok")
    );
    assert_eq!(
        cfg.storage,
        StorageConfig::Fs {
            dir: "/srv/data".into()
        }
    );
    assert!(!format!("{cfg:?}").contains("mdtok"));
}

#[test]
fn empty_pdf_settings_mean_defaults() {
    let cfg = Config::from_source(env(&[
        ("INVOICE__DATABASE_URL", "postgres://x"),
        ("INVOICE__PUBLIC_URL", "http://localhost:3000"),
        ("INVOICE__MDCAST_URL", " "),
        ("INVOICE__MDCAST_TOKEN", ""),
        ("INVOICE__STORAGE_DIR", ""),
    ]))
    .expect("valid config");
    assert_eq!(cfg.storage, StorageConfig::default());
    assert_eq!(cfg.mdcast_url, DEFAULT_MDCAST_URL);
    assert!(cfg.mdcast_token.is_none());
}

#[test]
fn leftover_design_dir_is_rejected() {
    let err = Config::from_source(env(&[
        ("INVOICE__DATABASE_URL", "postgres://x"),
        ("INVOICE__PUBLIC_URL", "http://localhost:3000"),
        ("INVOICE__DESIGN_DIR", "/srv/design"),
    ]))
    .expect_err("removed setting");
    let msg = err.to_string();
    assert!(msg.contains("INVOICE__DESIGN_DIR"), "{msg}");
    assert!(msg.contains("invoice storage migrate"), "{msg}");
    Config::from_source(env(&[
        ("INVOICE__DATABASE_URL", "postgres://x"),
        ("INVOICE__PUBLIC_URL", "http://localhost:3000"),
        ("INVOICE__DESIGN_DIR", ""),
    ]))
    .expect("an empty leftover is unset");
}

#[test]
fn invalid_storage_is_rejected() {
    let err = Config::from_source(env(&[
        ("INVOICE__DATABASE_URL", "postgres://x"),
        ("INVOICE__PUBLIC_URL", "http://localhost:3000"),
        ("INVOICE__STORAGE_KIND", "s3"),
    ]))
    .expect_err("s3 without a bucket");
    assert!(
        format!("{err:#}").contains("INVOICE__S3__BUCKET"),
        "{err:#}"
    );
}

#[test]
fn smtp_is_optional_and_validated() {
    let base = [
        ("INVOICE__DATABASE_URL", "postgres://x"),
        ("INVOICE__PUBLIC_URL", "http://localhost:3000"),
    ];
    assert!(
        Config::from_source(env(&base))
            .expect("valid")
            .smtp
            .is_none()
    );
    let mut with_host = base.to_vec();
    with_host.push(("INVOICE__SMTP__HOST", "smtp.example.com"));
    let err = Config::from_source(env(&with_host)).expect_err("HOST without FROM");
    assert!(
        format!("{err:#}").contains("INVOICE__SMTP__FROM"),
        "{err:#}"
    );
    with_host.push(("INVOICE__SMTP__FROM", "faktury@example.com"));
    let cfg = Config::from_source(env(&with_host)).expect("valid");
    assert_eq!(cfg.smtp.map(|s| s.port), Some(587));
}

#[test]
fn public_url_is_required_and_validated() {
    assert!(Config::from_source(env(&[("INVOICE__DATABASE_URL", "postgres://x")])).is_err());
    let err = Config::from_source(env(&[
        ("INVOICE__DATABASE_URL", "postgres://x"),
        ("INVOICE__PUBLIC_URL", "invoiceapp.cz"),
    ]))
    .expect_err("no scheme");
    assert!(
        format!("{err:#}").contains("INVOICE__PUBLIC_URL"),
        "{err:#}"
    );
}

#[test]
fn leftover_api_token_is_ignored() {
    Config::from_source(env(&[
        ("INVOICE__DATABASE_URL", "postgres://x"),
        ("INVOICE__PUBLIC_URL", "https://invoiceapp.cz"),
        ("INVOICE__API_TOKEN", "old"),
    ]))
    .expect("ignored");
}

#[test]
fn switches_are_parsed() {
    let base = [
        ("INVOICE__DATABASE_URL", "postgres://x"),
        ("INVOICE__PUBLIC_URL", "https://invoiceapp.cz"),
    ];
    let mut with = base.to_vec();
    with.push(("INVOICE__TRUST_FORWARDED", "true"));
    with.push(("INVOICE__REGISTRATION", ""));
    let cfg = Config::from_source(env(&with)).expect("valid");
    assert!(cfg.trust_forwarded);
    assert!(!cfg.registration);
    let mut bad = base.to_vec();
    bad.push(("INVOICE__TRUST_FORWARDED", "yes"));
    assert!(Config::from_source(env(&bad)).is_err());
}

#[test]
fn registration_needs_smtp() {
    let mut with = vec![
        ("INVOICE__DATABASE_URL", "postgres://x"),
        ("INVOICE__PUBLIC_URL", "https://invoiceapp.cz"),
        ("INVOICE__REGISTRATION", "true"),
    ];
    let err = Config::from_source(env(&with)).expect_err("no SMTP");
    assert!(format!("{err:#}").contains("SMTP"), "{err:#}");
    with.push(("INVOICE__SMTP__HOST", "smtp.example.com"));
    with.push(("INVOICE__SMTP__FROM", "faktury@example.com"));
    assert!(Config::from_source(env(&with)).expect("valid").registration);
}

#[test]
fn missing_database_url_is_rejected() {
    assert!(Config::from_source(env(&[("INVOICE__PUBLIC_URL", "http://localhost:3000")])).is_err());
    assert!(DbConfig::from_source(env(&[])).is_err());
}

#[test]
fn db_config_needs_only_the_database() {
    let cfg = DbConfig::from_source(env(&[("INVOICE__DATABASE_URL", "postgres://x")]))
        .expect("valid db config");
    assert_eq!(cfg.database_url.expose(), "postgres://x");
}

#[test]
fn debug_never_prints_secrets() {
    let cfg = Config::from_source(env(&[
        ("INVOICE__DATABASE_URL", "postgres://u:pw@h/db"),
        ("INVOICE__PUBLIC_URL", "http://localhost:3000"),
        ("INVOICE__MDCAST_TOKEN", "supersecret"),
    ]))
    .expect("valid config");
    let dbg = format!("{cfg:?}");
    assert!(!dbg.contains("supersecret"));
    assert!(!dbg.contains("pw@"));
}
