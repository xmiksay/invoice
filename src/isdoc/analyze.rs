//! Upload → per-document outcome (status, warnings, lookups). Preview returns
//! it; confirm recomputes it (the server keeps nothing in between).

use std::collections::HashSet;

use anyhow::Context as _;
use sea_orm::DatabaseConnection;

use super::parse::{self, Code};
use super::plan::{self, Plan};
use super::upload::{self, Exceeded, File, LIMITS};
use crate::error::AppError;
use crate::import::check::{self, Checked};
use crate::settings::repo::company;

pub use super::upload::PDF_SKIPPED;
pub use crate::import::check::{CONTACT_CREATED, RELATED_NOT_FOUND, Status};

#[derive(Debug, Clone)]
pub struct Ready {
    pub plan: Plan,
    /// The original PDF to store.
    pub pdf: Option<bytes::Bytes>,
    /// The plan's own warnings (incl. [`PDF_SKIPPED`]), before the check's.
    pub warnings: Vec<Code>,
    pub checked: Checked,
}

#[derive(Debug, Clone)]
pub struct Analyzed {
    pub key: String,
    pub outcome: Result<Ready, Code>,
}

/// A plan, its original PDF and whether a too-large PDF was skipped.
type Planned = (Plan, Option<bytes::Bytes>, bool);
type Keyed = (String, Result<Planned, Code>);

/// Unpack, parse and plan: CPU-bound, run off the async workers.
fn plan_all(files: Vec<File>, company_ico: Option<String>) -> Result<Vec<Keyed>, Exceeded> {
    let found = upload::unpack(files, LIMITS)?;
    Ok(found
        .into_iter()
        .map(|f| {
            let outcome = f.doc.and_then(|doc| {
                let parsed = parse::parse(&doc.xml)?;
                let plan = plan::plan(parsed, company_ico.as_deref())?;
                let (pdf, skipped) = match upload::pdf_bytes(doc.pdf.as_ref()) {
                    Ok(pdf) => (pdf, false),
                    Err(_) => (None, true),
                };
                Ok((plan, pdf, skipped))
            });
            (f.key, outcome)
        })
        .collect())
}

/// `only`: run the database lookups for these keys only (confirm: the
/// selected ones); the in-batch duplicate pass always covers every entry.
pub async fn analyze(
    db: &DatabaseConnection,
    files: Vec<File>,
    only: Option<&HashSet<String>>,
) -> Result<Vec<Analyzed>, AppError> {
    let ico = company::get(db).await?.ico;
    let planned = tokio::task::spawn_blocking(move || plan_all(files, ico))
        .await
        .context("ISDOC unpacking panicked")?
        .map_err(|e| match e {
            Exceeded::TooMany => AppError::field("files", "too_many"),
            Exceeded::TooLarge => AppError::field("files", "too_large"),
        })?;
    let entries: Vec<(&str, Option<&Plan>)> = planned
        .iter()
        .map(|(k, o)| (k.as_str(), o.as_ref().ok().map(|(p, _, _)| p)))
        .collect();
    let checked = check::check(db, &entries, only).await?;
    Ok(planned
        .into_iter()
        .zip(checked)
        .map(|((key, outcome), checked)| {
            let outcome = match (outcome, checked) {
                (Ok((plan, pdf, skipped)), Some(checked)) => {
                    let mut warnings = plan.warnings.clone();
                    if skipped {
                        warnings.push(PDF_SKIPPED);
                    }
                    // Unselected entries of a confirm are never stored.
                    let pdf = pdf.filter(|_| only.is_none_or(|o| o.contains(&key)));
                    Ok(Ready {
                        plan,
                        pdf,
                        warnings,
                        checked,
                    })
                }
                (Err(code), _) => Err(code),
                (Ok(_), None) => Err(parse::INVALID_XML),
            };
            Analyzed { key, outcome }
        })
        .collect())
}
