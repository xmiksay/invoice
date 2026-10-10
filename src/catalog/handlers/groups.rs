use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use uuid::Uuid;

use super::dto::{CatalogGroup, CatalogGroupInput, GroupMember, GroupQuery};
use crate::app::AppState;
use crate::auth::{Read, Write};
use crate::catalog::repo::groups::{self as repo, Full};
use crate::error::{AppError, ErrorBody};
use crate::extract::{ApiJson, ApiPath, ApiQuery};

impl From<Full> for CatalogGroup {
    fn from(f: Full) -> Self {
        Self {
            id: f.group.id,
            name: f.group.name,
            collapse: f.group.collapse,
            members: f
                .members
                .into_iter()
                .zip(1..)
                .map(|((m, item), position)| GroupMember {
                    item_id: m.item_id,
                    quantity: m.quantity.normalize(),
                    position,
                    item: item.into(),
                })
                .collect(),
            created_at: f.group.created_at,
            updated_at: f.group.updated_at,
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/catalog/groups",
    tag = "catalog",
    security(("cookie" = []), ("bearer" = [])),
    params(GroupQuery),
    responses((status = 200, body = Vec<CatalogGroup>))
)]
pub async fn list(
    State(state): State<AppState>,
    access: Read,
    ApiQuery(query): ApiQuery<GroupQuery>,
) -> Result<Json<Vec<CatalogGroup>>, AppError> {
    let q = query.q.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let rows = repo::list(&state.db, access.space(), q).await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    post,
    path = "/api/catalog/groups",
    tag = "catalog",
    security(("cookie" = []), ("bearer" = [])),
    request_body = CatalogGroupInput,
    responses(
        (status = 201, body = CatalogGroup),
        (status = 422, description = "Validation failed; `members`: `mixed_vat`", body = ErrorBody),
    )
)]
pub async fn create(
    State(state): State<AppState>,
    access: Write,
    ApiJson(input): ApiJson<CatalogGroupInput>,
) -> Result<(StatusCode, Json<CatalogGroup>), AppError> {
    let full = repo::create(&state.db, access.space(), input.validate()?).await?;
    Ok((StatusCode::CREATED, Json(full.into())))
}

#[utoipa::path(
    get,
    path = "/api/catalog/groups/{id}",
    tag = "catalog",
    security(("cookie" = []), ("bearer" = [])),
    params(("id" = Uuid, Path)),
    responses((status = 200, body = CatalogGroup), (status = 404, body = ErrorBody))
)]
pub async fn get(
    State(state): State<AppState>,
    access: Read,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<Json<CatalogGroup>, AppError> {
    Ok(Json(repo::get(&state.db, access.space(), id).await?.into()))
}

#[utoipa::path(
    put,
    path = "/api/catalog/groups/{id}",
    tag = "catalog",
    security(("cookie" = []), ("bearer" = [])),
    params(("id" = Uuid, Path)),
    request_body = CatalogGroupInput,
    responses(
        (status = 200, body = CatalogGroup),
        (status = 404, body = ErrorBody),
        (status = 422, description = "Validation failed; `members`: `mixed_vat`", body = ErrorBody),
    )
)]
pub async fn update(
    State(state): State<AppState>,
    access: Write,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(input): ApiJson<CatalogGroupInput>,
) -> Result<Json<CatalogGroup>, AppError> {
    let full = repo::update(&state.db, access.space(), id, input.validate()?).await?;
    Ok(Json(full.into()))
}

#[utoipa::path(
    delete,
    path = "/api/catalog/groups/{id}",
    tag = "catalog",
    security(("cookie" = []), ("bearer" = [])),
    params(("id" = Uuid, Path)),
    responses((status = 204), (status = 404, body = ErrorBody))
)]
pub async fn delete(
    State(state): State<AppState>,
    access: Write,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<StatusCode, AppError> {
    repo::delete(&state.db, access.space(), id).await?;
    Ok(StatusCode::NO_CONTENT)
}
