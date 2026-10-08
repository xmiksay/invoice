//! Single-user static Bearer token auth.

use axum::extract::{Request, State};
use axum::http::{StatusCode, header};
use axum::middleware::Next;
use axum::response::Response;
use subtle::ConstantTimeEq;

use crate::app::AppState;
use crate::error::AppError;

/// Extract the token from an `Authorization: Bearer <token>` header value.
/// The scheme is matched case-insensitively (RFC 7235).
pub fn parse_bearer(value: &str) -> Option<&str> {
    let (scheme, token) = value.split_once(' ')?;
    let token = token.trim();
    (scheme.eq_ignore_ascii_case("bearer") && !token.is_empty()).then_some(token)
}

/// Constant-time token comparison. Only the length can leak, which is fine
/// for a random high-entropy token.
pub fn token_matches(expected: &str, provided: &str) -> bool {
    expected.as_bytes().ct_eq(provided.as_bytes()).into()
}

/// Rejects any request without the configured Bearer token with 401.
pub async fn require_token(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let authorized = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_bearer)
        .is_some_and(|t| token_matches(state.api_token.expose(), t));
    if !authorized {
        return Err(AppError::Unauthorized);
    }
    Ok(next.run(req).await)
}

/// Verify the Bearer token (used by the SPA login screen).
#[utoipa::path(
    get,
    path = "/api/auth/check",
    tag = "auth",
    security(("bearer" = [])),
    responses(
        (status = 204, description = "Token is valid"),
        (status = 401, description = "Missing or invalid token", body = crate::error::ErrorBody),
    )
)]
pub async fn check() -> StatusCode {
    StatusCode::NO_CONTENT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_compare() {
        assert!(token_matches("abc123", "abc123"));
        assert!(!token_matches("abc123", "abc124"));
        assert!(!token_matches("abc123", "abc12"));
        assert!(!token_matches("abc123", ""));
    }

    #[test]
    fn parses_bearer_header() {
        assert_eq!(parse_bearer("Bearer tok"), Some("tok"));
        assert_eq!(parse_bearer("bearer tok"), Some("tok"));
        assert_eq!(parse_bearer("Bearer  tok "), Some("tok"));
        assert_eq!(parse_bearer("Basic tok"), None);
        assert_eq!(parse_bearer("Bearer "), None);
        assert_eq!(parse_bearer("Bearer"), None);
        assert_eq!(parse_bearer("tok"), None);
    }
}
