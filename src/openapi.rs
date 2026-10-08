//! OpenAPI document served at `/api/openapi.json`.

use axum::Json;
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::{Modify, OpenApi};

#[derive(OpenApi)]
#[openapi(
    info(title = "Invoice API"),
    paths(crate::health::health, crate::auth::check),
    components(schemas(crate::health::HealthResponse, crate::error::ErrorBody)),
    modifiers(&BearerAuth)
)]
pub struct ApiDoc;

struct BearerAuth;

impl Modify for BearerAuth {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer",
            SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
        );
    }
}

pub async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documents_the_skeleton_routes() {
        let doc = ApiDoc::openapi();
        assert!(doc.paths.paths.contains_key("/api/health"));
        assert!(doc.paths.paths.contains_key("/api/auth/check"));
        let components = doc.components.expect("components");
        assert!(components.security_schemes.contains_key("bearer"));
    }
}
