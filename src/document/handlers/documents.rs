use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use uuid::Uuid;

use super::compute_input::{ComputeCtx, ComputeInput};
use super::dto::{Computed, Document, DocumentList, ListQuery, lines_out};
use super::fetch;
use super::input::DocumentInput;
use crate::app::AppState;
use crate::contact::handlers::dto::ListQuery as Paging;
use crate::document::line::Status;
use crate::document::repo::{advance_sources, context, query, view, write};
use crate::error::{AppError, ErrorBody};
use crate::extract::{ApiJson, ApiPath, ApiQuery};
use crate::settings::repo::company;
use crate::time::today;

#[utoipa::path(
    get,
    path = "/api/documents",
    tag = "documents",
    security(("bearer" = [])),
    params(ListQuery),
    responses((status = 200, body = DocumentList))
)]
pub async fn list(
    State(state): State<AppState>,
    ApiQuery(query): ApiQuery<ListQuery>,
) -> Result<Json<DocumentList>, AppError> {
    let (q, limit, offset) = Paging {
        q: query.q.clone(),
        limit: query.limit,
        offset: query.offset,
    }
    .normalized();
    let today = today();
    let (docs, total) = query::list(&state.db, &query, q.as_deref(), limit, offset, today).await?;
    Ok(Json(DocumentList {
        items: view::summaries(&state.db, docs, today).await?,
        total,
    }))
}

#[utoipa::path(
    post,
    path = "/api/documents",
    tag = "documents",
    security(("bearer" = [])),
    request_body = DocumentInput,
    responses(
        (status = 201, body = Document),
        (status = 422, description = "Validation failed", body = ErrorBody),
    )
)]
pub async fn create(
    State(state): State<AppState>,
    ApiJson(input): ApiJson<DocumentInput>,
) -> Result<(StatusCode, Json<Document>), AppError> {
    let ids = input.advance_ids();
    let ctx = context::load(&state.db, input.contact_id, today(), None, &ids).await?;
    let (mut data, evaluated) = input.validate(&ctx)?;
    context::resolve_bank(&state.db, &mut data, true).await?;
    let id = write::create(&state.db, data, evaluated.totals).await?;
    Ok((StatusCode::CREATED, Json(fetch(&state, id).await?)))
}

#[utoipa::path(
    get,
    path = "/api/documents/{id}",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    responses((status = 200, body = Document), (status = 404, body = ErrorBody))
)]
pub async fn get(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<Json<Document>, AppError> {
    Ok(Json(fetch(&state, id).await?))
}

#[utoipa::path(
    put,
    path = "/api/documents/{id}",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    request_body = DocumentInput,
    responses(
        (status = 200, body = Document),
        (status = 404, body = ErrorBody),
        (status = 409, description = "`document_locked` (not a draft)", body = ErrorBody),
        (status = 422, description = "Validation failed", body = ErrorBody),
    )
)]
pub async fn update(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(input): ApiJson<DocumentInput>,
) -> Result<Json<Document>, AppError> {
    let doc = query::find(&state.db, id).await?;
    if view::status(&doc)? != Status::Draft {
        return Err(AppError::DocumentLocked);
    }
    let existing = Some(context::existing(&doc)?);
    let ids = input.advance_ids();
    let ctx = context::load(&state.db, input.contact_id, today(), existing, &ids).await?;
    let (mut data, evaluated) = input.validate(&ctx)?;
    context::resolve_bank(&state.db, &mut data, false).await?;
    write::update(&state.db, id, data, evaluated.totals).await?;
    Ok(Json(fetch(&state, id).await?))
}

#[utoipa::path(
    delete,
    path = "/api/documents/{id}",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = Uuid, Path)),
    responses(
        (status = 204),
        (status = 404, body = ErrorBody),
        (status = 409, description = "`document_locked` (not a draft)", body = ErrorBody),
    )
)]
pub async fn delete(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<StatusCode, AppError> {
    write::delete(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/documents/compute",
    tag = "documents",
    security(("bearer" = [])),
    request_body = ComputeInput,
    responses(
        (status = 200, body = Computed),
        (status = 422, description = "Same rules as save", body = ErrorBody),
    )
)]
pub async fn compute(
    State(state): State<AppState>,
    ApiJson(input): ApiJson<ComputeInput>,
) -> Result<Json<Computed>, AppError> {
    let company = company::get(&state.db).await?;
    let existing = match input.document_id {
        Some(id) => match query::find(&state.db, id).await {
            Ok(doc) => Some(context::existing(&doc)?),
            Err(AppError::NotFound) => None,
            Err(e) => return Err(e),
        },
        None => None,
    };
    let ctx = ComputeCtx {
        vat_payer: company.vat_payer,
        default_rate: context::default_vat_rate(&state.db).await?,
        default_locale: company.default_locale,
        existing,
        advances: advance_sources::load(&state.db, &input.advance_ids()).await?,
    };
    let (lines, evaluated) = input.validate(&ctx)?;
    Ok(Json(Computed {
        lines: lines_out(&lines, &evaluated.lines),
        totals: evaluated.totals.into(),
    }))
}
