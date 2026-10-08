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
| `app.rs` | `AppState { db, api_token, ares, cnb, pdf }`, `router(state)` — route composition (tests inject the ARES / ČNB / mdcast URLs via `AresClient::new` / `CnbClient::new` / `PdfService::new`) |
| `config.rs` | `config` crate, prefix `INVOICE`, separator `__`; `DbConfig` (migrate) and full config (serve) |
| `secret.rs` | `Secret<T>` — never printed by `Debug` |
| `auth.rs` | Bearer middleware, constant-time token compare (`subtle`) |
| `error.rs` | `AppError` → JSON `{"code"}` (+ `fields` for 422 `validation`); 409 `document_locked` / `invalid_state` / `advance_settled` / `advance_in_use` / `catalog_item_in_use` / `category_in_use` / `number_taken`, 404 `pdf_missing`, 413 `too_large`, 502 `cnb_unavailable`, 503 `pdf_unavailable`, 502 `pdf_render_failed` (+ `detail` only for template / design failures; other upstream errors logged, no detail); unique violations → 409 `conflict` unless a repo maps them to a field (`number_violation` maps the document-number indexes); internal causes logged only |
| `extract.rs` | `ApiJson` / `ApiQuery` / `ApiPath` — axum extractors whose rejections use the error format (400 `bad_request`, unparsable path → 404); `optional_json` for all-optional action bodies |
| `validation.rs` | pure field validators: IČO checksum, DIČ, IBAN mod-97 + normalization, CZ account number, currency/country/locale codes, text lengths |
| `time.rs` | `today()` / `current_year()` in Europe/Prague (`chrono-tz`) — overdue, default dates, ČNB cache, number preview |
| `health.rs` | `GET /api/health` with DB ping (`503 degraded` when DB is down) |
| `openapi.rs` | utoipa document at `/api/openapi.json` (bearer security scheme) |
| `spa.rs` | rust-embed of `frontend/dist`; client routes → `index.html` (`no-cache`), `/assets/*` immutable; 503 if not built |
| `migration/` | sea-orm-migration `Migrator`; raw-SQL migrations `m20261008_000001_settings` (company singleton, bank_accounts, vat_rates, number_series + number_series_counters, seeds), `m20261008_000002_contacts`, `m20261009_000001_documents` (documents, document_lines, document_vat_recap, payments, exchange_rates — `rate` NULL caches "currency not listed"), `m20261010_000001_phase_1c` (`documents.payment_id` unique FK → payments `ON DELETE SET NULL`, `correction_reason`, `exchange_rate_source` CHECK + `'original'`; `document_lines.kind` CHECK + `'advance'`, `advance_document_id` FK, `advance_recap` jsonb; catalog_items, catalog_groups, catalog_group_members), `m20261011_000001_pdf_archive` (`documents.pdf_path`, `pdf_sha256`, `pdf_rendered_at`), `m20261012_000001_received_import` (series seeds `received_credit_note` / `received_proforma` / `received_advance_tax_doc`; `categories` (unique `(kind, lower(name))`), `custom_fields` (unique `key`); `documents.due_date` nullable, `supplier_number`, `received_date`, `vat_deductible`, `supplier_account`, `category_id` FK (no action), `custom_fields jsonb`, `original_path` / `original_sha256` / `original_size` / `original_uploaded_at`; unique `(doc_type, number)` for received documents) |
| `settings/` | company profile, bank accounts, VAT rates, number series, categories, custom field definitions; `pattern.rs` (`Pattern::parse` / `Pattern::format`), `doc_type.rs` (`DocType` = series key: the four document types plus `received*`; `series(direction)` / `numbered()` map document ↔ series), `repo/number_series.rs::allocate_number(txn, doc_type, year) -> (number, seq)`; the counter `PUT` refuses to go below the highest issued sequence (of the series' direction + type, imports excluded). `repo/categories.rs` (in use → no delete / kind change; row lock against concurrent FK writers), `repo/custom_fields.rs` (`key` / `type` immutable) |
| `contact/` | address book CRUD + search (ILIKE on name/IČO/DIČ/city) |
| `ares/` | `AresClient` (reqwest, 10 s timeout) + JSON → `AresSubject` mapping, `GET /api/ares/{ico}` |
| `document/` | documents: invoices, proformas, DDPPs (`advance_tax_doc`), credit notes. Pure: `line.rs` (`LineData` incl. `Advance(AdvanceData)` with its deducted `AdvanceRow`s, `VatMode`/`Status`/`PaymentMethod`/`PaymentState`), `compute.rs` (line base, per-rate recap incl. advance deductions, rounding, CZK), `subtotals.rs` (refs/cycles/uniform rate), `state.rs` (payment state, overdue; none for DDPPs), `advance.rs` (advance-line rules + deducted amounts from `AdvanceSource`s), `ddpp.rs` (payment split by rate, VAT from above, DDPP totals), `credit.rs` (line copy, per-rate cap), `defaults.rs` (contact ?? company due date / locale), `received.rs` (received totals from the entered recap: Σ(base+vat) + rounding, entered payable, CZK per row), `custom_fields.rs` (value validation against the active definitions; stored values of inactive / deleted fields kept when sent back unchanged). `repo/`: `context` (defaults lookups, `Existing` draft), `write` (draft create/update/delete), `issue` (validation, ČNB rate, numbering per doc type, snapshots), `lifecycle` (cancel, mark-sent, internal note), `payments`, `ddpp` (auto-issue in the payment transaction, cancel on payment delete), `settle` (final invoice from a proforma), `credit` (credit-note draft, cap at issue), `advance_sources` (load / lock + re-check deducted documents; issued only), `query` (load/lock/list filters), `view` (rows → DTOs), `received` (create with the internal number in the same transaction, replace, delete), `original` (uploaded original PDF: put / delete), `meta` (custom field definitions, metadata write). `handlers/`: `documents` (create / PUT / DELETE dispatch by `direction`), `actions` (issue, cancel, mark-sent, note, metadata, settle, credit-note), `payments`, `original` (multipart upload, 20 MiB), `received` + `received_input` (received body, ČNB rate for `receivedDate` resolved before the transaction), `input`/`compute_input`/`line_input`/`existing`/`fields`/`imported`/`meta` (request validation; `Existing` = the draft a PUT replaces, credit-note binding; `imported` = manual import flag, own number, informational link; `meta` = category + custom fields), `dto`/`line_out` |
| `catalog/` | catalog items and groups (ordered members with quantities). `handlers/dto.rs` validation incl. `check_items` (members exist, one VAT rate → `mixed_vat`); `repo/groups.rs` replaces members in a transaction with the items locked `FOR SHARE`; deleting an item cascades out of groups unless it is a group's last member (409); an item rate change may not mix a group |
| `pdf/` | PDF via mdcast (`mdcast-client`, 60 s timeout). `mod.rs` `PdfService` (client, design dir, storage dir; `render(payload)` loads the design, adds `qr.svg`, posts `render_template` — the client does the blob `409` negotiation), `client.rs` (mdcast error → 503 `pdf_unavailable` / 502 `pdf_render_failed`), `design.rs` (rust-embed of `design/` ∪ `INVOICE__DESIGN_DIR`, dir wins per file, re-read per render, hidden paths skipped, > 10 MB → render failed, `fonts/**/*.ttf\|otf` → request `fonts`, `logo.svg` > `logo.png`, `signature.png`, `qr.svg` reserved). Pure: `format.rs` (cs/en money, quantity, percent, rate, date), `labels.rs` (every printed text, titles, legal notes), `payload.rs` (`Input` → `/data.json`), `amounts.rs` (lines with collapsed members hidden, recaps incl. CZK, totals rows, credit-note negation), `spayd.rs` (SPAYD string, QR rule, SVG). `source.rs` loads a document (snapshots once issued, live company / contact / bank for a draft), `archive.rs` (render + `UPDATE … WHERE pdf_path IS NULL` + file write inside the transaction; issue, DDPP after its payment, first download), `storage.rs` (temp file + fsync + rename, sha256), `preview.rs` (sample invoice), `handlers.rs`. Received / imported documents are never rendered: `GET …/pdf` serves their uploaded original (`documents/{year}/{id}-original-{sha8}.pdf`) or 404 `pdf_missing` |
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
counter row before its below-issued check, serialising with `allocate_number`. Unique `(doc_type, number)` per direction (imported drafts included; a native issue that collides → 409 `number_taken`); contact / bank account FKs are
`ON DELETE SET NULL` (issued documents keep their snapshots), lines / recap / payments cascade.

Advances (1c): a payment on an issued proforma whose supplier snapshot is a VAT payer in `standard` mode issues a
DDPP in the same transaction (rate resolved before it; ČNB failure → 422 and nothing stored). A DDPP's recap comes
from `ddpp.rs` (VAT from above) and is stored as is — DDPPs are created issued and never pass through save/issue,
so nothing recomputes it from base × rate. DDPPs have no payments, cannot be edited or cancelled directly; deleting
the payment cancels its DDPP unless an invoice deducts it (issued: 409 `advance_settled`, draft: 409 `advance_in_use`). `advance` lines store a
snapshot of the deducted amounts (`advance_recap`), refreshed and re-validated at issue; every write holding
advance lines locks the deducted documents (id order) and re-checks they are issued and not on another
non-cancelled invoice. Lock order is always owning document → referenced documents (invoice → DDPP/proforma,
proforma → DDPP, credit note → invoice). Credit notes are stored positive (`sign = -1`), keep the invoice's rate
(`exchange_rate_source = 'original'`), and issuing one locks the invoice before the per-rate cap check.

PDF (1d): issuing renders and archives the PDF inside the issue transaction (explicit `begin` / `commit`); a render
failure rolls everything back (no number, no snapshots), a failed commit removes the written file. The payment that
auto-issues a DDPP never fails because of PDF: after its transaction commits the handler spawns one archive attempt
(background, the 201 is not delayed); otherwise the first `GET …/pdf` does (concurrent first downloads race on
`WHERE pdf_path IS NULL`, the loser serves the stored file). A cancelled document without an archive renders with
the "STORNO" watermark and no QR. The render holds the issue transaction's locks (up to the 60 s timeout) — an
accepted trade-off for a single-user app. Archives are immutable. Default design: repo `design/` (`invoice.typ` + Inter TTFs + OFL),
embedded in release builds (debug builds read it from disk).

Received & import (1e): received documents have no draft — `POST` records them (`status = issued`) and allocates the
internal number from the doc type's `received*` series for the `receivedDate` year in the same transaction; `PUT` /
`DELETE` work in any status (number never changes; delete removes payments by cascade and the original file after the
commit). Their recap is entered and stored as is (`document_vat_recap`), never recomputed. The ČNB rate for
`receivedDate` is fetched before the transaction and kept while currency and `receivedDate` stay the same. Received and
imported documents never issue DDPPs, never get a rendered PDF, and are excluded from advance deductions. Imported
issued documents are drafts with their own `number`; issuing them allocates nothing and renders nothing. Custom field
values (`documents.custom_fields`) are validated on every save, at issue and by `PUT …/metadata`.

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
| `GET, POST /api/documents`, `GET, PUT, DELETE /api/documents/{id}` | Bearer | documents, both directions (list filters `?direction=&docType=&status=&paymentState=&overdue=&contactId=&categoryId=&imported=&q=&from=&to=&limit=&offset=`) |
| `POST /api/documents/compute` | Bearer | live totals, no DB writes |
| `POST /api/documents/{id}/issue`, `…/cancel`, `…/mark-sent`, `PUT …/internal-note`, `PUT …/metadata` | Bearer | lifecycle; metadata (category, custom fields, note) in every status |
| `PUT, DELETE /api/documents/{id}/original` | Bearer | original PDF of a received / imported document (multipart `file`, ≤ 20 MiB) |
| `GET, POST /api/settings/categories`, `PUT, DELETE …/{id}`; same for `/api/settings/custom-fields` | Bearer | categories, custom field definitions |
| `GET, POST /api/documents/{id}/payments`, `DELETE …/payments/{paymentId}` | Bearer | payments |
| `POST /api/documents/{id}/settle`, `POST /api/documents/{id}/credit-note` | Bearer | final invoice from a proforma / credit note from an invoice (201 draft) |
| `GET, POST /api/catalog/items`, `GET, PUT, DELETE …/{id}`; same for `/api/catalog/groups` | Bearer | catalog (`?q=`, items also `?active=`) |
| `GET /api/exchange-rates/{currency}?date=` | Bearer | ČNB rate (cached) |
| `GET /api/documents/{id}/pdf?download=1` | Bearer | PDF: received / imported → the uploaded original (404 `pdf_missing`); draft rendered live (watermark, never stored), issued / cancelled → the archive |
| `GET /api/pdf/preview?locale=`, `GET /api/pdf/design` | Bearer | sample invoice PDF; effective design file set |
| other `/api/*` | Bearer | 404 `{"code":"not_found"}` |
| anything else | public | SPA |

Wire format, error codes and validation rules: [`api/`](api/README.md).

Unauthenticated `/api/*` → 401 `{"code":"unauthorized"}` + `WWW-Authenticate: Bearer`.

## Configuration

| Env var | Required | Default | Notes |
|---|---|---|---|
| `INVOICE__DATABASE_URL` | yes | — | Postgres URL |
| `INVOICE__API_TOKEN` | serve only | — | non-empty, single-user Bearer token |
| `INVOICE__BIND` | no | `0.0.0.0:3000` | listen address |
| `INVOICE__ARES_URL` | no | `https://ares.gov.cz/ekonomicke-subjekty-v-be/rest` | ARES REST root; tests point it at a local mock |
| `INVOICE__CNB_URL` | no | `https://www.cnb.cz/cs/financni-trhy/devizovy-trh/kurzy-devizoveho-trhu/kurzy-devizoveho-trhu/denni_kurz.txt` | ČNB daily rates (`?date=DD.MM.YYYY` appended); tests point it at a local mock |
| `INVOICE__MDCAST_URL` | no | `https://mdcast.nexial.cz` | mdcast root (empty → default); tests point it at a local mock |
| `INVOICE__MDCAST_TOKEN` | no | — | Bearer for mdcast |
| `INVOICE__DESIGN_DIR` | no | — | design override dir; must exist when set (startup error otherwise) |
| `INVOICE__STORAGE_DIR` | no | `./data` | archive root (`documents/{year}/{id}.pdf`, originals `documents/{year}/{id}-original-{sha8}.pdf`), created at start; image: `/data` volume |
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
| `src/api/client.ts` | fetch wrapper: Bearer header, `ApiError{status, code, fields, detail}` (status 0 = network; `fields` from 422 `validation`; `detail` from 502 `pdf_render_failed`), 401 → logout + redirect via handler registered in `main.ts`. `request()` for JSON (a `FormData` body goes out as multipart without a Content-Type, so the browser sets the boundary), `requestBlob()` for binaries (PDF) → `{blob, filename}` (filename from `Content-Disposition`, `filename*` wins) |
| `src/api/types.ts` | shared wire types: `ApiErrorBody` (incl. optional `detail`), `FieldErrors`, `DocLocale`, `AresSubject` |
| `src/api/ares.ts` | `GET /api/ares/{ico}` (used by company + contact forms) |
| `src/lib/formErrors.ts` | 422 → field errors, reason code → i18n key (`validation.*`, incl. `below_issued`, `exceeds_original`, `mixed_vat`), error code → message key (`errors.*`, incl. `document_locked`, `invalid_state`, `cnb_unavailable`, `advance_settled`, `advance_in_use`, `catalog_item_in_use`, `pdf_unavailable`, `pdf_render_failed`, `pdf_missing`, `too_large`, `category_in_use`, `number_taken`; reasons incl. `unknown`, `inactive`), `errorDetailOf` (server `detail`), small validation helpers |
| `src/lib/ico.ts` | IČO mod-11 checksum |
| `src/lib/pdf.ts` | blob → object URL (revoked after 5 min): `openPdf` opens the tab synchronously **before** the fetch (a `window.open` after a slow render loses the click's user activation and is popup-blocked), then points it at the blob; blocked → navigates the current tab; failure → closes the blank tab. `downloadPdf` clicks an `<a download>` (server filename, else the caller's fallback) |
| `src/lib/bytes.ts` | `formatBytes` (1024 steps, locale decimals) |
| `src/composables/useFormSubmit.ts` | form submit state: client validate → request → inline 422 field errors |
| `src/composables/useAction.ts` | run an async action, keep a translated error message |
| `src/composables/usePdf.ts` | `open` / `download` a PDF loader with `busy`, translated `error` and render `detail` |
| `src/components/ErrorDetail.vue` | collapsible `<pre>` with a server error `detail` (typst diagnostics) |
| `src/components/form/` | `FormField` (label + translated error), `PartyFields` (name/IČO/DIČ/address + "Načíst z ARES"), `party.ts` (shared draft, validation, ARES merge) |
| `src/features/settings/` | company profile, bank accounts, VAT rates, number series (incl. the `received*` series), categories (`CategoriesTab`: name, kind expense/income, active toggle, position; 409 `category_in_use` → "deactivate instead"), custom fields (`CustomFieldsTab`: key + type immutable after create, select options one per line, appliesTo, required, active, position; `category.ts` / `customField.ts` drafts + validation), PDF design (`DesignTab`: design dir or built-in, effective files with custom/default source + size, preview CZ/EN via `pdfApi.preview`, render error detail). `stores.ts`: company store, CRUD stores that reload the list after each mutation (server owns default flags), number-series store, design store (reloaded per visit). `numberPattern.ts` mirrors the backend `Pattern::parse` / `Pattern::format` for the live preview |
| `src/features/contacts/` | contact list (debounced `q`, limit/offset paging, state kept in the store) and create/edit form |
| `src/features/documents/` | issued documents of every type: invoice, proforma, credit note, DDPP (`advance_tax_doc`). `types.ts` wire types (`DocumentLine` union incl. `advance`, decimals as strings, `sign`, `parent`, `relatedDocuments`, `pdf` archive `{sha256, renderedAt}`); `api.ts` documents, compute, lifecycle, PDF blob (`?download=1`), settle, credit note, internal note, payments (optional `exchangeRate`), `exchangeRatesApi`; `store.ts` list store (doc-type tab + filters + paging) and current-document store (every action stores the server's answer; payments reload the document). `lines.ts` pure line logic: client `key` per line, defaults, `moveLine` / `removeLine` remap subtotal `refs` (1-based positions), `refTargets` (items + subtotals only — never text or advance lines), advance lines keep their server display fields (description, negative base, per-rate recap) in the draft and go out as `{kind, advanceDocumentId}`, `splitLineErrors` routes 422 `lines.N.field` (N = 0-based index) to the line. `catalogLines.ts` catalog → lines: item → item line (price empty when the catalog currency differs), group → member lines + a subtotal over them with the group's collapse. `form.ts` draft ↔ `DocumentInput` with a fixed `docType` (proforma: no tax point; credit note: `correctionReason` required, currency/rate/contact fixed), client-side mirror of the server create defaults, contact/currency side effects. `useCompute.ts` live totals: `POST /api/documents/compute` debounced 300 ms, stale requests aborted and ignored, `pending` until the latest scheduled change is answered. `useIndicativeRate.ts` fetches the ČNB rate for a date (debounced) — shown as indicative and fed to compute only when no manual rate is set, never saved (issue fixes the rate); also the hint for a proforma payment's rate. `exchangeRate` in the editor is the manual override and is cleared on any currency change. `format.ts` Intl money/date formatting, exact remaining amount, exact `negate`/`signed` (credit notes are stored positive, shown negated). `routes.ts` route helpers; the list tab is `?type=`. Views: list with doc-type tabs, editor (draft only; issued redirects to the detail; `/invoices/new?docType=proforma`), detail with doc-type-dependent actions (settle a proforma, credit note from an invoice, a DDPP is read-only; issue maps `pdf_unavailable` / `pdf_render_failed` to "not issued" + collapsible detail), `DocumentPdf` (every type: open / download via `documentsApi.pdf`; draft → "PDF preview" + watermark hint; archive time from `pdf.renderedAt`), payments (DDPP links, delete warning), related documents and the metadata card **1e:** the API, list store factory (`useInvoiceListStore` issued / `useReceivedListStore` received; filters incl. `categoryId`, `imported`), current-document store and route helpers (`listLocation` / `detailLocation` / `editLocation` take a `direction`; `relatedTargetType`) serve both directions; `types.ts` adds `ReceivedDocumentInput`, `VatRecapEntry`, `OriginalPdf`, `MetadataInput`, `CustomFieldValues`. Manual import of issued documents: `/invoices/new?docType=…&imported=1` (any of the four types) → required own `number`, optional `RelatedDocumentPicker` (DDPP / final invoice → proforma, credit note → invoice; issued targets only, searchable by `q`, first 50 shown; only imports send `relatedDocumentId`), a native credit note's locks do not apply; detail of an imported document: settle / credit note stay available, issue confirm says no number allocation and no PDF render, `OriginalPdfPanel` instead of `DocumentPdf`, payments never offer a DDPP rate. `OriginalPdfPanel` (received + imported): upload / replace via `PUT …/original` (PDF only, ≤ 20 MB checked client-side in `original.ts`; 413 / 422 `file` mapped), delete, open / download through `documentsApi.pdf` (404 `pdf_missing` → "PDF nenahráno"). Lists: "Importovaný" badge, "bez PDF" hint (`hasPdf` false on received / imported), category name; shared `DocTypeTabs`, `ListPager`, `ListDocMeta`, `IndicativeRateNote`. A received document opened under `/invoices/…` redirects to `/received/…` and vice versa |
| `src/features/received/` | received documents (no draft: saved = recorded, editable and deletable any time). `recap.ts` pure VAT-recap logic in BigInt cents: VAT suggested as round2(base × rate / 100) (half away from zero) until the user types one (clearing resumes it), VAT forced 0 unless `vatMode` standard, `recapTotal` = Σ(base + vat) + rounding, `validateRecap` with the server's `vatRecap.N.rate|base|vat` keys (rate 0–100, unique). `form.ts` draft ↔ `ReceivedDocumentInput`: `receivedDate` follows taxPoint ?? issue and `payable` follows the total until edited (`applyDefaults`, auto flags re-derived on edit), tax point hidden for proforma, due date for DDPP, `vatDeductible` reset to "standard" on a VAT-mode change, manual rate only (ČNB rate for `receivedDate` fixed by the server; indicative rate shown). Views: list with doc-type tabs (`?type=`), filters incl. category; form (`ContactPicker` as supplier, `ReceivedHeaderFields`, `VatRecapEditor`, `RelatedDocumentPicker`, `MetadataFields`); detail (info, totals negated for credit notes, payments without DDPP links, related documents, `OriginalPdfPanel`, `MetadataCard`, edit / delete) |
| `src/features/metadata/` | category + custom fields + internal note of any document. `metadata.ts`: applicable definitions (active, matching direction, by position), category options (kind by direction: received → expense, issued → income; the current inactive one stays listed), custom field draft / validation (`customFields.<key>`: required, number, date, select option, text ≤ 500) / serialization ("" → null, decimal comma → dot, keys without an applicable definition sent back unchanged). `useMetadataSettings` loads categories + custom fields once. `MetadataFields` (used by the issued editor and the received form), `CustomFieldsInputs`, `MetadataCard` (detail panel, `PUT /api/documents/{id}/metadata`, every status; unsaved edits survive reloads of the same document; replaces the 1b internal-note card) |
| `src/features/catalog/` | catalog items + groups (`/catalog/items`, `/catalog/groups`): `api.ts`, `store.ts` (search term + list, reload after each mutation), `form.ts` drafts/validation (group members ordered, unique, one VAT rate), item form, group form with the member editor. Insertion into documents lives in `documents/catalogLines.ts` + `CatalogPicker.vue` |
| `src/stores/auth.ts` | token in `localStorage` (`invoice.token`), `login()` validates via `/api/auth/check` |
| `src/stores/health.ts` | backend version (footer in `App.vue`) |
| `src/router/index.ts` | `/login` (public), `/` → `/invoices`, `/invoices` (`?type=` doc-type tab), `/invoices/new` (`?docType=proforma`; `?docType=…&imported=1` imports any type), `/invoices/:id`, `/invoices/:id/edit` (every doc type), `/received` (`?type=` tab), `/received/new` (`?docType=`), `/received/:id`, `/received/:id/edit`, `/contacts`, `/contacts/new`, `/contacts/:id`, `/catalog/{items,groups}` (`/catalog` → items tab), `/settings/{company,bank-accounts,vat-rates,number-series,categories,custom-fields,design}` (`/settings` → company tab); guard keeps `?redirect=` |
| `src/locales/{cs,en}/<namespace>.json` | UI strings, one file per top-level namespace (file content = that namespace, e.g. `documents.json` → `documents.*`), merged in `src/i18n/index.ts` via `import.meta.glob` (eager); parity test: same files per locale and identical merged key sets. Messages must not contain `{`, `@` or `\|` literally (vue-i18n syntax) |
| `src/lib/storage.ts` | try/catch-wrapped `localStorage` |
| `src/test-utils.ts` | fetch stubs for tests: single response, sequence, `mockFetchRoutes` by `"METHOD /path"` (a reply may be a raw `Response`, e.g. `pdfReply()`) |
| `src/style.css` | Tailwind + shared component classes (`.input`, `.btn*`, `.card`, `.table`, `.badge`, `.alert-error`) |

Dev: Vite proxies `/api` → `http://localhost:3000`.

## Build, CI, image

- `Dockerfile`: node:22 builds the SPA → `rust:1.97.0-bookworm` builds the release binary with the SPA and `design/` embedded (`--locked`) → `debian:bookworm-slim`, non-root uid 10001, `INVOICE__STORAGE_DIR=/data` (`VOLUME /data`, owned by 10001), `EXPOSE 3000`, `invoice serve`.
- `.github/workflows/ci.yml`: PR + push to `master`; backend job (Postgres service, `make lint-backend test-backend`, toolchain pin check), frontend job (`make frontend-install lint-frontend test-frontend frontend-build`).
- `.github/workflows/docker.yml`: `v*` tag → `ghcr.io/xmiksay/invoice:<version>` + `latest`.
- Deployment manifests are out of scope for now.
