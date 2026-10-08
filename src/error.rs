//! Application error → client JSON `{ "code": ... }`.
//!
//! Internal and database errors are logged server-side and surface to the
//! client only as the generic `internal` code — never the underlying message.

use axum::Json;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("unauthorized")]
    Unauthorized,

    #[error("not found")]
    NotFound,

    #[error("internal error: {0:#}")]
    Internal(#[from] anyhow::Error),

    #[error("database error: {0}")]
    Database(#[from] sea_orm::DbErr),
}

/// Error body returned by every failing API call.
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    /// Machine-readable error code, e.g. `unauthorized`, `not_found`, `internal`.
    pub code: &'static str,
}

impl AppError {
    pub fn status_and_code(&self) -> (StatusCode, &'static str) {
        match self {
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            Self::Internal(_) | Self::Database(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal")
            }
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        if matches!(self, Self::Internal(_) | Self::Database(_)) {
            tracing::error!(error = %self, "request failed");
        }
        let (status, code) = self.status_and_code();
        let mut resp = (status, Json(ErrorBody { code })).into_response();
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

    #[test]
    fn maps_variants_to_status_and_code() {
        assert_eq!(
            AppError::Unauthorized.status_and_code(),
            (StatusCode::UNAUTHORIZED, "unauthorized")
        );
        assert_eq!(
            AppError::NotFound.status_and_code(),
            (StatusCode::NOT_FOUND, "not_found")
        );
        assert_eq!(
            AppError::from(anyhow::anyhow!("boom")).status_and_code(),
            (StatusCode::INTERNAL_SERVER_ERROR, "internal")
        );
        assert_eq!(
            AppError::from(sea_orm::DbErr::Custom("x".into())).status_and_code(),
            (StatusCode::INTERNAL_SERVER_ERROR, "internal")
        );
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
        let resp = AppError::from(anyhow::anyhow!("password=hunter2")).into_response();
        let body = axum::body::to_bytes(resp.into_body(), 1024)
            .await
            .expect("read body");
        assert_eq!(&body[..], br#"{"code":"internal"}"#);
    }
}
