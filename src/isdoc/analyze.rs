//! Upload → per-document outcome (status, warnings, lookups). Preview returns
//! it; confirm recomputes it (the server keeps nothing in between).

use std::collections::HashSet;

use anyhow::Context as _;
use sea_orm::DatabaseConnection;

use super::lookup;
use super::parse::{self, Code};
use super::plan::{self, Plan};
use super::upload::{self, Exceeded, File, LIMITS};
use crate::error::AppError;
use crate::settings::repo::company;

pub const RELATED_NOT_FOUND: Code = "related_not_found";
pub use super::upload::PDF_SKIPPED;
pub const CONTACT_CREATED: Code = "contact_created";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ok,
    Duplicate,
}

#[derive(Debug, Clone)]
pub struct Ready {
    pub plan: Plan,
    pub status: Status,
    /// The original PDF to store.
    pub pdf: Option<Vec<u8>>,
    pub warnings: Vec<Code>,
    /// `Some(true)`: an existing contact matches the counterparty.
    pub contact_exists: Option<bool>,
    pub related_found: bool,
}

#[derive(Debug, Clone)]
pub struct Analyzed {
    pub key: String,
    pub outcome: Result<Ready, Code>,
}

/// A plan, its original PDF and whether a too-large PDF was skipped.
type Planned = (Plan, Option<Vec<u8>>, bool);
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
    let plans: Vec<&Plan> = planned
        .iter()
        .filter_map(|(_, o)| o.as_ref().ok().map(|(p, _, _)| p))
        .collect();
    let mut seen = HashSet::new();
    let mut out = Vec::with_capacity(planned.len());
    for (key, outcome) in &planned {
        let outcome = match outcome {
            Err(code) => Err(*code),
            Ok((plan, pdf, skipped)) => {
                let in_batch = !seen.insert(lookup::identity(plan));
                let mut warnings = plan.warnings.clone();
                if *skipped {
                    warnings.push(PDF_SKIPPED);
                }
                if only.is_some_and(|o| !o.contains(key)) {
                    out.push(Analyzed {
                        key: key.clone(),
                        outcome: Ok(Ready {
                            plan: plan.clone(),
                            status: if in_batch {
                                Status::Duplicate
                            } else {
                                Status::Ok
                            },
                            pdf: None,
                            warnings,
                            contact_exists: None,
                            related_found: false,
                        }),
                    });
                    continue;
                }
                let duplicate = in_batch || lookup::duplicate(db, plan).await?;
                let contact_exists = match plan.counterparty() {
                    Some(p) => Some(lookup::contact(db, p).await?.is_some()),
                    None => None,
                };
                if contact_exists == Some(false) {
                    warnings.push(CONTACT_CREATED);
                }
                let related_found = plan.original_ref.is_some()
                    && (lookup::related(db, plan).await?.is_some()
                        || plans
                            .iter()
                            .any(|o| !std::ptr::eq(*o, plan) && lookup::is_original_of(plan, o)));
                if plan.original_ref.is_some() && !related_found {
                    warnings.push(RELATED_NOT_FOUND);
                }
                Ok(Ready {
                    plan: plan.clone(),
                    status: if duplicate {
                        Status::Duplicate
                    } else {
                        Status::Ok
                    },
                    pdf: pdf.clone(),
                    warnings,
                    contact_exists,
                    related_found,
                })
            }
        };
        out.push(Analyzed {
            key: key.clone(),
            outcome,
        });
    }
    Ok(out)
}
