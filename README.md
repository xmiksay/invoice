# invoice

Single-user invoice management: issued and received invoices, Czech VAT, ISDOC,
PDF rendering through [mdcast](https://github.com/xmiksay/mdcast) with a
per-instance design. Rust (Axum + SeaORM/Postgres) with an embedded Vue 3 SPA.

Status: skeleton — see [docs/plan.md](docs/plan.md) for scope and phases,
[docs/architecture.md](docs/architecture.md) for the layout.

## Quick start

```sh
cp .env.example .env            # set INVOICE__API_TOKEN (openssl rand -hex 32)
make test-db-up                 # Postgres 16 on localhost:5433
make frontend-install
make dev-server                 # backend on :3000
make dev-frontend               # Vite on :5173, proxies /api
```

Open the SPA and log in with the API token.

## Make targets

| Target | Description |
|---|---|
| `make build` | Build the SPA, then the release binary with the SPA embedded |
| `make run` / `make dev-server` | Run the server (`invoice serve`) |
| `make dev-frontend` | Vite dev server |
| `make frontend-install` / `make frontend-build` | `npm ci` / production SPA build |
| `make fmt` / `make fmt-check` / `make clippy` | Rust formatting and lints |
| `make lint` | `lint-backend` + `lint-frontend` (eslint + vue-tsc) |
| `make test-unit` / `make test-integration` | Rust unit / integration tests (integration needs Postgres) |
| `make test` | `test-backend` + `test-frontend` |
| `make test-db-up` / `make test-db-down` | Dev/test Postgres (`TEST_DB_PORT`, default 5433) |
| `make migrate` / `make migrate-status` | Apply / show migrations |
| `make docker-build` | Build the Docker image (`IMAGE=...` to override the tag) |
| `make clean` | Remove build artifacts |

## Configuration

See [.env.example](.env.example) and the table in [docs/architecture.md](docs/architecture.md#configuration).
