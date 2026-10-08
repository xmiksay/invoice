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
/// (`required`, `invalid`, `too_long`, `duplicate`, `invalid_ico`, `invalid_pattern`).
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
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match &self {
            Self::Internal(_) | Self::Database(_) => {
                tracing::error!(error = %self, "request failed")
            }
            Self::AresUnavailable(_) => tracing::warn!(error = %self, "ARES lookup failed"),
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
        let mut resp = (status, Json(ErrorBody { code, fields })).into_response();
        if matches!(self, Self::Unauthorized) {
            resp.headers_mut()
                .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
        }
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn body(err: AppError) -> (StatusCode, serde_json::Value) {
        let resp = err.into_response();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), 4096)
            .await
            .expect("read body");
        (status, serde_json::from_slice(&bytes).expect("json"))
    }

    #[test]
    fn maps_variants_to_status_and_code() {
        let cases = [
            (
                AppError::Unauthorized,
                StatusCode::UNAUTHORIZED,
                "unauthorized",
            ),
            (AppError::NotFound, StatusCode::NOT_FOUND, "not_found"),
            (
                AppError::BadRequest("x".into()),
                StatusCode::BAD_REQUEST,
                "bad_request",
            ),
            (
                AppError::field("name", "required"),
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation",
            ),
            (
                AppError::Conflict("x".into()),
                StatusCode::CONFLICT,
                "conflict",
            ),
            (
                AppError::AresNotFound,
                StatusCode::NOT_FOUND,
                "ares_not_found",
            ),
            (
                AppError::AresUnavailable(anyhow::anyhow!("timeout")),
                StatusCode::BAD_GATEWAY,
                "ares_unavailable",
            ),
            (
                AppError::from(anyhow::anyhow!("boom")),
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
            ),
            (
                AppError::from(DbErr::Custom("x".into())),
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
            ),
        ];
        for (err, status, code) in cases {
            assert_eq!(err.status_and_code(), (status, code), "{err}");
        }
    }

    #[test]
    fn unauthorized_sets_www_authenticate() {
        let resp = AppError::Unauthorized.into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            resp.headers().get(header::WWW_AUTHENTICATE),
            Some(&HeaderValue::from_static("Bearer"))
        );
    }

    #[tokio::test]
    async fn internal_error_does_not_leak_detail() {
        let (_, json) = body(AppError::from(anyhow::anyhow!("password=hunter2"))).await;
        assert_eq!(json, serde_json::json!({ "code": "internal" }));
        let (_, json) = body(AppError::AresUnavailable(anyhow::anyhow!("dns fail"))).await;
        assert_eq!(json, serde_json::json!({ "code": "ares_unavailable" }));
        let (_, json) = body(AppError::Conflict("contacts_pkey".into())).await;
        assert_eq!(json, serde_json::json!({ "code": "conflict" }));
    }

    #[tokio::test]
    async fn validation_lists_fields() {
        let mut errors = FieldErrors::new();
        errors.add("name", "required");
        errors.add("name", "too_long");
        errors.add("ico", "invalid_ico");
        let (status, json) = body(AppError::Validation(errors)).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            json,
            serde_json::json!({
                "code": "validation",
                "fields": { "ico": "invalid_ico", "name": "required" }
            })
        );
    }

    #[test]
    fn field_errors_check_and_result() {
        let mut errors = FieldErrors::new();
        assert_eq!(errors.check("a", Ok::<_, &'static str>(1)), Some(1));
        assert!(errors.clone().into_result().is_ok());
        assert_eq!(errors.check::<()>("b", Err("invalid")), None);
        assert!(matches!(errors.into_result(), Err(AppError::Validation(_))));
    }
}
