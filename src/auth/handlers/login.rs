//! Login, logout, the current user, revoking other sessions.

use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::ClientIp;
use crate::app::AppState;
use crate::auth::ctx::Authed;
use crate::auth::host::HostCtx;
use crate::auth::ratelimit::{LOGIN_EMAIL, LOGIN_IP};
use crate::auth::{crypto, session, users};
use crate::error::{AppError, ErrorBody};
use crate::extract::ApiJson;
use crate::space::{Role, repo as space_repo};

#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(default)]
pub struct LoginInput {
    pub email: String,
    pub password: String,
}

pub(crate) fn with_cookie(status: StatusCode, cookie: &str) -> Result<Response, AppError> {
    let mut resp = status.into_response();
    let value = HeaderValue::from_str(cookie)
        .map_err(|_| anyhow::anyhow!("session cookie is not a valid header value"))?;
    resp.headers_mut().insert(header::SET_COOKIE, value);
    Ok(resp)
}

/// Start a session on this host. Every failure is the same 401.
#[utoipa::path(
    post,
    path = "/api/auth/login",
    tag = "auth",
    request_body = LoginInput,
    responses(
        (status = 204, description = "Logged in; `Set-Cookie: invoice_session=…`"),
        (status = 401, description = "`invalid_credentials`", body = ErrorBody),
        (status = 429, description = "`rate_limited` (+ `Retry-After`)", body = ErrorBody),
    )
)]
pub async fn login(
    State(state): State<AppState>,
    Extension(host): Extension<HostCtx>,
    ClientIp(ip): ClientIp,
    headers: HeaderMap,
    ApiJson(input): ApiJson<LoginInput>,
) -> Result<Response, AppError> {
    let email = users::normalize_email(&input.email);
    // Reserved before the slow part, so a parallel burst cannot pass the
    // limit; only failures keep their reservation.
    let buckets = [(LOGIN_EMAIL, email.as_str()), (LOGIN_IP, ip.as_str())];
    state.limiter.reserve(&buckets)?;
    let user = if email.is_empty() {
        None
    } else {
        users::find_by_email(&state.db, &email).await?
    };
    let ok = crypto::verify_password(
        input.password,
        user.as_ref().map(|u| u.password_hash.clone()),
    )
    .await?;
    let space = host.space_id();
    let allowed = match (&user, space) {
        (Some(u), _) if !ok || u.disabled => false,
        (Some(_), None) => true,
        (Some(u), Some(sp)) => {
            u.email_verified_at.is_some()
                && space_repo::member_role(&state.db, sp, u.id)
                    .await?
                    .is_some()
        }
        (None, _) => false,
    };
    let Some(user) = user.filter(|_| allowed) else {
        return Err(AppError::InvalidCredentials);
    };
    state.limiter.refund(&buckets);
    let value = session::create(&state.db, user.id, space, session::user_agent(&headers)).await?;
    with_cookie(
        StatusCode::NO_CONTENT,
        &session::set_cookie(&value, state.public.https),
    )
}

/// End this session (a token request has none: 204 all the same).
#[utoipa::path(
    post,
    path = "/api/auth/logout",
    tag = "auth",
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 204, description = "Session deleted, cookie cleared"))
)]
pub async fn logout(State(state): State<AppState>, authed: Authed) -> Result<Response, AppError> {
    if let Ok(id) = authed.require_session() {
        session::delete(&state.db, id).await?;
    }
    with_cookie(
        StatusCode::NO_CONTENT,
        &session::clear_cookie(state.public.https),
    )
}

/// Delete every other session of the user, on all hosts (session only).
#[utoipa::path(
    post,
    path = "/api/auth/sessions/revoke-others",
    tag = "auth",
    security(("cookie" = [])),
    responses((status = 204), (status = 403, description = "`forbidden` with a token", body = ErrorBody))
)]
pub async fn revoke_others(
    State(state): State<AppState>,
    authed: Authed,
) -> Result<StatusCode, AppError> {
    let current = authed.require_session()?;
    session::delete_others(&state.db, authed.user.id, Some(current)).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MeUser {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub email_verified: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MeSpace {
    pub slug: String,
    pub name: String,
    pub role: Role,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Me {
    pub user: MeUser,
    pub space: Option<MeSpace>,
}

#[utoipa::path(
    get,
    path = "/api/auth/me",
    tag = "auth",
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = Me), (status = 401, body = ErrorBody))
)]
pub async fn me(authed: Authed) -> Json<Me> {
    Json(Me {
        user: MeUser {
            id: authed.user.id,
            email: authed.user.email,
            display_name: authed.user.display_name,
            email_verified: authed.user.email_verified_at.is_some(),
        },
        space: authed.space.map(|m| MeSpace {
            slug: m.slug,
            name: m.name,
            role: m.role,
        }),
    })
}
