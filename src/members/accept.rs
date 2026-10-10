//! Accepting an invitation (space host, no auth): `GET` shows what the link
//! is for, `POST` signs in (existing account) or creates the account, then
//! joins the space and starts a session — one transaction.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::{Extension, Json};
use sea_orm::TransactionTrait;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{invites, repo};
use crate::app::AppState;
use crate::auth::entity::user;
use crate::auth::handlers::ClientIp;
use crate::auth::handlers::login::with_cookie;
use crate::auth::host::{ContextSpace, HostCtx};
use crate::auth::ratelimit::{INVITE_IP, LOGIN_EMAIL, LOGIN_IP};
use crate::auth::{crypto, session, users};
use crate::error::{AppError, ErrorBody, FieldErrors};
use crate::extract::{ApiJson, ApiQuery};
use crate::space::{Role, SpaceId};
use crate::validation as v;

#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(default)]
pub struct TokenQuery {
    pub token: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InviteInfo {
    pub space: ContextSpace,
    pub email: String,
    pub role: Role,
    /// `true`: accept with that account's password; `false`: a new account.
    pub account_exists: bool,
}

fn host_space(host: &HostCtx) -> Result<(SpaceId, ContextSpace), AppError> {
    let s = host.space.as_ref().ok_or(AppError::NotFound)?;
    Ok((
        SpaceId(s.id),
        ContextSpace {
            slug: s.slug.clone(),
            name: s.name.clone(),
        },
    ))
}

#[utoipa::path(
    get,
    path = "/api/invites/accept",
    tag = "members",
    params(("token" = String, Query)),
    responses(
        (status = 200, body = InviteInfo),
        (status = 404, description = "Invalid, expired or used token", body = ErrorBody),
        (status = 429, description = "`rate_limited` (per IP)", body = ErrorBody),
    )
)]
pub async fn show(
    State(state): State<AppState>,
    Extension(host): Extension<HostCtx>,
    ClientIp(ip): ClientIp,
    ApiQuery(q): ApiQuery<TokenQuery>,
) -> Result<Json<InviteInfo>, AppError> {
    state.limiter.reserve(&[(INVITE_IP, ip.as_str())])?;
    let (space, info) = host_space(&host)?;
    let invite = invites::by_token(&state.db, space, q.token.trim())
        .await?
        .ok_or(AppError::NotFound)?;
    let account_exists = users::find_by_email(&state.db, &invite.email)
        .await?
        .is_some();
    Ok(Json(InviteInfo {
        space: info,
        role: Role::parse(&invite.role).ok_or(AppError::NotFound)?,
        email: invite.email,
        account_exists,
    }))
}

#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct AcceptInput {
    pub token: String,
    pub password: String,
    /// New accounts only (1–100); ignored for an existing account.
    pub display_name: Option<String>,
}

/// An existing account proves itself with its password, counted in the
/// login buckets like a login (only failures keep their reservation).
async fn sign_in(
    state: &AppState,
    ip: &str,
    user: user::Model,
    password: String,
) -> Result<user::Model, AppError> {
    let buckets = [(LOGIN_EMAIL, user.email.as_str()), (LOGIN_IP, ip)];
    state.limiter.reserve(&buckets)?;
    let ok = crypto::verify_password(password, Some(user.password_hash.clone())).await?;
    if !ok || user.disabled {
        return Err(AppError::InvalidCredentials);
    }
    state.limiter.refund(&buckets);
    Ok(user)
}

/// The new account's name and password hash; 422 before any hashing.
async fn new_account(input: AcceptInput) -> Result<(String, String), AppError> {
    let mut e = FieldErrors::new();
    let name = e.check(
        "displayName",
        v::required_text(input.display_name.as_deref().unwrap_or_default(), 100),
    );
    e.check("password", crate::auth::handlers::password(&input.password));
    e.into_result()?;
    let hash = crypto::hash_password(input.password).await?;
    Ok((name.unwrap_or_default(), hash))
}

enum Who {
    Existing(uuid::Uuid),
    New { name: String, hash: String },
}

#[utoipa::path(
    post,
    path = "/api/invites/accept",
    tag = "members",
    request_body = AcceptInput,
    responses(
        (status = 204, description = "Joined; `Set-Cookie: invoice_session=…` for this host"),
        (status = 401, description = "`invalid_credentials` (existing account: wrong password, disabled)", body = ErrorBody),
        (status = 422, description = "`token: invalid`; new account: `displayName: required | too_long`, `password: too_short | too_long`", body = ErrorBody),
        (status = 429, description = "`rate_limited`", body = ErrorBody),
    )
)]
pub async fn accept(
    State(state): State<AppState>,
    Extension(host): Extension<HostCtx>,
    ClientIp(ip): ClientIp,
    headers: HeaderMap,
    ApiJson(input): ApiJson<AcceptInput>,
) -> Result<Response, AppError> {
    state.limiter.reserve(&[(INVITE_IP, ip.as_str())])?;
    let (space, _) = host_space(&host)?;
    let token = input.token.trim().to_string();
    // Checked first: argon2 runs only for a live invitation, never a guess.
    let invite = invites::by_token(&state.db, space, &token)
        .await?
        .ok_or_else(|| AppError::field("token", "invalid"))?;
    let who = match users::find_by_email(&state.db, &invite.email).await? {
        Some(u) => Who::Existing(sign_in(&state, &ip, u, input.password).await?.id),
        None => {
            let (name, hash) = new_account(input).await?;
            Who::New { name, hash }
        }
    };
    let ua = session::user_agent(&headers);
    let previous = session::cookie_value(&headers);
    let cookie = state
        .db
        .transaction::<_, Option<String>, AppError>(|txn| {
            Box::pin(async move {
                // Under the membership lock: a removal / demotion of the
                // inviter cannot interleave with the accept.
                repo::lock(txn, space).await?;
                let invite = invites::consume(txn, space, &token)
                    .await?
                    .ok_or_else(|| AppError::field("token", "invalid"))?;
                let role =
                    Role::parse(&invite.role).ok_or_else(|| AppError::field("token", "invalid"))?;
                if !repo::may_grant(txn, space, invite.invited_by, role).await? {
                    // Commit the consumed invitation: the link stays dead.
                    return Ok(None);
                }
                let user_id = match who {
                    Who::Existing(id) => id,
                    // A parallel registration of the address wins: 409.
                    Who::New { name, hash } => {
                        users::create(txn, &invite.email, &name, hash)
                            .await?
                            .ok_or_else(|| AppError::Conflict("users_email_key".into()))?
                            .id
                    }
                };
                // The link proved the address.
                users::mark_verified(txn, user_id).await?;
                repo::join(txn, space, user_id, &invite.role).await?;
                // Signed in as someone else on this host: that session ends.
                if let Some(old) = previous {
                    session::delete_by_cookie(txn, &old).await?;
                }
                session::create(txn, user_id, Some(space), ua)
                    .await
                    .map(Some)
            })
        })
        .await?
        .ok_or_else(|| AppError::field("token", "invalid"))?;
    with_cookie(
        StatusCode::NO_CONTENT,
        &session::set_cookie(&cookie, state.public.https),
    )
}
