//! Application error → client JSON `{ "code": ... }`.
//!
//! Internal and database errors are logged server-side and surface to the
//! client only as the generic `internal` code — never the underlying message.

use std::collections::BTreeMap;

use axum::Json;
use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use sea_orm::{DbErr, RuntimeErr, TransactionError, sqlx};
use serde::Serialize;
use utoipa::ToSchema;

/// Per-field validation failures: camelCase wire field name → reason code
/// (`required`, `invalid`, `too_long`, `duplicate`, `invalid_ico`, `invalid_pattern`,
/// `below_issued`, `exceeds_original`, `mixed_vat`, `unknown`, `inactive`).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FieldErrors(BTreeMap<String, &'static str>);

impl FieldErrors {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record `reason` for `field`; the first reason reported for a field wins.
    pub fn add(&mut self, field: &str, reason: &'static str) {
        self.0.entry(field.to_string()).or_insert(reason);
    }

    /// Record the outcome of a field check (`Err(reason)`), ignoring `Ok`.
    pub fn check<T>(&mut self, field: &str, result: Result<T, &'static str>) -> Option<T> {
        match result {
            Ok(v) => Some(v),
            Err(reason) => {
                self.add(field, reason);
                None
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The reason recorded for `field`.
    pub fn get(&self, field: &str) -> Option<&'static str> {
        self.0.get(field).copied()
    }

    /// Add every error of `other` (existing reasons win).
    pub fn merge(&mut self, other: FieldErrors) {
        for (field, reason) in other.0 {
            self.0.entry(field).or_insert(reason);
        }
    }

    /// `Ok(())` when nothing was recorded, else [`AppError::Validation`].
    pub fn into_result(self) -> Result<(), AppError> {
        if self.is_empty() {
            Ok(())
        } else {
            Err(AppError::Validation(self))
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("unauthorized")]
    Unauthorized,

    #[error("not found")]
    NotFound,

    #[error("malformed request: {0}")]
    BadRequest(String),

    #[error("validation failed: {0:?}")]
    Validation(FieldErrors),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("ARES subject not found")]
    AresNotFound,

    #[error("ARES unavailable: {0:#}")]
    AresUnavailable(anyhow::Error),

    #[error("document is locked (not a draft)")]
    DocumentLocked,

    #[error("action not allowed in the current document state")]
    InvalidState,

    #[error("ČNB unavailable: {0:#}")]
    CnbUnavailable(anyhow::Error),

    #[error("the advance is deducted by an issued invoice")]
    AdvanceSettled,

    #[error("the advance is deducted by a draft invoice")]
    AdvanceInUse,

    #[error("the catalog item is the last member of a group")]
    CatalogItemInUse,

    #[error("the category is used by a document")]
    CategoryInUse,

    /// The allocated number is already held by another (imported) document.
    #[error("document number already taken")]
    NumberTaken,

    /// Cancelling a debit note would leave its original over-credited.
    #[error("the credit notes would exceed the original")]
    ExceedsOriginal,

    #[error("the document has no PDF")]
    PdfMissing,

    #[error("upload too large")]
    TooLarge,

    /// mdcast unreachable, token rejected, or a gateway answering for it.
    #[error("PDF service unavailable: {0}")]
    PdfUnavailable(String),

    /// mdcast answered but the render failed; the message (typst diagnostics
    /// of the user's own template) is returned to the client as `detail`.
    #[error("PDF render failed: {0}")]
    PdfRenderFailed(String),

    /// mdcast failed in a way that is not the template's fault (its own
    /// 500, an undecodable answer…): 502 `pdf_render_failed` without
    /// `detail` — the upstream body is logged, never returned.
    #[error("PDF service error: {0}")]
    PdfUpstream(String),

    /// The file storage (directory or S3 bucket) failed; the cause is logged.
    #[error("storage unavailable: {0}")]
    StorageUnavailable(String),

    #[error("internal error: {0:#}")]
    Internal(#[from] anyhow::Error),

    #[error("database error: {0}")]
    Database(DbErr),
}

impl AppError {
    /// Single-field validation error.
    pub fn field(field: &str, reason: &'static str) -> Self {
        let mut errors = FieldErrors::new();
        errors.add(field, reason);
        Self::Validation(errors)
    }

    pub fn status_and_code(&self) -> (StatusCode, &'static str) {
        match self {
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            Self::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            Self::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, "validation"),
            Self::Conflict(_) => (StatusCode::CONFLICT, "conflict"),
            Self::AresNotFound => (StatusCode::NOT_FOUND, "ares_not_found"),
            Self::AresUnavailable(_) => (StatusCode::BAD_GATEWAY, "ares_unavailable"),
            Self::DocumentLocked => (StatusCode::CONFLICT, "document_locked"),
            Self::InvalidState => (StatusCode::CONFLICT, "invalid_state"),
            Self::CnbUnavailable(_) => (StatusCode::BAD_GATEWAY, "cnb_unavailable"),
            Self::AdvanceSettled => (StatusCode::CONFLICT, "advance_settled"),
            Self::AdvanceInUse => (StatusCode::CONFLICT, "advance_in_use"),
            Self::CatalogItemInUse => (StatusCode::CONFLICT, "catalog_item_in_use"),
            Self::CategoryInUse => (StatusCode::CONFLICT, "category_in_use"),
            Self::NumberTaken => (StatusCode::CONFLICT, "number_taken"),
            Self::ExceedsOriginal => (StatusCode::CONFLICT, "exceeds_original"),
            Self::PdfMissing => (StatusCode::NOT_FOUND, "pdf_missing"),
            Self::TooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "too_large"),
            Self::PdfUnavailable(_) => (StatusCode::SERVICE_UNAVAILABLE, "pdf_unavailable"),
            Self::StorageUnavailable(_) => (StatusCode::SERVICE_UNAVAILABLE, "storage_unavailable"),
            Self::PdfRenderFailed(_) | Self::PdfUpstream(_) => {
                (StatusCode::BAD_GATEWAY, "pdf_render_failed")
            }
            Self::Internal(_) | Self::Database(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal")
            }
        }
    }
}

/// The name of the unique constraint or index `err` violated, if it is a
/// unique violation (empty when the driver reports no name).
pub fn unique_violation(err: &DbErr) -> Option<String> {
    let (DbErr::Exec(RuntimeErr::SqlxError(sqlx::Error::Database(e)))
    | DbErr::Query(RuntimeErr::SqlxError(sqlx::Error::Database(e)))) = err
    else {
        return None;
    };
    e.is_unique_violation()
        .then(|| e.constraint().unwrap_or_default().to_string())
}

/// Maps a violation of the unique document-number indexes to `on_number`,
/// any other error as usual.
pub fn number_violation(err: DbErr, on_number: impl FnOnce() -> AppError) -> AppError {
    match unique_violation(&err).as_deref() {
        Some("documents_issued_number_key" | "documents_received_number_key") => on_number(),
        _ => err.into(),
    }
}

impl From<DbErr> for AppError {
    /// Unique violations nobody mapped to a field (typically a race on a
    /// partial unique index) are a client-visible 409, not a 500.
    fn from(err: DbErr) -> Self {
        match unique_violation(&err) {
            Some(msg) => Self::Conflict(msg),
            None => Self::Database(err),
        }
    }
}

/// For `db.transaction(|txn| …)`, which awaits the rollback when the closure
/// fails — a dropped `DatabaseTransaction` only queues it.
impl From<TransactionError<AppError>> for AppError {
    fn from(err: TransactionError<AppError>) -> Self {
        match err {
            TransactionError::Connection(e) => e.into(),
            TransactionError::Transaction(e) => e,
        }
    }
}

impl From<JsonRejection> for AppError {
    fn from(rejection: JsonRejection) -> Self {
        Self::BadRequest(rejection.body_text())
    }
}

impl From<QueryRejection> for AppError {
    fn from(rejection: QueryRejection) -> Self {
        Self::BadRequest(rejection.body_text())
    }
}

/// A path segment that does not parse (e.g. a non-UUID id) cannot name an
/// existing resource.
impl From<PathRejection> for AppError {
    fn from(_: PathRejection) -> Self {
        Self::NotFound
    }
}

/// Error body returned by every failing API call.
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    /// Machine-readable error code, e.g. `unauthorized`, `not_found`, `validation`, `internal`.
    pub code: &'static str,
    /// Only for `validation`: camelCase field name → reason code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<BTreeMap<String, String>>,
    /// Only for `pdf_render_failed`: the render service's message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match &self {
            Self::Internal(_) | Self::Database(_) => {
                tracing::error!(error = %self, "request failed")
            }
            Self::AresUnavailable(_) => tracing::warn!(error = %self, "ARES lookup failed"),
            Self::CnbUnavailable(_) => tracing::warn!(error = %self, "ČNB lookup failed"),
            Self::PdfUnavailable(_) | Self::PdfRenderFailed(_) | Self::PdfUpstream(_) => {
                tracing::warn!(error = %self, "PDF render failed")
            }
            Self::StorageUnavailable(_) => tracing::error!(error = %self, "storage failed"),
            Self::BadRequest(_) | Self::Conflict(_) => tracing::debug!(error = %self, "rejected"),
            _ => {}
        }
        let (status, code) = self.status_and_code();
        let fields = match &self {
            Self::Validation(f) => Some(
                f.0.iter()
                    .map(|(k, v)| (k.clone(), (*v).to_string()))
                    .collect(),
            ),
            _ => None,
        };
        let detail = match &self {
            Self::PdfRenderFailed(d) => Some(d.clone()),
            _ => None,
        };
        let mut resp = (
            status,
            Json(ErrorBody {
                code,
                fields,
                detail,
            }),
        )
            .into_response();
        if matches!(self, Self::Unauthorized) {
            resp.headers_mut()
                .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
        }
        resp
    }
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
