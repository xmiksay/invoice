//! `/api/spaces` (base host: my spaces, create) and `/api/space` (space
//! host: the current space, rename, delete).

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::entity::space;
use super::{Role, repo, slug};
use crate::app::AppState;
use crate::auth::handlers::check_password;
use crate::auth::{Authed, Manage, Own, Read};
use crate::error::{AppError, ErrorBody, FieldErrors};
use crate::extract::ApiJson;
use crate::validation as v;

#[derive(Debug, Serialize, ToSchema)]
pub struct SpaceInfo {
    pub slug: String,
    pub name: String,
    pub role: Role,
    /// `{scheme}://{slug}.{base host}{:port}`.
    pub url: String,
}

fn info(state: &AppState, s: space::Model, role: Role) -> SpaceInfo {
    SpaceInfo {
        url: state.public.space_url(&s.slug),
        slug: s.slug,
        name: s.name,
        role,
    }
}

#[utoipa::path(
    get,
    path = "/api/spaces",
    tag = "spaces",
    security(("cookie" = [])),
    responses((status = 200, body = Vec<SpaceInfo>), (status = 401, body = ErrorBody))
)]
pub async fn list(
    State(state): State<AppState>,
    authed: Authed,
) -> Result<Json<Vec<SpaceInfo>>, AppError> {
    let rows = repo::list_for_user(&state.db, authed.user.id).await?;
    Ok(Json(
        rows.into_iter()
            .map(|(s, role)| info(&state, s, role))
            .collect(),
    ))
}

#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(default)]
pub struct CreateInput {
    pub slug: String,
    pub name: String,
}

#[utoipa::path(
    post,
    path = "/api/spaces",
    tag = "spaces",
    security(("cookie" = [])),
    request_body = CreateInput,
    responses(
        (status = 201, body = SpaceInfo),
        (status = 403, description = "`email_unverified`", body = ErrorBody),
        (status = 422, description = "`slug: required | invalid | reserved | taken`, `name: required | too_long`", body = ErrorBody),
    )
)]
pub async fn create(
    State(state): State<AppState>,
    authed: Authed,
    ApiJson(input): ApiJson<CreateInput>,
) -> Result<(StatusCode, Json<SpaceInfo>), AppError> {
    if authed.user.email_verified_at.is_none() {
        return Err(AppError::EmailUnverified);
    }
    let mut e = FieldErrors::new();
    let slug = e.check("slug", slug::validate(&input.slug));
    let name = e.check("name", v::required_text(&input.name, 200));
    e.into_result()?;
    let row = repo::create(
        &state.db,
        slug.unwrap_or_default(),
        name.unwrap_or_default(),
        authed.user.id,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(info(&state, row, Role::Owner))))
}

async fn current(state: &AppState, access_space: super::SpaceId) -> Result<space::Model, AppError> {
    repo::find(&state.db, access_space)
        .await?
        .ok_or(AppError::NotFound)
}

#[utoipa::path(
    get,
    path = "/api/space",
    tag = "spaces",
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = SpaceInfo))
)]
pub async fn get(State(state): State<AppState>, access: Read) -> Result<Json<SpaceInfo>, AppError> {
    let row = current(&state, access.space()).await?;
    Ok(Json(info(&state, row, access.scope.role)))
}

#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(default)]
pub struct RenameInput {
    pub name: String,
}

#[utoipa::path(
    put,
    path = "/api/space",
    tag = "spaces",
    security(("cookie" = []), ("bearer" = [])),
    request_body = RenameInput,
    responses(
        (status = 200, body = SpaceInfo),
        (status = 403, description = "`forbidden` below admin", body = ErrorBody),
        (status = 422, description = "`name: required | too_long`", body = ErrorBody),
    )
)]
pub async fn rename(
    State(state): State<AppState>,
    access: Manage,
    ApiJson(input): ApiJson<RenameInput>,
) -> Result<Json<SpaceInfo>, AppError> {
    let name = v::required_text(&input.name, 200).map_err(|r| AppError::field("name", r))?;
    let row = repo::rename(&state.db, access.space(), name).await?;
    Ok(Json(info(&state, row, access.scope.role)))
}

#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(default)]
pub struct DeleteInput {
    pub slug: String,
    pub password: String,
}

#[utoipa::path(
    delete,
    path = "/api/space",
    tag = "spaces",
    security(("cookie" = [])),
    request_body = DeleteInput,
    responses(
        (status = 204, description = "The space, all its data and files are deleted"),
        (status = 403, description = "`forbidden`: not the owner, or a token", body = ErrorBody),
        (status = 422, description = "`slug: mismatch`, `password: invalid`", body = ErrorBody),
    )
)]
pub async fn delete(
    State(state): State<AppState>,
    access: Own,
    authed: Authed,
    ApiJson(input): ApiJson<DeleteInput>,
) -> Result<StatusCode, AppError> {
    authed.require_session()?;
    let space = current(&state, access.space()).await?;
    let mut e = FieldErrors::new();
    if input.slug.trim() != space.slug {
        e.add("slug", "mismatch");
    }
    if !check_password(&state, &authed.user, input.password).await? {
        e.add("password", "invalid");
    }
    e.into_result()?;
    let space = access.space();
    repo::delete(&state.db, space).await?;
    // Detached from the request: a client that disconnects after the commit
    // must not leave the files behind.
    let storage = state.pdf.root_storage().clone();
    tokio::spawn(async move {
        let prefix = space.storage_prefix();
        match storage.delete_prefix(&prefix).await {
            Ok(n) => tracing::info!(space = %space, files = n, "space deleted"),
            Err(e) => tracing::error!(
                space = %space,
                error = %e,
                "space deleted, but its files were not removed (orphaned under {prefix}/)"
            ),
        }
    });
    Ok(StatusCode::NO_CONTENT)
}
