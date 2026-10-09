# API — ISDOC import and export (phase 1f-b)

ISDOC 6.0.2 (`http://isdoc.cz/namespace/2013`). This builds on the 1e manual import and the 1f-a document types.
Fixtures under `tests/fixtures/isdoc/` must be anonymized: no real IČO, DIČ, IBAN or names.

## Document type mapping (both directions)
| ISDOC `DocumentType` | docType |
|---|---|
| 1 | `invoice` |
| 2 | `credit_note` |
| 3 | `debit_note` |
| 4 | `proforma` |
| 5 | `advance_tax_doc` |
| 6 | `advance_credit_note` |
| 7 | `simplified` |

## Import — `POST /api/import/isdoc/preview` and `POST /api/import/isdoc/confirm`

### Upload
Both endpoints take `multipart/form-data` with one or more `files` parts (`.isdoc`, `.isdocx` or `.zip`).
`confirm` additionally takes a JSON part `options`:

```
{
  selected: string[],          // entry keys from the preview (status "ok" only)
  markPaid: boolean,           // default true
  categoryId: uuid | null,     // received only; an expense category, as in 1e
  vatDeductible: boolean       // received only; default true
}
```

The server keeps nothing between preview and confirm. The client re-sends the same files with `confirm`, and the
server parses them again.

### Limits
- 50 MiB total upload. Over the limit → 413 `too_large`.
- At most 500 documents. More → 422 `{"fields":{"files":"too_many"}}`.
- ZIPs are unpacked recursively, at most 3 levels deep.
- At most 200 MiB unpacked in total. Over the limit → 422 `{"fields":{"files":"too_large"}}`.

### Archive entries
- Inside a zip, `.isdoc`, `.isdocx` and `.zip` entries are processed; anything else is ignored.
- An `.isdocx` is a ZIP holding the ISDOC XML (the `*.isdoc` entry) and optionally a PDF. When it has exactly one PDF
  (or a `manifest.xml` names the main PDF), that PDF becomes the document's original (`documents/{year}/…`, ≤ 20 MiB
  as in 1e). A larger PDF is skipped, with a warning.

### Entry key
An entry key is the path inside the upload, e.g. `invoices.zip/2026.zip/faktura_1.isdoc`. Every document has a stable
key, so the client can echo it back in `selected`.

### Preview response
`200 { entries: PreviewEntry[] }`, in upload order:

```
PreviewEntry {
  key: string,
  status: "ok" | "duplicate" | "error",
  error: string | null,          // code, see below; set when status = "error"
  warnings: string[],            // codes, see below
  direction: "issued" | "received" | null,
  docType: string | null,
  number: string | null,         // ISDOC ID (issued: our number; received: supplier number)
  counterparty: { name: string, ico: string | null } | null,
  contactMatch: "existing" | "new" | null,
  issueDate, taxPointDate, dueDate,   // YYYY-MM-DD | null
  currency: string | null,
  total: string | null,          // the ISDOC's LegalMonetaryTotal/TaxInclusiveAmount (document currency)
  hasPdf: boolean,
  relatedNumber: string | null,  // OriginalDocumentReference ID
  relatedFound: boolean          // resolved in the DB or in this batch
}
```

### Error codes (an error entry is never imported)
- `invalid_xml`: not well-formed, or the root is not an ISDOC `Invoice`.
- `unsupported_version`: a version other than 6.x.
- `unsupported_type`: DocumentType outside 1–7.
- `foreign`: our company IČO matches neither the supplier (`AccountingSupplierParty`) nor the customer
  (`AccountingCustomerParty`).
- `ambiguous`: it matches both.
- `missing_field`: a required element is missing (ID, IssueDate, currency or the totals).
- `invalid_amount`: an amount cannot be parsed, or is ≥ 10^12.

### Warning codes
- `related_not_found`
- `pdf_skipped`
- `rate_from_cnb`: a foreign currency with no usable rate in the ISDOC.
- `contact_created`
- `paid_ignored`: `PaidAmount` is present; it is never used.

### Direction
- Our company's IČO equals the supplier's IČO → `issued`.
- It equals the customer's IČO → `received`.
- IČO values are compared with whitespace stripped.

### Duplicates (status `duplicate`, never imported)
- **Issued:** an issued document of the same docType with the same `number` already exists (drafts included).
- **Received:** a received document with the same supplier IČO (contact snapshot) and the same `supplierNumber`
  already exists. With no supplier IČO, the match is on supplier name + `supplierNumber`.
- A second identical key in the same batch is also a `duplicate` of the first.

### Field mapping
- **Dates:** `IssueDate` is the issue date. `TaxPointDate` falls back to `IssueDate` when missing, and is null for a
  proforma. `PaymentMeans/Payment/Details/PaymentDueDate` is the due date; missing → `issueDate`. A received
  `advance_tax_doc` may have no due date.
- **Currency:**
  - `ForeignCurrencyCode` set → the document currency is that code. The rate is `CurrRate / RefCurrRate` (≤ 6 dp),
    stored as `manual`, provided the rate is > 0 and ≠ 1.
  - Otherwise ČNB at the tax point date (received: the received date), with warning `rate_from_cnb`. ČNB down at
    confirm → that entry fails with `rate_unavailable`; other entries still import.
- **Amounts:**
  - `vatMode`: every `TaxSubTotal` at 0 % with `VATApplicable` false → `non_payer` for issued documents, `exempt`
    for received ones. Otherwise `standard`.
  - Reverse charge is detected from `VATApplicable` false with a non-zero rate, or from a note containing
    "přenesen", and maps to `reverse_charge`.
  - The VAT recap is copied verbatim from `TaxTotal/TaxSubTotal` (`TaxableAmount`, `TaxAmount`, `Percent`). In ISDOC
    the plain elements are in the local currency (CZK) and the `…Curr` elements are in the foreign currency. So for a
    foreign-currency document the document amounts come from `…Curr` and `baseCzk`/`vatCzk` from the plain elements.
    The same applies to the totals.
  - Totals are copied verbatim from `LegalMonetaryTotal`.
  - Rounding is `PayableRoundingAmount`, and `payable` is `PayableAmount`.
  - Credit notes and DDPP corrections store positive amounts (the absolute value of what the ISDOC carries).
- **Lines:**
  - Each `InvoiceLine` becomes an `item`: `Item/Description`, `InvoicedQuantity` (+ `unitCode` → unit),
    `UnitPrice`, `ClassifiedTaxCategory/Percent`, discount 0.
  - A line with no amounts becomes `text`.
  - The lines are **stored only**. The recap and totals above are never recomputed from them.
- **Counterparty:**
  - An existing contact with the same IČO is used. Otherwise a contact is created from the ISDOC party (name, IČO,
    DIČ, address, country; no ARES), with warning `contact_created`.
  - The snapshot is taken from the ISDOC party data as written, not from the contact.
- **Bank:**
  - Issued: the company bank account whose IBAN or account number matches `PaymentMeans/…/ID` / `IBAN`; else null.
  - Received: `supplierAccount` is the IBAN or account number.
- **Payment fields:** `VariableSymbol`, `ConstantSymbol`; `PaymentMeansCode` 42 → `bank_transfer`, 10 → `cash`,
  48 → `card`, otherwise `other`.
- **Note and locale:** `Note` becomes `headerNote` (issued) or `note` (received). Locale `cs`.
- **Related document:**
  - `OriginalDocumentReference/ID` is resolved against a non-cancelled document of the same direction, of a type
    allowed by the 1f-a link table:
    - issued: by `number`;
    - received: by `supplierNumber` + the same supplier IČO.
  - Documents earlier in the same batch count as well; the import is ordered so that originals come first.
  - Not found → no link, with warning `related_not_found`.

### Confirm
- Each selected `ok` entry is imported in its own transaction.
- **Issued entries:**
  - Created `imported: true`, already `issued`, with the number from the ISDOC.
  - They take no number from a series and get no PDF render; the snapshots come from the ISDOC.
- **Received entries:**
  - Created with an internal number from the received series of their type.
  - `receivedDate` = the tax point date. `categoryId` and `vatDeductible` come from `options`.
- **`markPaid`:**
  - One payment of `payable` on the due date (issue date if none), for payable types: invoice, debit_note,
    proforma, simplified, plus credit_note and advance_credit_note as refunds. A DDPP gets none.
  - Skipped when `payable ≤ 0`.
  - An imported proforma payment never creates a DDPP (1e).
- **Rules relaxed for imported documents (as in 1e):** caps and the `correctionReason` requirement are not enforced.
- **Response:** `200 { results: [{ key, status: "imported" | "skipped" | "failed", documentId: uuid|null,
  error: string|null }] }`.
  - `skipped`: a key that was not selected, or that is no longer `ok` (e.g. it became a duplicate meanwhile).
  - `failed`: carries an error code; anything internal is logged and returned as `internal`.

## Export
### `GET /api/documents/{id}/isdoc`
- Only for an issued (non-draft) document with direction `issued`, cancelled ones included. Otherwise → 409
  `invalid_state`; a received document → 404.
- The response is an `.isdocx` (`application/zip`). The ZIP holds:
  - `{number}.isdoc`, the XML;
  - `{number}.pdf`, which is the archived PDF for native documents (rendered lazily, as for a DDPP in 1d) and the
    original for imported ones;
  - `manifest.xml`, per the ISDOC isdocx spec, naming both files.
- An imported document without an original, or a native one whose PDF cannot be rendered (mdcast down), gets a
  plain `.isdoc` (`application/xml`) instead, never a 5xx.
- Filename: `{number}.isdocx` / `{number}.isdoc`.

### `GET /api/documents/isdoc?{list filters}`
- Takes the same filters as `GET /api/documents`, with `direction=issued` forced. Drafts are always excluded.
- The response is a ZIP of the per-document files described above (`isdoc-export.zip`).
- More than 1000 matching documents → 422 `{"fields":{"filter":"too_many"}}`.
- A native document whose archive is missing and cannot be rendered is exported as plain `.isdoc`.

### XML content
- **Header:**
  - `DocumentType` from the mapping table; `UUID` = document id; `ID` = number; `IssueDate`.
  - `TaxPointDate` (omitted for a proforma).
  - `VATApplicable` = the supplier snapshot's `vatPayer`.
  - `ElectronicPossibilityAgreementReference` is empty.
  - `LocalCurrencyCode` CZK.
  - `ForeignCurrencyCode` + `CurrRate` / `RefCurrRate` (1) when the currency is not CZK.
- **Parties:** `AccountingSupplierParty` / `AccountingCustomerParty` from the snapshots. A simplified document
  without a customer omits the customer party.
- **Lines:**
  - `item` lines are written with the full price breakdown.
  - `text` lines become `InvoiceLine` entries with zero amounts.
  - `subtotal` lines are omitted (the members are exported).
  - `advance` lines are written as `NonTaxedDeposits` / `TaxedDeposits` per the ISDOC spec.
- **Totals:** `TaxTotal` from the stored recap (with the CZK amounts for foreign currency). `LegalMonetaryTotal` from
  the stored totals; `PayableRoundingAmount` = rounding.
- **Payment:**
  - `PaymentMeans`: due date, VS, KS, and the IBAN + BIC / account number from the bank snapshot.
  - The `PaymentMeansCode` reverse map, with `other` → 97.
- **References and notes:**
  - `OriginalDocumentReference` for a credit note, debit note or DDPP correction, giving the original's number and
    issue date.
  - `Note` = header note; the correction reason goes into `Note` as well.
- **Signs:** credit notes and DDPP corrections are exported with the sign convention the ISDOC 6.0.2 spec prescribes
  for document types 2 and 6. Check it against the spec text and record the result in Clarifications. On import, the
  absolute values are stored.
- **Validation:** the output must validate against the ISDOC 6.0.2 XSD. Tests check this with a vendored copy of the
  schema in `tests/fixtures/isdoc/schema/`, if its licence allows; otherwise with structural assertions.
- **Round trip:** import(export(doc)) must reproduce the totals, the recap, the lines (items/text), the dates, the
  parties' IČO and the number. A test checks it on our own exports.

## Frontend
- **Import page:** opened from the issued and received lists by an "Import ISDOC" button.
  - A drop zone takes the files.
  - Preview table: status, direction, type, number, counterparty (with a new/existing badge), dates, total, PDF,
    warnings.
  - Checkboxes, with the `ok` rows selected by default.
  - Batch options: mark as paid (on); category and VAT deductible for received documents.
  - Confirm, then a result table with links to the imported documents.
- **Detail:** a "Stáhnout ISDOC" button on issued non-draft documents.
- **Lists:** "Exportovat ISDOC" on the issued list exports the current filter as one ZIP.
