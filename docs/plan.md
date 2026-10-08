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
| Document types | Invoice (tax document), credit note (references original), proforma; for VAT payers a tax document for a received advance payment (DDPP) after the proforma is paid, and "issue final invoice from proforma" deducting the advance and its VAT. |
| VAT | Rate per line, VAT computed from the per-rate recap (§37 ZDPH) rounded to 0.01. Modes: standard, reverse charge, exempt. Optional rounding of the payable total to whole CZK (rounding line). Foreign-currency invoices also show VAT in CZK. |
| Exchange rate | ČNB daily rate at the tax point date (received invoices: date of receipt), stored on the invoice, manually overridable; if ČNB is unreachable the user must enter it. |
| Numbering | Configurable pattern per document type, e.g. `{YYYY}{NNNN}`, yearly reset, number assigned at issue (drafts have none). Counter is manually settable in Settings. Imported invoices keep their own number and do **not** move the counter. |
| Lifecycle | `draft → issued → sent → paid`, `cancelled`. On issue the invoice is locked, supplier/customer snapshots are stored and the PDF is rendered and archived; issuing fails if mdcast is down. "Overdue" is derived, never stored. Payments: date + amount. |
| PDF | mdcast `/v1/render/template` (typst + JSON data). Default design embedded in the binary; `INVOICE__DESIGN_DIR` (sub-directory on the PVC) overrides it file by file (`brand.toml`, `invoice.typ`, fonts, logo). Preview endpoint renders a sample invoice. Invoice language cs/en per document. SPAYD QR payment code. |
| Received invoices | Metadata entered manually + original PDF upload. |
| File storage | Filesystem, `INVOICE__STORAGE_DIR` (PVC); DB keeps path + sha256. |
| Contacts | One address book for customers and suppliers, ARES lookup by IČO. Invoices store a snapshot, so editing/deleting a contact never changes an invoice. ISDOC import matches contacts by IČO or creates one. |
| Received invoices numbering | Internal evidence number from its own series (e.g. `P{YYYY}{NNNN}`) assigned on save, plus the supplier's original number and VS. |
| Payments | Multiple (partial) payments per document (date, amount, note). Status becomes `paid` automatically when payments cover the payable amount. A DDPP is issued per received advance payment. |

## Phases

Each phase is its own branch + PR into `master`. Phase 1 is split into four PRs:
1a settings + contacts + ARES, 1b documents + VAT + numbering + payments + ČNB,
1c PDF via mdcast + design dir + QR + archive, 1d received invoices + imports (PDF, ISDOC).

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

Phase 1b note:
- Setting a number-series counter below the highest already-issued number for that doc type/year must be rejected (needs the documents table).

### Phase 2 — interchange
ISDOC export, CSV/XLSX bulk import (fixed documented template, sample downloadable
in the UI, one row = one invoice with VAT recap, no lines), CSV export for the
accountant, e-mail sending via SMTP (manual button, prefilled cs/en template,
PDF + ISDOC attached, send log in DB, status → `sent`).

### Phase 3 — accounting & MCP
Pohoda XML (Stormware) and Money S3 XML export of issued + received invoices per
period; MCP endpoint (list/create/issue invoices) behind the same Bearer token.

## Out of scope (for now)
Deployment manifests, VAT return / control statement XML (EPO), automatic payment
reminders, multiple own companies per instance, Pohoda/Money S3 import.
