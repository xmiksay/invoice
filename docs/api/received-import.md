# Received documents, categories, custom fields, manual import (phase 1e)

Bulk ISDOC import is phase 1f (separate part). Everything here extends the existing `documents` API
([documents.md](documents.md)); unchanged rules (payments, payment state, overdue, list paging) apply to received
documents too.

## Received documents (`direction = "received"`)

Doc types: `invoice`, `credit_note` (sign −1), `proforma`, `advance_tax_doc` — the same `docType` values as issued
documents, distinguished by `direction`. No lines: the supplier's amounts are entered as a **VAT recap**.

### Lifecycle
- No draft: `POST /api/documents` with `direction: "received"` stores the document with `status: "issued"` (UI label
  "Zaevidováno" / "Recorded") and allocates the internal number from the series of its doc type in the same
  transaction (year = `receivedDate` year).
- Editable at any time with `PUT /api/documents/{id}` (the internal number never changes, also not when the year of
  `receivedDate` changes). `DELETE` allowed at any time (the number is not returned to the series; payments and the
  stored original PDF are deleted with it).
- `cancel`, `mark-sent`, `issue`, `settle`, `credit-note` on a received document → 409 `invalid_state`.
- Payments work as for issued documents (outgoing payments). Payments on a received proforma **never** create a DDPP.

### Number series
New series (seeded, editable in Settings like the others; `docType` keys of `/api/settings/number-series`):
| key | received doc type | seed pattern |
|---|---|---|
| `received` | invoice | `P{YYYY}{NNNN}` (existing) |
| `received_credit_note` | credit_note | `PD{YYYY}{NNNN}` |
| `received_proforma` | proforma | `PZ{YYYY}{NNNN}` |
| `received_advance_tax_doc` | advance_tax_doc | `PDP{YYYY}{NNNN}` |
The counter guard and "no duplicate pattern" rule apply unchanged. Internal numbers are unique per (direction,
docType).

### Wire (Document, received)
Fields beyond / instead of the issued shape:
```
direction: "received",
docType: "invoice" | "credit_note" | "proforma" | "advance_tax_doc",
number: string,                       // internal evidence number (read-only)
supplierNumber: string,               // required, <=40, the supplier's own document number
contactId: uuid,                      // required: the supplier (from the shared address book)
issueDate: date,                      // required
taxPointDate: date | null,            // required except proforma (must be null for proforma)
receivedDate: date,                   // required; default (client + server) = taxPointDate ?? issueDate
dueDate: date | null,                 // required except advance_tax_doc
currency, exchangeRate, exchangeRateDate, exchangeRateSource,   // as issued; ČNB rate for receivedDate,
                                      // fixed on create/when currency or receivedDate changes, manual override wins
vatMode: "standard" | "reverse_charge" | "exempt" | "non_payer",   // non_payer = supplier is not a VAT payer
vatRecap: [ { rate: string, base: string, vat: string } ],   // >=1 row, rate unique, entered as on the document
                                      // (positive for credit notes); vat must be 0 unless vatMode standard
rounding: string,                     // default "0", entered as on the document, |rounding| < 100
total: string,                        // read-only = Σ(base+vat) + rounding
payable: string,                      // required, entered (e.g. a final invoice after advances), >= 0
vatDeductible: boolean,               // default true when vatMode = standard, else false
variableSymbol, constantSymbol: string | null,
supplierAccount: string | null,       // <=60, free text (IBAN or CZ account) to pay to
relatedDocumentId: uuid | null,       // informational link: received DDPP / final invoice → received proforma,
                                      // received credit note → received invoice; must be a received document
categoryId: uuid | null, customFields: object, internalNote: string | null,
original: { sha256, size, uploadedAt } | null,   // uploaded original PDF
```
`totalCzk` / recap CZK computed from the rate as for issued documents. `supplier` snapshot = the contact at the last
save (refreshed on every PUT). `relatedDocuments` lists the link in both directions.

422 field keys for recap rows: `vatRecap.N.rate|base|vat` (0-based). Unknown rate value (not in Settings VAT rates) is
**allowed** for received documents (foreign VAT, old rates); rate 0..100, 2 dp.

## Original PDF (received and imported issued documents)
- `PUT /api/documents/{id}/original` — `multipart/form-data`, one part `file`; must start with `%PDF-`, ≤ 20 MB
  (413 `{"code":"too_large"}`; other → 422 `{"fields":{"file":"invalid"}}`). Replaces any previous original. Stored
  at `documents/{year}/{id}-original.pdf` under `INVOICE__STORAGE_DIR` (same temp+rename+sha256 as the archive).
  Allowed for every received document and for imported issued documents (any status); a non-imported issued
  document → 409 `invalid_state` (its PDF is the rendered archive).
- `DELETE /api/documents/{id}/original` → 204 (same rules).
- `GET /api/documents/{id}/pdf` for received / imported documents serves the original; none → 404
  `{"code":"pdf_missing"}`. Never rendered with our design.
- `pdf` (rendered archive) stays `null` for received / imported documents; `original` is `null` for native issued ones.

## Manual import of issued documents (`imported = true`)
- Any of the four issued doc types. Created as a **draft** with `imported: true` and a required `number` (<=40,
  unique per issued docType → 422 `{"fields":{"number":"duplicate"}}`); full line editor and computation as native
  drafts. Optional `relatedDocumentId` (DDPP / final invoice → proforma, credit note → invoice) instead of the
  native settlement/credit flows; deduction lines (`advance`) are not available.
- `POST /api/documents/{id}/issue` on an imported draft: same validation, snapshots and ČNB rules, but **no number
  allocation** (keeps its `number`; `numberYear` = issueDate year, `numberSeq` null) and **no PDF render**. Payments
  on an imported proforma never create a DDPP. The counter guard ignores imported documents (unchanged).
- Body of create/PUT for issued drafts gains `imported: boolean` (default false; immutable after create → 422
  `{"fields":{"imported":"invalid"}}` if changed) and `number` (required iff imported, otherwise must be absent/null).
- A native issue whose allocated number collides with an imported one → 409 `{"code":"number_taken"}` (set the
  counter in Settings).

## Categories — `/api/settings/categories`
```
Category { id, name: string /* required <=100, unique per kind */, kind: "expense" | "income", active: boolean, position: number }
```
`GET` (ordered kind, position, name), `POST`, `PUT /{id}`, `DELETE /{id}` (204; used by a document → 409
`{"code":"category_in_use"}` — deactivate instead). Documents: `categoryId` optional; received documents accept only
`expense`, issued only `income` (else 422 `categoryId: invalid`); an inactive category stays on documents that have it
but cannot be newly assigned (422 `categoryId: inactive`).

## Custom fields — `/api/settings/custom-fields`
```
CustomField { id, key: string /* ^[a-z][a-z0-9_]{0,39}$, unique, immutable */, label: string /* <=100 */,
  type: "text" | "number" | "date" | "bool" | "select", options: string[] /* select only, 1..50 unique, each <=100 */,
  appliesTo: "issued" | "received" | "both", required: boolean, active: boolean, position: number }
```
`GET`, `POST`, `PUT /{id}` (key and type immutable → 422 `invalid`), `DELETE /{id}` (204; values stay in documents'
JSON and are ignored).
- Documents carry `customFields: { [key]: value }` (`documents.custom_fields jsonb NOT NULL DEFAULT '{}'`). Values:
  text string <=500, number decimal string, date `YYYY-MM-DD`, bool boolean, select one of `options`; `null` /
  missing = empty. Validation on every save against the **active** definitions that apply to the document's
  direction: unknown key → 422 `customFields.<key>: unknown`; wrong type → `invalid`; required + empty → `required`
  (required is enforced on create/PUT and — for issued drafts — at issue). Keys of inactive/deleted definitions
  already stored are kept untouched on PUT if sent back unchanged.

## Metadata after issue
`PUT /api/documents/{id}/metadata` body `{ categoryId, customFields, internalNote }` → Document; allowed in every
status for both directions (replaces the 1b `PUT …/internal-note`, which stays as an alias). Same validation.

## List
`GET /api/documents?direction=received` (existing filter) + new filters `categoryId=`, `imported=true|false`. List
items gain `supplierNumber`, `categoryId`, `hasPdf` (rendered archive or original present). `q` also matches
`supplierNumber`.

## Errors (new)
- 404 `pdf_missing`, 413 `too_large`, 409 `category_in_use`, 409 `number_taken`.
