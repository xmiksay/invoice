# Architecture

One crate, one binary (`invoice`), one Postgres database, one embedded SPA.

```
invoice serve            → migrations up → Axum server (API + SPA) on INVOICE__BIND
invoice migrate up|status|down [-n N]
```

## Backend (`src/`)

| File | Role |
|---|---|
| `main.rs` | clap CLI (`serve`, `migrate`), tracing init, graceful shutdown (SIGINT/SIGTERM) |
| `lib.rs` | module root; lets integration tests build the router |
| `app.rs` | `AppState { db, api_token, ares, cnb }`, `router(state)` — route composition (tests inject the ARES / ČNB URLs via `AresClient::new` / `CnbClient::new`) |
| `config.rs` | `config` crate, prefix `INVOICE`, separator `__`; `DbConfig` (migrate) and full config (serve) |
| `secret.rs` | `Secret<T>` — never printed by `Debug` |
| `auth.rs` | Bearer middleware, constant-time token compare (`subtle`) |
| `error.rs` | `AppError` → JSON `{"code"}` (+ `fields` for 422 `validation`); 409 `document_locked` / `invalid_state`, 502 `cnb_unavailable`; unique violations → 409 `conflict` unless a repo maps them to a field; internal causes logged only |
| `extract.rs` | `ApiJson` / `ApiQuery` / `ApiPath` — axum extractors whose rejections use the error format (400 `bad_request`, unparsable path → 404); `optional_json` for all-optional action bodies |
| `validation.rs` | pure field validators: IČO checksum, DIČ, IBAN mod-97 + normalization, CZ account number, currency/country/locale codes, text lengths |
| `time.rs` | `today()` / `current_year()` in Europe/Prague (`chrono-tz`) — overdue, default dates, ČNB cache, number preview |
| `health.rs` | `GET /api/health` with DB ping (`503 degraded` when DB is down) |
| `openapi.rs` | utoipa document at `/api/openapi.json` (bearer security scheme) |
| `spa.rs` | rust-embed of `frontend/dist`; client routes → `index.html` (`no-cache`), `/assets/*` immutable; 503 if not built |
| `migration/` | sea-orm-migration `Migrator`; raw-SQL migrations `m20261008_000001_settings` (company singleton, bank_accounts, vat_rates, number_series + number_series_counters, seeds), `m20261008_000002_contacts`, `m20261009_000001_documents` (documents, document_lines, document_vat_recap, payments, exchange_rates — `rate` NULL caches "currency not listed") |
| `settings/` | company profile, bank accounts, VAT rates, number series; `pattern.rs` (`Pattern::parse` / `Pattern::format`), `doc_type.rs` (`DocType`), `repo/number_series.rs::allocate_number(txn, doc_type, year) -> (number, seq)`; the counter `PUT` refuses to go below the highest issued sequence |
| `contact/` | address book CRUD + search (ILIKE on name/IČO/DIČ/city) |
| `ares/` | `AresClient` (reqwest, 10 s timeout) + JSON → `AresSubject` mapping, `GET /api/ares/{ico}` |
| `document/` | documents (1b: issued invoices). Pure: `line.rs` (`LineData`, `VatMode`/`Status`/`PaymentMethod`/`PaymentState`), `compute.rs` (line base, per-rate recap, rounding, CZK), `subtotals.rs` (refs/cycles/uniform rate), `state.rs` (payment state, overdue). `repo/`: `context` (defaults lookups), `write` (draft create/update/delete), `issue` (validation, ČNB rate, numbering, snapshots), `lifecycle` (cancel, mark-sent, internal note), `payments`, `query` (load/lock/list filters), `view` (rows → DTOs). `handlers/`: `documents`, `actions`, `payments`, `input`/`line_input` (request validation), `dto` |
| `cnb/` | `CnbClient` (reqwest, 10 s timeout) + `denni_kurz.txt` parser, `repo::rate` through the `exchange_rates` cache, `GET /api/exchange-rates/{currency}` |

Domain modules follow a simplified nexial/infra pattern: `src/<module>/{mod.rs (router), entity/, repo/, handlers/}`
(handler files hold their DTOs + validation; repos take validated DTOs), mounted in `app.rs` behind the Bearer
middleware and registered in `openapi.rs`. They share `AppState`, `error.rs` and `validation.rs` instead of
per-module config/error files. Migrations live in `src/migration/`.

Invariants enforced in the database as well as in the repos: one company row (`CHECK (id = 1)`), at most one
default bank account per currency and one default VAT rate, which must be active (partial unique indexes; repos promote a replacement
so a non-empty group always has exactly one), unique VAT rate value, unique contact IČO when not null. Multi-row
writes run in `db.transaction(|txn| …)`, which awaits the rollback on error. `allocate_number` is an atomic
`INSERT … ON CONFLICT DO UPDATE … RETURNING` on `number_series_counters` and must run inside the transaction
that stores the numbered document.

Documents: totals are recomputed by `compute.rs` on every save and at issue and stored (`documents.total_*`,
`payable`, `total_czk` + `document_vat_recap`); reads return the stored values. `documents.paid` is re-summed
from `payments` in the payment transaction; `paymentState` / `overdue` are derived on read (and mirrored as SQL
conditions for list filters). Every state-changing write locks the document row (`SELECT … FOR UPDATE`) before
checking its status. Issuing calls ČNB before opening the transaction and refuses (409 `conflict`) if the draft
changed meanwhile. Money arithmetic in `compute.rs` is checked; overflow is a 422. The counter `PUT` locks the
counter row before its below-issued check, serialising with `allocate_number`. Unique `(doc_type, number)` for issued documents; contact / bank account FKs are
`ON DELETE SET NULL` (issued documents keep their snapshots), lines / recap / payments cascade.

## Routes

| Route | Auth | Response |
|---|---|---|
| `GET /api/health` | public | `{"status":"ok","version"}` / 503 `{"status":"degraded"}` |
| `GET /api/openapi.json` | public | OpenAPI document |
| `GET /api/auth/check` | Bearer | 204 |
| `GET, PUT /api/settings/company` | Bearer | company profile (singleton) |
| `GET, POST /api/settings/bank-accounts`, `PUT, DELETE …/{id}` | Bearer | bank accounts |
| `GET, POST /api/settings/vat-rates`, `PUT, DELETE …/{id}` | Bearer | VAT rates |
| `GET /api/settings/number-series`, `PUT …/{docType}`, `PUT …/{docType}/counters/{year}` | Bearer | number series |
| `GET, POST /api/contacts`, `GET, PUT, DELETE /api/contacts/{id}` | Bearer | contacts (`?q=&limit=&offset=`) |
| `GET /api/ares/{ico}` | Bearer | ARES lookup → contact draft |
| `GET, POST /api/documents`, `GET, PUT, DELETE /api/documents/{id}` | Bearer | documents (list filters `?direction=&docType=&status=&paymentState=&overdue=&contactId=&q=&from=&to=&limit=&offset=`) |
| `POST /api/documents/compute` | Bearer | live totals, no DB writes |
| `POST /api/documents/{id}/issue`, `…/cancel`, `…/mark-sent`, `PUT …/internal-note` | Bearer | lifecycle |
| `GET, POST /api/documents/{id}/payments`, `DELETE …/payments/{paymentId}` | Bearer | payments |
| `GET /api/exchange-rates/{currency}?date=` | Bearer | ČNB rate (cached) |
| other `/api/*` | Bearer | 404 `{"code":"not_found"}` |
| anything else | public | SPA |

Wire format, error codes and validation rules: [`api.md`](api.md).

Unauthenticated `/api/*` → 401 `{"code":"unauthorized"}` + `WWW-Authenticate: Bearer`.

## Configuration

| Env var | Required | Default | Notes |
|---|---|---|---|
| `INVOICE__DATABASE_URL` | yes | — | Postgres URL |
| `INVOICE__API_TOKEN` | serve only | — | non-empty, single-user Bearer token |
| `INVOICE__BIND` | no | `0.0.0.0:3000` | listen address |
| `INVOICE__ARES_URL` | no | `https://ares.gov.cz/ekonomicke-subjekty-v-be/rest` | ARES REST root; tests point it at a local mock |
| `INVOICE__CNB_URL` | no | `https://www.cnb.cz/cs/financni-trhy/devizovy-trh/kurzy-devizoveho-trhu/kurzy-devizoveho-trhu/denni_kurz.txt` | ČNB daily rates (`?date=DD.MM.YYYY` appended); tests point it at a local mock |
| `RUST_LOG` | no | `invoice=info,tower_http=info,sea_orm_migration=info` | |
| `TEST_DATABASE_URL` | tests | `…@localhost:5432/invoice_test` | integration tests |

## Frontend (`frontend/`)

Vue 3 + TS + Vite + Tailwind v4 + Pinia + vue-router + vue-i18n (cs default, en).

Feature code lives in `src/features/<feature>/` with `api.ts` (calls through `request()`),
`types.ts` (wire types — decimals as strings, nullable fields as `| null`), Pinia store(s),
`views/` (routed) and `components/`. Form state is all-strings "drafts"; each feature has
`toDraft` / `validate*` / `to<Wire>` helpers so conversion and client-side checks are unit-tested
outside components. Client validation is light (required, length, IČO checksum, pattern);
the server's 422 stays the source of truth and its `fields` are shown inline.

| Path | Role |
|---|---|
| `src/api/client.ts` | fetch wrapper: Bearer header, `ApiError{status, code, fields}` (status 0 = network; `fields` from 422 `validation`), 401 → logout + redirect via handler registered in `main.ts` |
| `src/api/types.ts` | shared wire types: `ApiErrorBody`, `FieldErrors`, `DocLocale`, `AresSubject` |
| `src/api/ares.ts` | `GET /api/ares/{ico}` (used by company + contact forms) |
| `src/lib/formErrors.ts` | 422 → field errors, reason code → i18n key (`validation.*`, incl. `below_issued`), error code → message key (`errors.*`, incl. `document_locked`, `invalid_state`, `cnb_unavailable`), small validation helpers |
| `src/lib/ico.ts` | IČO mod-11 checksum |
| `src/composables/useFormSubmit.ts` | form submit state: client validate → request → inline 422 field errors |
| `src/composables/useAction.ts` | run an async action, keep a translated error message |
| `src/components/form/` | `FormField` (label + translated error), `PartyFields` (name/IČO/DIČ/address + "Načíst z ARES"), `party.ts` (shared draft, validation, ARES merge) |
| `src/features/settings/` | company profile, bank accounts, VAT rates, number series. `stores.ts`: company store, CRUD stores that reload the list after each mutation (server owns default flags), number-series store. `numberPattern.ts` mirrors the backend `Pattern::parse` / `Pattern::format` for the live preview |
| `src/features/contacts/` | contact list (debounced `q`, limit/offset paging, state kept in the store) and create/edit form |
| `src/features/documents/` | issued invoices (Phase 1b). `types.ts` wire types (`DocumentLine` union, decimals as strings); `api.ts` documents, compute, lifecycle, internal note, payments, `exchangeRatesApi`; `store.ts` list store (filters + paging) and current-document store (every action stores the server's answer; payments reload the document). `lines.ts` pure line logic: client `key` per line, defaults, `moveLine` / `removeLine` remap subtotal `refs` (1-based positions), `splitLineErrors` routes 422 `lines.N.field` (N = 0-based index) to the line. `form.ts` draft ↔ `DocumentInput`, client-side mirror of the server create defaults, contact/currency side effects. `useCompute.ts` live totals: `POST /api/documents/compute` debounced 300 ms, stale requests aborted and ignored, `pending` until the latest scheduled change is answered. `useIndicativeRate.ts` fetches the ČNB rate for the tax point date (debounced) — shown as indicative and fed to compute only when no manual rate is set, never saved (issue fixes the rate); `exchangeRate` in the editor is the manual override and is cleared on any currency change. `format.ts` Intl money/date formatting, exact remaining amount. Views: list, editor (draft only; issued redirects to the detail), detail with state-dependent actions, payments and internal note |
| `src/stores/auth.ts` | token in `localStorage` (`invoice.token`), `login()` validates via `/api/auth/check` |
| `src/stores/health.ts` | backend version (footer in `App.vue`) |
| `src/router/index.ts` | `/login` (public), `/` → `/invoices`, `/invoices`, `/invoices/new`, `/invoices/:id`, `/invoices/:id/edit`, `/contacts`, `/contacts/new`, `/contacts/:id`, `/settings/{company,bank-accounts,vat-rates,number-series}` (`/settings` → company tab); guard keeps `?redirect=` |
| `src/locales/{cs,en}.json` | UI strings; parity test. Messages must not contain `{`, `@` or `\|` literally (vue-i18n syntax) |
| `src/lib/storage.ts` | try/catch-wrapped `localStorage` |
| `src/test-utils.ts` | fetch stubs for tests: single response, sequence, `mockFetchRoutes` by `"METHOD /path"` |
| `src/style.css` | Tailwind + shared component classes (`.input`, `.btn*`, `.card`, `.table`, `.badge`, `.alert-error`) |

Dev: Vite proxies `/api` → `http://localhost:3000`.

## Build, CI, image

- `Dockerfile`: node:22 builds the SPA → `rust:1.97.0-bookworm` builds the release binary with the SPA embedded (`--locked`) → `debian:bookworm-slim`, non-root uid 10001, `EXPOSE 3000`, `invoice serve`.
- `.github/workflows/ci.yml`: PR + push to `master`; backend job (Postgres service, `make lint-backend test-backend`, toolchain pin check), frontend job (`make frontend-install lint-frontend test-frontend frontend-build`).
- `.github/workflows/docker.yml`: `v*` tag → `ghcr.io/xmiksay/invoice:<version>` + `latest`.
- Deployment manifests are out of scope for now.
