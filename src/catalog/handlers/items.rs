use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use uuid::Uuid;

use super::dto::{CatalogItem, CatalogItemInput, ItemQuery};
use crate::app::AppState;
use crate::auth::{Read, Write};
use crate::catalog::repo::items as repo;
use crate::error::{AppError, ErrorBody};
use crate::extract::{ApiJson, ApiPath, ApiQuery};

#[utoipa::path(
    get,
    path = "/api/catalog/items",
    tag = "catalog",
    security(("cookie" = []), ("bearer" = [])),
    params(ItemQuery),
    responses((status = 200, body = Vec<CatalogItem>))
)]
pub async fn list(
    State(state): State<AppState>,
    access: Read,
    ApiQuery(query): ApiQuery<ItemQuery>,
) -> Result<Json<Vec<CatalogItem>>, AppError> {
    let q = query.q.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let rows = repo::list(&state.db, access.space(), q, query.active).await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    post,
    path = "/api/catalog/items",
    tag = "catalog",
    security(("cookie" = []), ("bearer" = [])),
    request_body = CatalogItemInput,
    responses(
        (status = 201, body = CatalogItem),
        (status = 422, description = "Validation failed", body = ErrorBody),
    )
)]
pub async fn create(
    State(state): State<AppState>,
    access: Write,
    ApiJson(input): ApiJson<CatalogItemInput>,
) -> Result<(StatusCode, Json<CatalogItem>), AppError> {
    let row = repo::create(&state.db, access.space(), input.validate()?).await?;
    Ok((StatusCode::CREATED, Json(row.into())))
}

#[utoipa::path(
    get,
    path = "/api/catalog/items/{id}",
    tag = "catalog",
    security(("cookie" = []), ("bearer" = [])),
    params(("id" = Uuid, Path)),
    responses((status = 200, body = CatalogItem), (status = 404, body = ErrorBody))
)]
pub async fn get(
    State(state): State<AppState>,
    access: Read,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<Json<CatalogItem>, AppError> {
    Ok(Json(repo::get(&state.db, access.space(), id).await?.into()))
}

#[utoipa::path(
    put,
    path = "/api/catalog/items/{id}",
    tag = "catalog",
    security(("cookie" = []), ("bearer" = [])),
    params(("id" = Uuid, Path)),
    request_body = CatalogItemInput,
    responses(
        (status = 200, body = CatalogItem),
        (status = 404, body = ErrorBody),
        (status = 422, description = "Validation failed", body = ErrorBody),
    )
)]
pub async fn update(
    State(state): State<AppState>,
    access: Write,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(input): ApiJson<CatalogItemInput>,
) -> Result<Json<CatalogItem>, AppError> {
    Ok(Json(
        repo::update(&state.db, access.space(), id, input.validate()?)
            .await?
            .into(),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/catalog/items/{id}",
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
