//! `/api/members` and `POST /api/space/leave`.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::repo::{self, MemberRow};
use crate::app::AppState;
use crate::auth::handlers::login::with_cookie;
use crate::auth::{Authed, Manage, Read, session};
use crate::error::{AppError, ErrorBody};
use crate::extract::{ApiJson, ApiPath};
use crate::space::Role;

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Member {
    pub user_id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: Role,
    pub joined_at: DateTime<FixedOffset>,
    /// The caller's own row.
    pub is_self: bool,
}

fn item(m: MemberRow, me: Uuid) -> Member {
    Member {
        is_self: m.user_id == me,
        user_id: m.user_id,
        email: m.email,
        display_name: m.display_name,
        role: m.role,
        joined_at: m.joined_at,
    }
}

/// Members by role (owner, admin, member, accountant), then e-mail.
#[utoipa::path(
    get,
    path = "/api/members",
    tag = "members",
    security(("cookie" = []), ("bearer" = [])),
    responses(
        (status = 200, body = Vec<Member>),
        (status = 403, description = "`forbidden` below admin", body = ErrorBody),
    )
)]
pub async fn list(
    State(state): State<AppState>,
    access: Manage,
) -> Result<Json<Vec<Member>>, AppError> {
    let me = access.scope.user_id;
    let rows = repo::list(&state.db, access.space()).await?;
    Ok(Json(rows.into_iter().map(|m| item(m, me)).collect()))
}

#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(default)]
pub struct RoleInput {
    pub role: Option<String>,
}

#[utoipa::path(
    put,
    path = "/api/members/{userId}",
    tag = "members",
    security(("cookie" = []), ("bearer" = [])),
    params(("userId" = Uuid, Path)),
    request_body = RoleInput,
    responses(
        (status = 200, body = Member),
        (status = 403, description = "`forbidden`: below admin, or an owner the caller may not touch", body = ErrorBody),
        (status = 404, description = "Not a member of this space", body = ErrorBody),
        (status = 422, description = "`role: required | invalid | too_high | last_owner`", body = ErrorBody),
    )
)]
pub async fn update(
    State(state): State<AppState>,
    access: Manage,
    ApiPath(user_id): ApiPath<Uuid>,
    ApiJson(input): ApiJson<RoleInput>,
) -> Result<Json<Member>, AppError> {
    let s = access.scope;
    let role = Role::from_input(input.role.as_deref()).map_err(|r| AppError::field("role", r))?;
    let row = repo::change_role(&state.db, s.space, (s.user_id, s.role), user_id, role).await?;
    Ok(Json(item(row, s.user_id)))
}

#[utoipa::path(
    delete,
    path = "/api/members/{userId}",
    tag = "members",
    security(("cookie" = []), ("bearer" = [])),
    params(("userId" = Uuid, Path)),
    responses(
        (status = 204, description = "Removed; their sessions on this host and tokens in this space are deleted"),
        (status = 403, description = "`forbidden`: below admin, an owner the caller may not touch, or the caller (use leave)", body = ErrorBody),
        (status = 404, description = "Not a member of this space", body = ErrorBody),
        (status = 409, description = "`last_owner`", body = ErrorBody),
    )
)]
pub async fn remove(
    State(state): State<AppState>,
    access: Manage,
    ApiPath(user_id): ApiPath<Uuid>,
) -> Result<StatusCode, AppError> {
    let s = access.scope;
    if user_id == s.user_id {
        return Err(AppError::Forbidden);
    }
    repo::remove(&state.db, s.space, (s.user_id, s.role), user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Leave the space (session only): the membership, the caller's sessions on
/// this host and tokens in this space are deleted; the cookie is cleared.
#[utoipa::path(
    post,
    path = "/api/space/leave",
    tag = "members",
    security(("cookie" = [])),
    responses(
        (status = 204, description = "Left; `Set-Cookie` clears `invoice_session`"),
        (status = 403, description = "`forbidden` with a token", body = ErrorBody),
        (status = 409, description = "`last_owner`", body = ErrorBody),
    )
)]
pub async fn leave(
    State(state): State<AppState>,
    access: Read,
    authed: Authed,
) -> Result<Response, AppError> {
    authed.require_session()?;
    repo::leave(&state.db, access.space(), authed.user.id).await?;
    with_cookie(
        StatusCode::NO_CONTENT,
        &session::clear_cookie(state.public.https),
    )
}
