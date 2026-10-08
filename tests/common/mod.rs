//! Shared integration-test harness.
//!
//! Each [`TestDb`] owns a unique throwaway Postgres schema (`test_<n>`) on the
//! server named by `TEST_DATABASE_URL`, runs every migration into it, and
//! drops it on teardown — tests are parallel-safe with no shared state.

#![allow(dead_code)]

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, Response};
use invoice::app::{self, AppState};
use invoice::migration::{Migrator, MigratorTrait};
use invoice::secret::Secret;
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection};
use tower::ServiceExt;

pub const TEST_TOKEN: &str = "test-token-0123456789";

pub fn test_database_url() -> String {
    std::env::var("TEST_DATABASE_URL").expect(
        "TEST_DATABASE_URL must be set for integration tests \
         (local Postgres, see .env.example)",
    )
}

fn unique_schema() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    format!(
        "test_{}_{}_{}",
        std::process::id(),
        nanos,
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

pub struct TestDb {
    pub conn: DatabaseConnection,
    pub schema: String,
    base_url: String,
}

impl TestDb {
    pub async fn new() -> Self {
        let base_url = test_database_url();
        let schema = unique_schema();

        let admin = Database::connect(&base_url)
            .await
            .expect("connect to TEST_DATABASE_URL (is the local Postgres running? see .env.example)");
        admin
            .execute_unprepared(&format!("CREATE SCHEMA \"{schema}\""))
            .await
            .expect("create test schema");
        admin.close().await.expect("close admin connection");

        let mut opts = ConnectOptions::new(base_url.clone());
        opts.set_schema_search_path(schema.clone())
            .max_connections(5)
            .sqlx_logging(false);
        let conn = Database::connect(opts)
            .await
            .expect("connect to test schema");
        Migrator::up(&conn, None).await.expect("run migrations");

        Self {
            conn,
            schema,
            base_url,
        }
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        // Dedicated thread + runtime: works under any test runtime flavor.
        let url = self.base_url.clone();
        let schema = self.schema.clone();
        let _ = std::thread::spawn(move || {
            let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            else {
                return;
            };
            rt.block_on(async move {
                if let Ok(conn) = Database::connect(&url).await {
                    let _ = conn
                        .execute_unprepared(&format!("DROP SCHEMA IF EXISTS \"{schema}\" CASCADE"))
                        .await;
                }
            });
        })
        .join();
    }
}

pub fn router(db: DatabaseConnection) -> Router {
    app::router(AppState {
        db,
        api_token: Secret::new(TEST_TOKEN.to_string()),
    })
}

pub async fn send(app: Router, req: Request<Body>) -> Response<Body> {
    app.oneshot(req).await.expect("infallible router")
}

pub fn get(uri: &str, bearer: Option<&str>) -> Request<Body> {
    let mut b = Request::builder().uri(uri);
    if let Some(t) = bearer {
        b = b.header("authorization", format!("Bearer {t}"));
    }
    b.body(Body::empty()).expect("build request")
}

pub async fn json(resp: Response<Body>) -> serde_json::Value {
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20)
        .await
        .expect("read body");
    serde_json::from_slice(&bytes).expect("JSON body")
}
