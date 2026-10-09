//! `POST /api/import/isdoc/preview` and `…/confirm`.

use std::collections::{HashMap, HashSet};

use axum::Json;
use axum::extract::multipart::{MultipartError, MultipartRejection};
use axum::extract::{Multipart, State};
use axum::http::StatusCode;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::analyze::{self, Analyzed, Ready, Status};
use super::lookup;
use super::parse::Code;
use super::store::{self, DUPLICATE, NUMBER_TAKEN, Options};
use super::upload::File;
use crate::app::AppState;
use crate::cnb;
use crate::document::repo::issue::Rate;
use crate::error::{AppError, ErrorBody};
use crate::settings::entity::category;
use crate::time::today;

/// Largest accepted upload (all files together).
pub const MAX_UPLOAD: usize = 50 * 1024 * 1024;
/// Request body limit of the import routes: the files plus multipart overhead.
pub const BODY_LIMIT: usize = MAX_UPLOAD + 1024 * 1024;

#[derive(Debug, Serialize, ToSchema)]
pub struct Counterparty {
    pub name: String,
    pub ico: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PreviewEntry {
    pub key: String,
    /// `ok` | `duplicate` | `error`.
    pub status: &'static str,
    pub error: Option<&'static str>,
    pub warnings: Vec<&'static str>,
    pub direction: Option<&'static str>,
    pub doc_type: Option<&'static str>,
    pub number: Option<String>,
    pub counterparty: Option<Counterparty>,
    /// `existing` | `new`.
    pub contact_match: Option<&'static str>,
    pub issue_date: Option<NaiveDate>,
    pub tax_point_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub currency: Option<String>,
    pub total: Option<Decimal>,
    pub has_pdf: bool,
    pub related_number: Option<String>,
    pub related_found: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Preview {
    pub entries: Vec<PreviewEntry>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmResult {
    pub key: String,
    /// `imported` | `skipped` | `failed`.
    pub status: &'static str,
    pub document_id: Option<Uuid>,
    pub error: Option<&'static str>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Confirmed {
    pub results: Vec<ConfirmResult>,
}

/// The `options` part of `confirm` (a text field holding JSON).
#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct OptionsInput {
    pub selected: Vec<String>,
    /// Default `true`.
    pub mark_paid: Option<bool>,
    pub category_id: Option<Uuid>,
    /// Default `true`.
    pub vat_deductible: Option<bool>,
}

fn multipart_error(e: MultipartError) -> AppError {
    if e.status() == StatusCode::PAYLOAD_TOO_LARGE {
        AppError::TooLarge
    } else {
        AppError::BadRequest(e.body_text())
    }
}

/// The `files` parts (> [`MAX_UPLOAD`] together → `too_large`) and the raw
/// `options` part; other parts are ignored.
async fn read_form(
    form: Result<Multipart, MultipartRejection>,
) -> Result<(Vec<File>, Option<String>), AppError> {
    let mut form = form.map_err(|e| AppError::BadRequest(e.body_text()))?;
    let (mut files, mut options, mut total) = (Vec::new(), None, 0usize);
    while let Some(mut field) = form.next_field().await.map_err(multipart_error)? {
        match field.name() {
            Some("files") => {
                let name = field
                    .file_name()
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("file{}", files.len() + 1));
                let mut bytes = Vec::new();
                while let Some(chunk) = field.chunk().await.map_err(multipart_error)? {
                    total += chunk.len();
                    if total > MAX_UPLOAD {
                        return Err(AppError::TooLarge);
                    }
                    bytes.extend_from_slice(&chunk);
                }
                files.push(File { name, bytes });
            }
            Some("options") => options = Some(field.text().await.map_err(multipart_error)?),
            _ => {}
        }
    }
    if files.is_empty() {
        return Err(AppError::field("files", "required"));
    }
    Ok((files, options))
}

fn entry(a: &Analyzed) -> PreviewEntry {
    let mut e = PreviewEntry {
        key: a.key.clone(),
        status: "error",
        error: None,
        warnings: Vec::new(),
        direction: None,
        doc_type: None,
        number: None,
        counterparty: None,
        contact_match: None,
        issue_date: None,
        tax_point_date: None,
        due_date: None,
        currency: None,
        total: None,
        has_pdf: false,
        related_number: None,
        related_found: false,
    };
    let r = match &a.outcome {
        Err(code) => {
            e.error = Some(code);
            return e;
        }
        Ok(r) => r,
    };
    let p = &r.plan;
    e.status = match r.status {
        Status::Ok => "ok",
        Status::Duplicate => "duplicate",
    };
    e.warnings = r.warnings.clone();
    e.direction = Some(p.direction);
    e.doc_type = Some(p.doc_type.as_str());
    e.number = Some(p.number.clone());
    e.counterparty = p.counterparty().map(|c| Counterparty {
        name: c.name.clone(),
        ico: c.ico.clone(),
    });
    e.contact_match = r.contact_exists.map(|x| if x { "existing" } else { "new" });
    e.issue_date = Some(p.issue_date);
    e.tax_point_date = p.tax_point_date;
    e.due_date = p.due_date;
    e.currency = Some(p.currency.clone());
    e.total = Some(p.gross);
    e.has_pdf = r.pdf.is_some();
    e.related_number = p.original_ref.clone();
    e.related_found = r.related_found;
    e
}

#[utoipa::path(
    post,
    path = "/api/import/isdoc/preview",
    tag = "import",
    security(("bearer" = [])),
    request_body(content_type = "multipart/form-data", description = "`files` parts: `.isdoc`, `.isdocx` or `.zip` (≤ 50 MiB together)"),
    responses(
        (status = 200, body = Preview),
        (status = 413, description = "`too_large`", body = ErrorBody),
        (status = 422, description = "`files`: `required` / `too_many` / `too_large`", body = ErrorBody),
    )
)]
pub async fn preview(
    State(state): State<AppState>,
    form: Result<Multipart, MultipartRejection>,
) -> Result<Json<Preview>, AppError> {
    let (files, _) = read_form(form).await?;
    let analyzed = analyze::analyze(&state.db, files, None).await?;
    Ok(Json(Preview {
        entries: analyzed.iter().map(entry).collect(),
    }))
}

fn options(raw: Option<String>) -> Result<OptionsInput, AppError> {
    raw.and_then(|s| serde_json::from_str(&s).ok())
        .ok_or(AppError::field("options", "invalid"))
}

/// An expense category that can be assigned.
async fn check_category(state: &AppState, id: Option<Uuid>) -> Result<(), AppError> {
    let Some(id) = id else { return Ok(()) };
    match category::Entity::find_by_id(id).one(&state.db).await? {
        Some(c) if c.kind == "expense" && c.active => Ok(()),
        Some(c) if c.kind == "expense" => Err(AppError::field("categoryId", "inactive")),
        _ => Err(AppError::field("categoryId", "invalid")),
    }
}

/// The ISDOC rate (manual), else ČNB for the tax point (`rate_unavailable`).
async fn rate(state: &AppState, r: &Ready) -> Result<Rate, Code> {
    let p = &r.plan;
    if p.currency == "CZK" {
        return Ok(Rate {
            rate: None,
            date: None,
            source: None,
        });
    }
    if let Some(rate) = p.rate {
        return Ok(Rate {
            rate: Some(rate),
            date: None,
            source: Some("manual"),
        });
    }
    match cnb::repo::rate(&state.db, &state.cnb, &p.currency, p.rate_date(), today()).await {
        Ok(r) => Ok(Rate {
            rate: Some(r.rate),
            date: Some(r.date),
            source: Some("cnb"),
        }),
        Err(e) => {
            tracing::warn!(error = %e, "ISDOC import: no ČNB rate");
            Err("rate_unavailable")
        }
    }
}

async fn import_one(state: &AppState, r: &Ready, opts: &Options) -> Result<Uuid, Code> {
    let rate = rate(state, r).await?;
    store::import(
        &state.db,
        &state.pdf,
        &r.plan,
        r.pdf.as_deref(),
        &rate,
        opts,
    )
    .await
    .map_err(|e| match e {
        AppError::Conflict(m) if m == DUPLICATE => DUPLICATE,
        AppError::Conflict(m) if m == NUMBER_TAKEN => NUMBER_TAKEN,
        AppError::NumberTaken => NUMBER_TAKEN,
        other => {
            tracing::error!(error = %other, "ISDOC import failed");
            "internal"
        }
    })
}

#[utoipa::path(
    post,
    path = "/api/import/isdoc/confirm",
    tag = "import",
    security(("bearer" = [])),
    request_body(content_type = "multipart/form-data", description = "The same `files` as the preview plus an `options` text part holding JSON (`OptionsInput`)"),
    responses(
        (status = 200, body = Confirmed),
        (status = 413, description = "`too_large`", body = ErrorBody),
        (status = 422, description = "`files` / `options` / `categoryId`", body = ErrorBody),
    )
)]
pub async fn confirm(
    State(state): State<AppState>,
    form: Result<Multipart, MultipartRejection>,
) -> Result<Json<Confirmed>, AppError> {
    let (files, raw) = read_form(form).await?;
    let input = options(raw)?;
    check_category(&state, input.category_id).await?;
    let opts = Options {
        mark_paid: input.mark_paid.unwrap_or(true),
        category_id: input.category_id,
        vat_deductible: input.vat_deductible.unwrap_or(true),
    };
    let selected: HashSet<String> = input.selected.into_iter().collect();
    let analyzed = analyze::analyze(&state.db, files, Some(&selected)).await?;
    let mut todo: Vec<(&str, &Ready)> = analyzed
        .iter()
        .filter(|a| selected.contains(&a.key))
        .filter_map(|a| match &a.outcome {
            Ok(r) if r.status == Status::Ok => Some((a.key.as_str(), r)),
            _ => None,
        })
        .collect();
    todo.sort_by_key(|(_, r)| lookup::rank(r.plan.doc_type));
    let mut done: HashMap<&str, Result<Uuid, Code>> = HashMap::new();
    for (key, r) in todo {
        done.insert(key, import_one(&state, r, &opts).await);
    }
    let results = analyzed
        .iter()
        .map(|a| {
            let (status, document_id, error) = match done.get(a.key.as_str()) {
                Some(Ok(id)) => ("imported", Some(*id), None),
                Some(Err(code)) => ("failed", None, Some(*code)),
                None => ("skipped", None, None),
            };
            ConfirmResult {
                key: a.key.clone(),
                status,
                document_id,
                error,
            }
        })
        .collect();
    Ok(Json(Confirmed { results }))
}
