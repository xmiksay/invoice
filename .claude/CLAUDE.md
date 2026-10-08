# Invoice — project brief

Single-user invoice management (issued + received invoices, Czech VAT, PDF via the
mdcast service with a per-instance design). Rust/Axum + SeaORM/Postgres backend,
Vue 3 SPA embedded in the binary.

- Agreed scope, decisions and phases: [`docs/plan.md`](../docs/plan.md) — read before any feature work.
- Architecture and module layout: [`docs/architecture.md`](../docs/architecture.md).
- Domain logic is ported from `nexial/infra/src/invoice` (simplified: no tenants, no OAuth). Mirror its module pattern.

## Commands

Everything goes through `make` (`make help` lists targets). `CARGO_BUILD_JOBS=4` is exported by the Makefile.

| Task | Command |
|---|---|
| Test DB (Postgres 16, docker) | `make test-db-up` / `make test-db-down` |
| Full gate (must be green before push) | `make lint` and `make test` |
| Backend only | `make lint-backend`, `make test-unit`, `make test-integration` |
| Frontend only | `make frontend-install`, `make lint-frontend`, `make test-frontend` |
| Dev | `make dev-server` + `make dev-frontend` (Vite proxies `/api` → `:3000`) |
| Release build / image | `make build`, `make docker-build` |

If host port 5433 is taken, pass `TEST_DB_PORT=<port>` to both `make test-db-up` and `make test`.

## Rules

- Branch `master` is the default. Work on a feature branch, one PR per phase / task, fast-forward only, Conventional Commits.
- Toolchain is pinned: `rust-toolchain.toml` (1.97.0) must match `dtolnay/rust-toolchain` in `.github/workflows/ci.yml` and the `rust:` image in `Dockerfile`; Node is `frontend/.nvmrc` (22). Bump them together.
- Integration tests need Postgres (`TEST_DATABASE_URL`); each test gets a throwaway schema (`tests/common/mod.rs`). They fail, not skip, without it.
- Errors to clients are `{"code": "..."}` only — internal/DB detail is logged, never returned (`src/error.rs`).
- All `/api/*` except `/api/health` and `/api/openapi.json` require the Bearer token (`src/auth.rs`).
- UI strings go through vue-i18n; `frontend/src/locales/cs.json` and `en.json` must have identical keys (test enforced).
- Migrations are append-only (`src/migration/mod.rs` explains how to add one).
- New env var → `.env.example` + `docs/architecture.md` in the same change.
