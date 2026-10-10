use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::{Manage, Read};
use crate::document::custom_fields::FieldType;
use crate::error::{AppError, ErrorBody, FieldErrors};
use crate::extract::{ApiJson, ApiPath};
use crate::settings::entity::custom_field;
use crate::settings::repo::custom_fields as repo;
use crate::validation as v;

pub const APPLIES_TO: [&str; 3] = ["issued", "received", "both"];
pub const MAX_OPTIONS: usize = 50;

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustomField {
    pub id: Uuid,
    pub key: String,
    pub label: String,
    /// `text` | `number` | `date` | `bool` | `select`.
    #[serde(rename = "type")]
    pub field_type: String,
    pub options: Vec<String>,
    /// `issued` | `received` | `both`.
    pub applies_to: String,
    pub required: bool,
    pub active: bool,
    pub position: i32,
}

impl From<custom_field::Model> for CustomField {
    fn from(m: custom_field::Model) -> Self {
        Self {
            id: m.id,
            key: m.key,
            label: m.label,
            field_type: m.field_type,
            options: m.options,
            applies_to: m.applies_to,
            required: m.required,
            active: m.active,
            position: m.position,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct CustomFieldInput {
    /// `^[a-z][a-z0-9_]{0,39}$`, unique; immutable.
    pub key: String,
    /// Required, <= 100.
    pub label: String,
    /// Immutable.
    #[serde(rename = "type")]
    pub field_type: Option<String>,
    /// `select` only: 1..50 unique options, each <= 100.
    pub options: Vec<String>,
    /// Default `both`.
    pub applies_to: Option<String>,
    pub required: bool,
    /// Default `true`.
    pub active: Option<bool>,
    /// Default `0`.
    pub position: Option<i32>,
}

/// Validated [`CustomFieldInput`].
#[derive(Debug, Clone, PartialEq)]
pub struct CustomFieldData {
    pub key: String,
    pub label: String,
    pub field_type: FieldType,
    pub options: Vec<String>,
    pub applies_to: String,
    pub required: bool,
    pub active: bool,
    pub position: i32,
}

pub fn is_valid_key(k: &str) -> bool {
    let mut chars = k.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && k.len() <= 40
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn options(field_type: Option<FieldType>, raw: &[String], e: &mut FieldErrors) -> Vec<String> {
    if field_type != Some(FieldType::Select) {
        if !raw.is_empty() {
            e.add("options", "invalid");
        }
        return Vec::new();
    }
    if raw.is_empty() {
        e.add("options", "required");
    } else if raw.len() > MAX_OPTIONS {
        e.add("options", "too_long");
    }
    let mut out: Vec<String> = Vec::with_capacity(raw.len());
    for (i, o) in raw.iter().enumerate() {
        let field = format!("options.{i}");
        match v::required_text(o, 100) {
            Ok(o) if out.contains(&o) => e.add(&field, "duplicate"),
            Ok(o) => out.push(o),
            Err(r) => e.add(&field, r),
        }
    }
    out
}

impl CustomFieldInput {
    pub fn validate(self) -> Result<CustomFieldData, AppError> {
        let mut e = FieldErrors::new();
        let key = self.key.trim().to_string();
        if key.is_empty() {
            e.add("key", "required");
        } else if !is_valid_key(&key) {
            e.add("key", "invalid");
        }
        let label = e.check("label", v::required_text(&self.label, 100));
        let field_type = e.check(
            "type",
            match self.field_type.as_deref().map(str::trim) {
                None | Some("") => Err("required"),
                Some(t) => FieldType::parse(t).ok_or("invalid"),
            },
        );
        let options = options(field_type, &self.options, &mut e);
        let applies_to = e.check(
            "appliesTo",
            match self.applies_to.as_deref().map(str::trim) {
                None | Some("") => Ok("both".to_string()),
                Some(a) if APPLIES_TO.contains(&a) => Ok(a.to_string()),
                Some(_) => Err("invalid"),
            },
        );
        e.into_result()?;
        Ok(CustomFieldData {
            key,
            label: label.unwrap_or_default(),
            field_type: field_type.unwrap_or(FieldType::Text),
            options,
            applies_to: applies_to.unwrap_or_default(),
            required: self.required,
            active: self.active.unwrap_or(true),
            position: self.position.unwrap_or(0),
        })
    }
}

#[utoipa::path(
    get,
    path = "/api/settings/custom-fields",
    tag = "settings",
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, description = "Ordered by position, key", body = Vec<CustomField>))
)]
pub async fn list(
    State(state): State<AppState>,
    access: Read,
) -> Result<Json<Vec<CustomField>>, AppError> {
    let rows = repo::list(&state.db, access.space()).await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

#[utoipa::path(
    post,
    path = "/api/settings/custom-fields",
    tag = "settings",
    security(("cookie" = []), ("bearer" = [])),
    request_body = CustomFieldInput,
    responses(
        (status = 201, body = CustomField),
        (status = 422, description = "`key` (`invalid` / `duplicate`), `label`, `type`, `options`, `appliesTo`", body = ErrorBody),
    )
)]
pub async fn create(
    State(state): State<AppState>,
    access: Manage,
    ApiJson(input): ApiJson<CustomFieldInput>,
) -> Result<(StatusCode, Json<CustomField>), AppError> {
    let row = repo::create(&state.db, access.space(), input.validate()?).await?;
    Ok((StatusCode::CREATED, Json(row.into())))
}

#[utoipa::path(
    put,
    path = "/api/settings/custom-fields/{id}",
    tag = "settings",
    security(("cookie" = []), ("bearer" = [])),
    params(("id" = Uuid, Path)),
    request_body = CustomFieldInput,
    responses(
        (status = 200, body = CustomField),
        (status = 404, body = ErrorBody),
        (status = 422, description = "As create; `key` / `type` changed → `invalid`", body = ErrorBody),
    )
)]
pub async fn update(
    State(state): State<AppState>,
    access: Manage,
    ApiPath(id): ApiPath<Uuid>,
    ApiJson(input): ApiJson<CustomFieldInput>,
) -> Result<Json<CustomField>, AppError> {
    let row = repo::update(&state.db, access.space(), id, input.validate()?).await?;
    Ok(Json(row.into()))
}

#[utoipa::path(
    delete,
    path = "/api/settings/custom-fields/{id}",
    tag = "settings",
    security(("cookie" = []), ("bearer" = [])),
    params(("id" = Uuid, Path)),
    responses(
        (status = 204, description = "Deleted; stored values stay in documents and are ignored"),
        (status = 404, body = ErrorBody),
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

    fn input(key: &str, t: &str, options: &[&str]) -> CustomFieldInput {
        CustomFieldInput {
            key: key.into(),
            label: "Project".into(),
            field_type: Some(t.into()),
            options: options.iter().map(|o| o.to_string()).collect(),
            ..Default::default()
        }
    }

    fn errors(i: CustomFieldInput) -> FieldErrors {
        match i.validate() {
            Err(AppError::Validation(e)) => e,
            other => panic!("expected validation error, got {other:?}"),
        }
    }

    #[test]
    fn keys() {
        assert!(is_valid_key("project"));
        assert!(is_valid_key("a1_b"));
        assert!(is_valid_key(&format!("a{}", "b".repeat(39))));
        assert!(!is_valid_key(&format!("a{}", "b".repeat(40))));
        assert!(!is_valid_key("1a"));
        assert!(!is_valid_key("Project"));
        assert!(!is_valid_key("a-b"));
        assert!(!is_valid_key(""));
    }

    #[test]
    fn validation() {
        let ok = input("project", "text", &[]).validate().expect("valid");
        assert_eq!(ok.applies_to, "both");
        assert!(ok.active);
        let ok = input("color", "select", &[" red ", "blue"])
            .validate()
            .expect("valid");
        assert_eq!(ok.options, vec!["red", "blue"]);
        assert_eq!(
            errors(input("Bad", "text", &[])).get("key"),
            Some("invalid")
        );
        assert_eq!(
            errors(input("x", "money", &[])).get("type"),
            Some("invalid")
        );
        assert_eq!(
            errors(input("x", "text", &["a"])).get("options"),
            Some("invalid")
        );
        assert_eq!(
            errors(input("x", "select", &[])).get("options"),
            Some("required")
        );
        let e = errors(input("x", "select", &["a", "a", ""]));
        assert_eq!(e.get("options.1"), Some("duplicate"));
        assert_eq!(e.get("options.2"), Some("required"));
        let many: Vec<String> = (0..51).map(|i| i.to_string()).collect();
        let refs: Vec<&str> = many.iter().map(String::as_str).collect();
        assert_eq!(
            errors(input("x", "select", &refs)).get("options"),
            Some("too_long")
        );
        let mut bad = input("x", "bool", &[]);
        bad.applies_to = Some("all".into());
        assert_eq!(errors(bad).get("appliesTo"), Some("invalid"));
    }
}
