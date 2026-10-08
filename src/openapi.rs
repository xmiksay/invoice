//! OpenAPI document served at `/api/openapi.json`.

use axum::Json;
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::{Modify, OpenApi};

#[derive(OpenApi)]
#[openapi(
    info(title = "Invoice API"),
    paths(
        crate::health::health,
        crate::auth::check,
        crate::settings::handlers::company::get_company,
        crate::settings::handlers::company::put_company,
        crate::settings::handlers::bank_accounts::list,
        crate::settings::handlers::bank_accounts::create,
        crate::settings::handlers::bank_accounts::update,
        crate::settings::handlers::bank_accounts::delete,
        crate::settings::handlers::vat_rates::list,
        crate::settings::handlers::vat_rates::create,
        crate::settings::handlers::vat_rates::update,
        crate::settings::handlers::vat_rates::delete,
        crate::settings::handlers::number_series::list,
        crate::settings::handlers::number_series::put_pattern,
        crate::settings::handlers::number_series::put_counter,
        crate::contact::handlers::contacts::list,
        crate::contact::handlers::contacts::create,
        crate::contact::handlers::contacts::get,
        crate::contact::handlers::contacts::update,
        crate::contact::handlers::contacts::delete,
        crate::ares::handlers::lookup,
    ),
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
    fn documents_the_routes() {
        let doc = ApiDoc::openapi();
        assert!(doc.paths.paths.contains_key("/api/health"));
        assert!(doc.paths.paths.contains_key("/api/auth/check"));
        for path in [
            "/api/settings/company",
            "/api/settings/bank-accounts/{id}",
            "/api/settings/vat-rates",
            "/api/settings/number-series/{docType}/counters/{year}",
            "/api/contacts",
            "/api/contacts/{id}",
            "/api/ares/{ico}",
        ] {
            assert!(doc.paths.paths.contains_key(path), "{path}");
        }
        let components = doc.components.expect("components");
        assert!(components.security_schemes.contains_key("bearer"));
    }
}
