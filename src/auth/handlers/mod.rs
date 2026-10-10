//! `/api/auth/*` and `/api/account/*` handlers. Contract: `docs/api/auth.md`.

pub mod login;
pub mod password;
pub mod register;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::app::AppState;
use crate::error::AppError;
use crate::validation as v;

pub const PASSWORD_MIN: usize = 12;
pub const PASSWORD_MAX: usize = 200;

/// Password rule: 12–200 characters, nothing else.
pub fn password(raw: &str) -> Result<(), &'static str> {
    match raw.chars().count() {
        n if n < PASSWORD_MIN => Err("too_short"),
        n if n > PASSWORD_MAX => Err("too_long"),
        _ => Ok(()),
    }
}

/// A required e-mail address: `required` | `invalid`.
pub fn email(raw: &str) -> Result<String, &'static str> {
    match v::opt_email(Some(raw)) {
        Ok(Some(e)) => Ok(crate::auth::users::normalize_email(&e)),
        Ok(None) => Err("required"),
        Err(_) => Err("invalid"),
    }
}

/// Verify the signed-in `user`'s own password (password change, space
/// delete). Failures count in the user's login bucket (5 / 15 min, shared
/// with login), so the check cannot be used to guess; 429 when used up.
pub async fn check_password(
    state: &AppState,
    user: &crate::auth::entity::user::Model,
    password: String,
) -> Result<bool, AppError> {
    let bucket = [(crate::auth::ratelimit::LOGIN_EMAIL, user.email.as_str())];
    state.limiter.reserve(&bucket)?;
    let ok =
        crate::auth::crypto::verify_password(password, Some(user.password_hash.clone())).await?;
    if ok {
        state.limiter.refund(&bucket);
    }
    Ok(ok)
}

/// The client IP for rate limits (see `ratelimit::client_ip`).
pub struct ClientIp(pub String);

impl FromRequestParts<AppState> for ClientIp {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        Ok(Self(crate::auth::ratelimit::client_ip(
            parts,
            state.trust_forwarded,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_rule() {
        assert_eq!(password(""), Err("too_short"));
        assert_eq!(password(&"x".repeat(11)), Err("too_short"));
        assert_eq!(password(&"ž".repeat(12)), Ok(()));
        assert_eq!(password(&"x".repeat(200)), Ok(()));
        assert_eq!(password(&"x".repeat(201)), Err("too_long"));
    }

    #[test]
    fn email_rule() {
        assert_eq!(email(" Jan@Example.com "), Ok("jan@example.com".into()));
        assert_eq!(email(""), Err("required"));
        assert_eq!(email("nope"), Err("invalid"));
        assert_eq!(
            email(&format!("{}@example.com", "a".repeat(200))),
            Err("invalid")
        );
    }
}
