# Invoice — agreed plan

Single-user invoice management: issued and received invoices, metadata, Czech VAT,
PDF rendering through the mdcast service (`xmiksay/mdcast`) with a per-instance design.

Domain logic is ported and simplified from `nexial/infra/src/invoice` (no tenants,
no OAuth, no stock/task/cost-center links). Module layout follows the infra
"Service Module Pattern": `mod.rs` (`Deps`, `connect`, `router`), `config.rs`,
`error.rs`, `openapi.rs`, `entity/`, `repo/`, `handlers/`.

## Decisions

| Topic | Decision |
|---|---|
| Users | Single user. One static Bearer token `INVOICE__API_TOKEN`, constant-time compare. SPA asks for it on a login screen and keeps it in `localStorage`. Same token for MCP. |
| Binary | One crate, one binary: `invoice serve` (runs pending migrations on start), `invoice migrate up\|status\|down`. |
| Frontend | Vue 3 + TS + Vite + Tailwind + Pinia + vue-i18n (cs/en), embedded via rust-embed. |
| Scope | Czech OSVČ / s.r.o., one own company per instance, CZK + foreign currencies. |
| Own company & settings | In DB, edited in the UI: company profile, VAT payer flag, bank accounts per currency, VAT rates (seeded 21/12/0, user-editable), number series. |
| Document types | Invoice (tax document), credit note and debit note (reference the original), simplified tax document, proforma; for VAT payers a tax document for a received advance payment (DDPP) after the proforma is paid (and its correction), and "issue final invoice from proforma" deducting the advance and its VAT. |
| VAT | Rate per line, VAT computed from the per-rate recap (§37 ZDPH) rounded to 0.01. Modes: standard, reverse charge, exempt. Optional rounding of the payable total to whole CZK (rounding line). Foreign-currency invoices also show VAT in CZK. |
| Exchange rate | ČNB daily rate at the tax point date (received invoices: date of receipt), stored on the invoice, manually overridable; if ČNB is unreachable the user must enter it. |
| Numbering | Configurable pattern per document type, e.g. `{YYYY}{NNNN}`, yearly reset, number assigned at issue (drafts have none). Counter is manually settable in Settings. Imported invoices keep their own number and do **not** move the counter. |
| Lifecycle | `draft → issued → sent → paid`, `cancelled`. On issue the invoice is locked, supplier/customer snapshots are stored and the PDF is rendered and archived; issuing fails if mdcast is down. "Overdue" is derived, never stored. Payments: date + amount. |
| PDF | mdcast `/v1/render/template` (typst + JSON data). Default design (minimalist, Inter) embedded in the binary; storage keys `design/…` override it file by file (`invoice.typ`, fonts, logo, signature). Preview of a sample invoice in Settings → Design. Invoice language cs/en per document. SPAYD QR payment code. Details: [api/pdf.md](api/pdf.md). |
| Received invoices | Metadata entered manually + original PDF upload. |
| File storage | `Storage` over `object_store`: filesystem (`INVOICE__STORAGE_DIR`, PVC) or S3-compatible bucket (#9); DB keeps key + sha256. |
| Contacts | One address book for customers and suppliers, ARES lookup by IČO. Invoices store a snapshot, so editing/deleting a contact never changes an invoice. ISDOC import matches contacts by IČO or creates one. |
| Received invoices numbering | Internal evidence number from its own series (e.g. `P{YYYY}{NNNN}`) assigned on save, plus the supplier's original number and VS. |
| Payments | Multiple (partial) payments per document (date, amount, note). Status becomes `paid` automatically when payments cover the payable amount. A DDPP is issued per received advance payment. |

## Phases

Each phase is its own branch + PR into `master`. Phase 1 is split into seven PRs:
- 1a settings + contacts + ARES (done)
- 1b document core: `documents` table, invoice lines (item / text / subtotal with collapse, % discount), VAT recap + rounding, ČNB rates, issue (numbering, snapshots), cancel, payments, counter guard
- 1c proforma → DDPP (auto on payment) → final invoice settlement, credit notes, catalog (items + groups)
- 1d PDF via mdcast + design dir + QR + archive on issue
- 1e received documents (all four types, VAT recap only) + original PDF upload + manual import of issued documents + categories + custom fields
- 1f-a debit notes, DDPP corrections, simplified tax documents (all seven ISDOC document types, both directions)
- 1f-b bulk ISDOC import (`.isdoc`/`.isdocx`, zip-in-zip, preview → confirm) + ISDOC export (moved from phase 2)

### Phase 1 — core
Settings, contacts + ARES, all document types incl. DDPP and proforma settlement,
VAT recap, ČNB rates, numbering, lifecycle + payments, PDF via mdcast with design
dir override + QR, received invoices with PDF upload, import of issued invoices
(manual PDF + metadata; bulk `.isdoc`/`.isdocx`).

ISDOC import notes (FakturaOnline export, ISDOC 6.0.2):
- The export is a **zip inside a zip** — unpack recursively.
- `TaxPointDate` may be missing (non-VAT payer) — fall back to `IssueDate`.
- `PaymentMeans/Payment/PaidAmount` equals the total even for unpaid invoices — do not treat it as a payment.
- Test fixtures must be anonymized (no real IČO/DIČ/IBAN/names).

Document decisions (1b/1c):
- One `documents` table with `direction` (`issued` / `received`); received documents carry only a VAT recap, issued ones have lines and a stored recap computed from them.
- Lines: `item` (description, quantity, unit, unit price excl. VAT, % discount, VAT rate), `text` (no amounts), `subtotal` (ported from infra: references other lines by position, nested allowed, no cycles, all members share one VAT rate; `collapse` shows only the subtotal on the PDF). Catalog groups (1c) insert their members + a collapsed subtotal.
- Credit notes are entered with **positive** amounts; the sign comes from the document type (reports/exports negate).
- Issued documents are locked: only payments, mark-sent, cancel, credit note and the internal note change afterwards. Drafts can be deleted.
- ČNB rate is fixed at issue for the tax point date (latest published on or before it) unless entered manually; cached in `exchange_rates`; ČNB down + no manual rate → issue fails with 422.
- Proforma payments by a VAT payer automatically issue a DDPP (tax point = payment date, VAT from above, split proportionally by the proforma's rates); the final invoice deducts DDPPs per rate (non-payer: deducts the paid amount).
- Deleting a proforma payment cancels its DDPP (reason recorded); refused once that DDPP is deducted by an issued final invoice.
- Foreign-currency proforma payments take an optional manual rate; without it the ČNB rate for the payment date is used, and if ČNB is down the payment is rejected (422) so no DDPP is half-created.
- Credit notes: only for an issued (not cancelled) invoice, correction reason required, tax point = correction date, exchange rate copied from the original invoice, and all non-cancelled credit notes together may not exceed the original's base per VAT rate.
- Catalog items carry name, unit, price, currency and VAT rate; inserting into a document in another currency leaves the price empty (no conversion). Catalog groups = name + member items with quantities; inserting adds the members plus a collapsed subtotal.
- Setting a number-series counter below the highest number already issued from that series/year is rejected.

PDF decisions (1d):
- Drafts can be downloaded as a live render with a "NÁVRH / DRAFT" watermark (no QR, never stored).
- The PDF archived at issue is immutable — never re-rendered (not after cancel, not after a design change).
- Issuing fails (nothing changes) when mdcast is unavailable; an auto-issued DDPP does not block the payment — its PDF is rendered right after the payment, or on first download.
- SPAYD QR only on invoices and proformas paid by bank transfer with an IBAN and a positive payable amount (any currency).
- `INVOICE__MDCAST_URL` defaults to `https://mdcast.nexial.cz`.

Received & import decisions (1e/1f):
- Received documents mirror the issued types (invoice, credit note, proforma, DDPP), each with its own internal number series; no draft — saved = recorded, editable and deletable at any time. Links between a received proforma, its DDPP and the final invoice are informational only (amounts as on the supplier's documents).
- Received metadata: supplier number, received date (ČNB rate date), VAT deductible flag, category, note, custom fields. Original PDF optional, can be uploaded/replaced later.
- Categories: one list with kind expense/income. Custom fields defined in Settings (text/number/date/bool/select, for issued/received/both), stored in `custom_fields jsonb`.
- Manual import of issued documents: any of the four types, full line editor with its own number, issued without number allocation or PDF render; the original PDF can be uploaded. Imported documents never get our rendered PDF.
- ISDOC (1f-b): direction by the supplier IČO vs. company IČO; lines stored, recap/totals taken verbatim from the ISDOC; preview → confirm; duplicates skipped; "mark as paid" option (payment of the full amount on the due date, default on). Per-document outcome: confirm imports only the OK rows, each in its own transaction, and reports per file. Foreign-currency rate from the ISDOC (`CurrRate`/`RefCurrRate`), else ČNB. Batch options for received documents: category (optional), VAT deductible (default yes), received date = tax point date; edited per document afterwards. Limits 50 MiB upload / 500 documents / 3 zip levels / 200 MiB unpacked; confirm re-sends the files (stateless). `OriginalDocumentReference` is linked when found (DB or same batch). Export: issued non-draft documents as `.isdocx` (ISDOC + archived PDF or original), from the detail and in bulk from the list filter (≤ 1000). Details: [api/isdoc.md](api/isdoc.md).

Document type decisions (1f-a, details in [api/doc-types.md](api/doc-types.md)):
- Debit note (`V…`): created empty from an issued invoice / simplified document, reason required, original's rate, payable with QR; issued debit notes raise the credit-note cap.
- DDPP correction (`OP…`): created from an issued DDPP like a credit note (lines copied, cap = DDPP base per rate); the final invoice deducts the DDPP net of its corrections; refused once the DDPP is deducted.
- Simplified tax document (`ZD…`): its own type and series; customer optional; > 10 000 CZK only warns in the UI; credit and debit notes allowed, no advances.
- Received counterparts `PV…`, `POP…`, `PZD…`; manual import accepts all seven types.

### Phase 2 — interchange
Order: storage abstraction (#9, pulled forward) → 2a e-mail → 2b CSV/XLSX import → 2c CSV export, one PR each.

Storage (#9, before 2a):
- `Storage` over the `object_store` crate: put (sha256, atomic), get (stream), delete, exists, list by prefix. Backend by
  `INVOICE__STORAGE_KIND` = `fs` (default, `INVOICE__STORAGE_DIR`) or `s3` (`INVOICE__S3__ENDPOINT`, `BUCKET`, `REGION`,
  `ACCESS_KEY_ID`, `SECRET_ACCESS_KEY`, `PATH_STYLE`); the Docker image works without S3.
- Everything goes through it: PDF archive, originals, e-mail templates (2a) **and the design** (keys `design/…`, overlaid
  on the embedded default file by file as before). `INVOICE__DESIGN_DIR` is removed; files reach storage via
  `invoice design push|pull|ls` or straight into the bucket (UI comes with #10). Keys stay as today
  (`documents/{year}/…`); the `spaces/{id}/` prefix arrives with #3.
- Design reads list the `design/` prefix per render and cache file contents by key + ETag/size, so unchanged fonts are
  not downloaded again.
- Storage unreachable → 503 `storage_unavailable` (issue fails and changes nothing, like mdcast down).
- `invoice storage migrate --from-dir <path> [--design-dir <path>]` copies the fs archive (and an old design dir) into
  the configured backend, verifies sha256, is idempotent and never deletes the source.
- Tests: one shared suite against fs and S3. S3 = test bucket `invoice-test` on our Garage (`s3.mmik.cz`, path-style,
  region `garage`), `TEST_S3_*` in `.env` locally and GitHub secrets in CI; every test uses a random prefix and cleans
  up. Fails, not skips, without it.
- As implemented: `invoice design rm <path>…` added next to push / pull / ls; `migrate` never overwrites an object with
  different content (reported, exit 1); the design cache compares the listed version (ETag + size + modification
  time); `GET /api/pdf/design` returns `storage: "fs" | "s3"` instead of `designDir`; an ISDOC export whose PDF cannot
  be read because the storage is down fails with 503 instead of silently leaving the PDF out (the bulk export stops at
  the first such document); a leftover `INVOICE__DESIGN_DIR` refuses the start; `migrate` / `design push` decide
  "identical" by size + MD5 ETag where the backend offers one (full download otherwise) and report unreadable local
  entries (exit 1); a custom endpoint without path-style gets the bucket put into its host; an ISDOC import confirm whose
  storage is down fails that entry with `storage_unavailable` (like `rate_unavailable`).

2a e-mail (SMTP via `lettre`):
- SMTP from env only (`INVOICE__SMTP__HOST/PORT/USERNAME/PASSWORD/FROM/TLS`); Settings shows configured yes/no and
  sends a test e-mail.
- Send dialog: To (prefilled from the contact, several addresses), Cc, Bcc, optional Bcc to the company e-mail
  (ticked), subject and body rendered from the template and editable before sending; attachments PDF and plain
  `.isdoc` (checkboxes). Any issued, non-draft document of ours (cancelled too); never received ones.
- Templates: MiniJinja, plain text, subject + body per locale (cs/en), defaults embedded, overrides stored in Storage;
  Settings → E-mail editor with a preview on a sample document and "restore default". Saving compiles and renders the
  template on the sample in strict mode; an error → 422 with line + message, the active template stays.
- Sending is synchronous (30 s timeout); every attempt is logged (to/cc/bcc, subject, time, ok/error, Message-ID);
  success sets `sentAt`; re-sending allowed; the history is shown on the document detail.

2b CSV/XLSX import: one template for both directions and all seven types, one row = one document, VAT recap in
per-rate columns (`base_21`, `vat_21`, `base_12`, `vat_12`, `base_0`), contacts matched by IČO or created, sample
downloadable in the UI, preview → confirm like ISDOC.

2c CSV export for the accountant: issued and received per the list filter, one row = one document, `;` separator,
UTF-8 with BOM, decimal comma, dates `dd.mm.yyyy`; columns direction, type, number, supplier number, dates,
counterparty + IČO/DIČ, currency, rate, base/VAT per rate in CZK, total, paid, category; credit notes negative.

### Phase 3 — accounting & MCP
Pohoda XML (Stormware) and Money S3 XML export of issued + received invoices per
period; MCP endpoint (list/create/issue invoices) behind the same Bearer token.

## Out of scope (for now)
Deployment manifests, VAT return / control statement XML (EPO), automatic payment
reminders, multiple own companies per instance, Pohoda/Money S3 import.
