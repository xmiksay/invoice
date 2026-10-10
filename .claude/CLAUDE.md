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
| Database | local Postgres 18 on `localhost:5432`: `invoice` (dev), `invoice_test` (tests); setup in `.env.example` |
| Full gate (must be green before push) | `make lint` and `make test` |
| Backend only | `make lint-backend`, `make test-unit`, `make test-integration` |
| Frontend only | `make frontend-install`, `make lint-frontend`, `make test-frontend` |
| Dev | `make dev-server` + `make dev-frontend` (Vite proxies `/api` → `:3000`) |
| Release build / image | `make build`, `make docker-build` |

## Rules

- Branch `master` is the default. Work on a feature branch, one PR per phase / task, fast-forward only, Conventional Commits.
- Toolchain is pinned: `rust-toolchain.toml` (1.97.0) must match `dtolnay/rust-toolchain` in `.github/workflows/ci.yml` and the `rust:` image in `Dockerfile`; Node is `frontend/.nvmrc` (22). Bump them together.
- Integration tests need Postgres (`TEST_DATABASE_URL`); each test gets a throwaway schema (`tests/common/mod.rs`). They fail, not skip, without it.
- The storage tests also need the S3 test bucket (`TEST_S3_*`, in `.env` locally, GitHub secrets in CI; random prefix per test, cleaned up) — they fail, not skip, without it. The ISDOC, Pohoda and Money export tests also need `xmllint` (`libxml2-utils`) for XSD validation.
- E-mail tests (`tests/email*.rs`) send through an in-process mock SMTP server (`tests/common/smtp.rs`, plain SMTP, can be told to reject) — never a real SMTP server.
- Errors to clients are `{"code": "..."}`, plus a `fields` map on 422 (`{"code":"validation","fields":{"<camelCaseField>":"<reason>"}}`) and, where a contract says so, a human-oriented `detail` string (e.g. the missing CSV column, a template line, an SMTP reply) — internal/DB detail is logged, never returned (`src/error.rs`).
- Wire format and every route are specified in [`docs/api/`](../docs/api/README.md) (one file per area, each < 400 lines) — update it with any API change.
- All `/api/*` except `/api/health` and `/api/openapi.json` require the Bearer token (`src/auth.rs`).
- UI strings go through vue-i18n; `frontend/src/locales/{cs,en}/<namespace>.json` must have the same files and identical keys (test enforced); split a namespace file before it passes 400 lines.
- Migrations are append-only (`src/migration/mod.rs` explains how to add one).
- New env var → `.env.example` + `docs/architecture.md` in the same change.
