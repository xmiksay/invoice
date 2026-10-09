# CSV / XLSX interchange (phases 2b import, 2c export)

One tabular format, shared by the import (2b) and the accountant export (2c), so an export imports back unchanged.
One row = one document (no lines); amounts are the VAT recap per rate.

## Format

- **CSV as written by the export:** separator `;`, UTF-8 with BOM, CRLF, RFC 4180 quoting (`"` doubled), decimal
  comma without thousands separators (`1234,50`), dates `dd.mm.yyyy`, empty cell = no value.
- **What the import also accepts:**
  - Encoding: UTF-8 (BOM optional); a file that is not valid UTF-8 is decoded as Windows-1250.
  - Separator: detected from the header line (`;`, `,` or TAB, the one that yields the most known columns).
  - Numbers: decimal comma or dot; spaces / NBSP as thousands separators are ignored; `1.234,50` and `1,234.50` are
    accepted (the last `,`/`.` is the decimal mark when both occur).
  - Dates: `d.m.yyyy` (with or without spaces after the dots) and `yyyy-mm-dd`.
  - **XLSX:** the first worksheet (crate `calamine`), row 1 = header; numeric cells are numbers, date cells
    (Excel serials) are dates, text cells parse as in CSV.
  - Header names are matched case-insensitively with surrounding whitespace trimmed; unknown columns are ignored;
    column order is free. Completely empty rows are skipped.
- **Header names** are fixed machine names (English, snake_case) — they are the contract, not UI text.

### Columns

| Column | Meaning | Import |
|---|---|---|
| `direction` | `issued` \| `received` | required |
| `doc_type` | `invoice` \| `proforma` \| `credit_note` \| `advance_tax_doc` \| `debit_note` \| `advance_credit_note` \| `simplified` | required |
| `number` | issued: our number; received: our internal number | issued: required; received: ignored (a new internal number is allocated from the series) |
| `supplier_number` | received: the supplier's document number | received: required |
| `related_number` | corrections / settlement: the original's number (issued: our number, received: supplier number) | optional, linked if found |
| `issue_date` | | required |
| `tax_date` | DUZP | optional; empty → `issue_date` (proforma: always null) |
| `due_date` | | optional; empty → `issue_date` (received DDPP may stay empty) |
| `received_date` | received only | optional; empty → `tax_date` |
| `counterparty_name` | customer (issued) / supplier (received) | required except `simplified` |
| `counterparty_ico`, `counterparty_dic` | | optional |
| `counterparty_street`, `counterparty_city`, `counterparty_zip`, `counterparty_country` | country ISO alpha-2 | optional (country default `CZ`) |
| `currency` | ISO 4217 | optional, default `CZK` |
| `exchange_rate` | CZK per 1 unit, ≤ 6 dp | required when `currency` ≠ CZK, must be empty / `1` for CZK |
| `vat_mode` | `standard` \| `reverse_charge` \| `exempt` \| `non_payer` | optional, default `standard` |
| `base_{rate}`, `vat_{rate}` | VAT recap per rate, **in CZK** | see below |
| `rounding` | in the document currency | optional, default 0, \|x\| < 100 |
| `total` | total incl. VAT + rounding, **in the document currency** | required |
| `total_czk` | total in CZK | ignored on import (export only) |
| `paid_date` | date of full payment | optional |
| `variable_symbol` | digits ≤ 10 | optional |
| `vat_deductible` | received: `1`/`0`, `ano`/`ne`, `true`/`false`, `yes`/`no` | optional, default per 1e (true for `standard`) |
| `category` | category name | optional |
| `note` | issued: `headerNote`, received: `note` | optional |

**Rate columns** are dynamic: the export writes `base_{r}` and `vat_{r}` for every VAT rate in Settings → VAT rates
(active and inactive), ordered by rate descending; for rate 0 only `base_0`. `{r}` is the rate with the decimal point
written as `_` and trailing zeros dropped (`21`, `12`, `0`, `12_5`). The import recognises any `base_{r}` / `vat_{r}`
header, also rates not in Settings (as received documents allow); a `vat_{r}` without its `base_{r}` column is a file
error. A rate takes part in a row when its base or VAT cell is non-empty.

**Signs:** credit notes and DDPP corrections (`credit_note`, `advance_credit_note`) are written **negative** (all
amount columns); the import takes their absolute value. Every other type must be ≥ 0 (`invalid_amount`).

## Import — `POST /api/import/csv/preview` and `POST /api/import/csv/confirm` (2b)

- `multipart/form-data`, one part `file` (`.csv`, `.txt` or `.xlsx`, detected by content: ZIP magic → XLSX).
  `confirm` adds a text part `options` = JSON `{ selected: string[] }` (keys from the preview, status `ok` only).
  Stateless like ISDOC: the client re-sends the file with confirm, the server parses it again.
- **Limits** (as ISDOC): 50 MiB → 413 `too_large`; more than 500 data rows → 422 `{"fields":{"file":"too_many"}}`.
- **File errors** → 422 `{"code":"validation","fields":{"file":"<reason>"}}`: `invalid` (unreadable / not CSV or
  XLSX / XLSX without a sheet), `empty` (no data rows), `missing_column` (a required column is absent; `detail` names
  it, e.g. `"detail":"supplier_number"`), `invalid_column` (`vat_{r}` without `base_{r}`, or a rate header that does
  not parse; `detail` names it).
- **Entry key** = `row:{n}`, `n` = the 1-based line / sheet row number (header = 1).

### Preview response
`200 { entries: PreviewEntry[] }` in file order, the same shape as the ISDOC preview
([isdoc.md](isdoc.md#preview-response)) with `hasPdf: false` always, plus:

```
  row: number,
  field: string | null,        // the column an error refers to (camelCase-free: the CSV header name)
  categoryMatch: "existing" | "new" | null
```

### Row errors (`status: "error"`, never imported)
- `missing_field` (`field` set), `invalid_value` (`field` set; unknown direction / doc_type / vat_mode, bad
  country / currency / VS / boolean), `invalid_date`, `invalid_amount` (unparsable, ≥ 10^12, wrong sign, > 2 dp for
  amounts / > 6 dp for the rate), `no_vat_rows` (no rate column filled), `total_mismatch` (see Amounts),
  `not_allowed` (combinations the domain forbids: proforma with `tax_date`, `simplified` received without
  `counterparty_name`, VAT on a non-`standard` row, `paid_date` on an `advance_tax_doc`).

### Warnings
`contact_created`, `category_created`, `category_inactive` (an inactive category matched by name → imported without a
category), `related_not_found`.

### Duplicates (`status: "duplicate"`)
Exactly as ISDOC: issued — same docType + `number`; received — same supplier IČO (else name) + `supplier_number`;
a repeat inside the file is a duplicate of the earlier row.

### Mapping
- **Counterparty:** an existing contact matched by IČO, else by DIČ, else by exact name (case-insensitive, trimmed);
  none → a contact is created from the row (warning `contact_created`, `contactMatch: "new"`). The document snapshot
  is taken from the row as written. `simplified` without a name → no contact (as 1f-a). Our own IČO as the
  counterparty → `invalid_value` on `counterparty_ico`.
- **Category:** matched by name case-insensitively within the kind of the direction (issued `income`, received
  `expense`); none → created (active, appended last; warning `category_created`).
- **Amounts:**
  - The recap is stored in CZK as given (`baseCzk` / `vatCzk`).
  - **CZK document:** document recap = the CZK recap; `Σ(base + vat) + rounding` must equal `total` exactly, else
    `total_mismatch`.
  - **Foreign currency:** document recap per rate = `round(czk / exchange_rate, 2)` (half away from zero). The
    difference `total − rounding − Σ(base + vat)` is added to the VAT of the highest rate with VAT (else to the base of
    the highest rate) when |difference| ≤ 0.01 × the number of recap rows; larger → `total_mismatch`.
  - `payable` = `total` (imports carry no advance deductions); stored rate source `manual`.
- **Issued rows:** created `imported: true`, status `issued`, number from the row, no number allocation, no PDF render,
  no lines (recap-only, like an ISDOC import without lines); supplier snapshot = current company; bank account = the
  default one of the currency (none → null); payment method `bank_transfer`; locale = the company default.
- **Received rows:** internal number from the received series of the type; `receivedDate`, `vatDeductible`,
  `supplierAccount` null.
- **Related document:** `related_number` resolved as in ISDOC (same direction, type allowed by the 1f-a link table,
  rows earlier in the file count; originals are imported first); not found → warning, no link.
- **`paid_date`:** one payment of `payable` on that date (credit notes: a refund), skipped when `payable` is 0;
  an imported proforma payment never creates a DDPP.
- Imported documents relax the 1e rules (caps, `correctionReason`) as ISDOC does.

### Confirm
Each selected `ok` row in its own transaction, response as ISDOC:
`200 { results: [{ key, status: "imported" | "skipped" | "failed", documentId, error }] }`.

### Sample — `GET /api/import/csv/sample`
`200 text/csv` (`Content-Disposition: attachment; filename="import-sample.csv"`), written exactly as the export:
the header with the current rate columns and three example rows (an issued CZK invoice, a received EUR invoice with a
rate, an issued credit note with negative amounts), using fictitious parties.

## UI (2b)
Issued and received list → "Import CSV / XLSX" (next to the ISDOC import): file picker, "Download sample", preview
table (row, status, direction, type, number, counterparty + contact match, dates, total, category match, error /
warnings with the column), select all `ok`, Confirm, result table with links. Reuses the ISDOC import components where
they fit.

## Export (2c)
Specified in phase 2c.

## Clarifications (as implemented)
