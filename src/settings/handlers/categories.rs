use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::{Manage, Read};
use crate::error::{AppError, ErrorBody, FieldErrors};
use crate::extract::{ApiJson, ApiPath};
use crate::settings::entity::category;
use crate::settings::repo::categories as repo;
use crate::validation as v;

pub const KINDS: [&str; 2] = ["expense", "income"];

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: Uuid,
    pub name: String,
    /// `expense` (received documents) | `income` (issued documents).
    pub kind: String,
    pub active: bool,
    pub position: i32,
}

impl From<category::Model> for Category {
    fn from(m: category::Model) -> Self {
        Self {
            id: m.id,
            name: m.name,
            kind: m.kind,
            active: m.active,
            position: m.position,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct CategoryInput {
    /// Required, <= 100, unique per kind (case-insensitive).
    pub name: String,
    pub kind: Option<String>,
    /// Default `true`.
    pub active: Option<bool>,
    /// Default `0`.
    pub position: Option<i32>,
}

/// Validated [`CategoryInput`].
#[derive(Debug, Clone, PartialEq)]
pub struct CategoryData {
    pub name: String,
    pub kind: String,
    pub active: bool,
    pub position: i32,
}

impl CategoryInput {
    pub fn validate(self) -> Result<CategoryData, AppError> {
        let mut e = FieldErrors::new();
        let name = e.check("name", v::required_text(&self.name, 100));
        let kind = e.check(
            "kind",
            match self.kind.as_deref().map(str::trim) {
                None | Some("") => Err("required"),
                Some(k) if KINDS.contains(&k) => Ok(k.to_string()),
                Some(_) => Err("invalid"),
            },
        );
        e.into_result()?;
        Ok(CategoryData {
            name: name.unwrap_or_default(),
            kind: kind.unwrap_or_default(),
            active: self.active.unwrap_or(true),
            position: self.position.unwrap_or(0),
        })
    }
}

#[utoipa::path(
    get,
    path = "/api/settings/categories",
    tag = "settings",
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, description = "Ordered by kind, position, name", body = Vec<Category>))
)]
pub async fn list(
    State(state): State<AppState>,
    access: Read,
) -> Result<Json<Vec<Category>>, AppError> {
    let rows = repo::list(&state.db, access.space()).await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    post,
    path = "/api/settings/categories",
    tag = "settings",
    security(("cookie" = []), ("bearer" = [])),
    request_body = CategoryInput,
    responses(
        (status = 201, body = Category),
        (status = 422, description = "`name` (`required` / `too_long` / `duplicate`), `kind`", body = ErrorBody),
    )
)]
pub async fn create(
    State(state): State<AppState>,
    access: Manage,
    ApiJson(input): ApiJson<CategoryInput>,
) -> Result<(StatusCode, Json<Category>), AppError> {
    let row = repo::create(&state.db, access.space(), input.validate()?).await?;
    Ok((StatusCode::CREATED, Json(row.into())))
}

#[utoipa::path(
    put,
    path = "/api/settings/categories/{id}",
    tag = "settings",
    security(("cookie" = []), ("bearer" = [])),
    params(("id" = Uuid, Path)),
    request_body = CategoryInput,
    responses(
        (status = 200, body = Category),
        (status = 404, body = ErrorBody),
        (status = 422, description = "As create; `kind: invalid` when changed while documents use it", body = ErrorBody),
    )
)]
pub async fn update(
    State(state): State<AppState>,
    access: Manage,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(input): ApiJson<CategoryInput>,
) -> Result<Json<Category>, AppError> {
    let row = repo::update(&state.db, access.space(), id, input.validate()?).await?;
    Ok(Json(row.into()))
}

#[utoipa::path(
    delete,
    path = "/api/settings/categories/{id}",
    tag = "settings",
    security(("cookie" = []), ("bearer" = [])),
    params(("id" = Uuid, Path)),
    responses(
        (status = 204),
        (status = 404, body = ErrorBody),
        (status = 409, description = "`category_in_use` (deactivate it instead)", body = ErrorBody),
    )
)]
pub async fn delete(
    State(state): State<AppState>,
    access: Manage,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<StatusCode, AppError> {
    repo::delete(&state.db, access.space(), id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation() {
        let ok = CategoryInput {
            name: " Software ".into(),
            kind: Some("expense".into()),
            ..Default::default()
        }
        .validate()
        .expect("valid");
        assert_eq!(ok.name, "Software");
        assert!(ok.active);
        assert_eq!(ok.position, 0);
        let Err(AppError::Validation(e)) = CategoryInput {
            name: "x".repeat(101),
            kind: Some("cost".into()),
            ..Default::default()
        }
        .validate() else {
            panic!("expected validation error");
        };
        assert_eq!(e.get("name"), Some("too_long"));
        assert_eq!(e.get("kind"), Some("invalid"));
        let Err(AppError::Validation(e)) = CategoryInput::default().validate() else {
            panic!("expected validation error");
        };
        assert_eq!(e.get("name"), Some("required"));
        assert_eq!(e.get("kind"), Some("required"));
    }
}
