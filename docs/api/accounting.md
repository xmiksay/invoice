# Accounting exports: Pohoda XML (3b), Money S3 XML (3c)

Exports for the accountant's program, chosen as a **format** of the 2c accountant export
([csv.md](csv.md#get-apiexportaccountantfromyyyy-mm-ddtoyyyy-mm-dddirectionissuedreceivedboth--for-the-accountant)):
same period (tax date), same direction choice, same document set (no drafts, no cancelled, no proformas), same
10 000 cap, same consistent snapshot and skip-and-log of a broken document. Content = document header + VAT summary
per rate **in CZK** (no lines), like the CSV.

## Route
`GET /api/export/accountant?from&to&direction&format=csv|pohoda|money` — `format` default `csv` (2c unchanged).
`pohoda` → `200 application/xml`, filename `pohoda-{from}-{to}.xml`. `money` → 3c. Unknown format → 422
`format: invalid`.

## Settings → Accounting (`/api/settings/accounting`)

Per program, per **direction × document type** (the 6 exported types: invoice, credit_note, debit_note,
advance_tax_doc, advance_credit_note, simplified) a set of optional codes; empty = the element is not written and the
accountant fills it in the program.

```
AccountingSettings {
  pohoda: {
    ico: string | null,        // dataPack/@ico (the accounting unit); null → the company IČO
    codes: [ { direction, docType, accounting: string|null /* předkontace, typ:ids */,
               classificationVat: string|null /* členění DPH, typ:ids */, numberSeries: string|null /* číselná řada, typ:ids */ } ]
  },
  money: { codes: [ { direction, docType, … } ] }   // defined in 3c
}
```
- `GET` → the full object, every direction × type row present (missing rows → all null).
- `PUT` → replaces; each code ≤ 20 chars, trimmed, `""` → null; unknown direction / docType or a duplicate row → 422
  (`pohoda.codes.N.docType: invalid` / `duplicate`). Stored as one JSON settings row (migration).

## Pohoda XML (3b)

- Stormware data exchange XML, `dataPack` **version 2.0** (`dat:dataPack` with `id`, `ico`, `application="invoice"`,
  `version="2.0"`, `note`), one `dat:dataPackItem` (`id` = document id, `version="2.0"`) per document holding an
  `inv:invoice` (`version="2.0"`). Encoding **Windows-1250** (`<?xml version="1.0" encoding="Windows-1250"?>`, as
  Pohoda imports it; characters outside 1250 → numeric character references). Namespaces `dat`, `inv`, `typ` as in
  the official schemas.
- **Validation:** the official XSDs (data.xsd, invoice.xsd, type.xsd + their imports, version 2) are vendored under
  `tests/fixtures/pohoda/schema/` (with their source URL and date in a README) and every test export is validated with
  `xmllint` (already in CI for ISDOC).
- **invoiceType** by direction × doc type:

  | doc type | issued | received |
  |---|---|---|
  | invoice, simplified | `issuedInvoice` | `receivedInvoice` |
  | credit_note | `issuedCreditNotice` | `receivedCreditNotice` |
  | debit_note | `issuedDebitNote` | `receivedDebitNote` |
  | advance_tax_doc, advance_credit_note | `issuedAdvanceInvoice` | `receivedAdvanceInvoice` |

  (the exact enumeration values and the right agenda for the DDPP and its correction are confirmed against the XSD and
  Stormware's documentation; deviations go to Clarifications).
- **Header (`inv:invoiceHeader`):**
  - `invoiceType`; `number/numberRequested` = our number (received: our internal number) when no series code is set,
    else `number/ids` = the configured series; `symVar` = variable symbol; `originalDocument` = the supplier number
    (received) / our number (issued); `date` = issue date, `dateTax` = tax date, `dateAccounting` = tax date
    (received: received date), `dateDue`; `accounting/ids`, `classificationVAT/ids` from settings when set;
    `text` = a short description (doc type label + number);
  - `partnerIdentity/address`: company, name, street, city, zip, `ico`, `dic`, country `ids` — from the document's
    counterparty snapshot (contactless simplified → omitted);
  - `paymentType/paymentType` from the payment method (`draft` = bank transfer, `cash`, `creditcard`, …; exact
    values per XSD); `account` for issued documents only when the snapshot has an account / bank code;
  - corrections (credit / debit notes, DDPP corrections): the original's number in the element the XSD provides for
    the linked document (`originalDocumentNumber` or equivalent), when linked;
  - received `vatDeductible` false → the classification chosen in settings applies; nothing extra is invented.
- **Summary (`inv:invoiceSummary`):** `roundingDocument` none; CZK documents → `homeCurrency` with `priceNone`
  (0 %), `priceLow` + `priceLowVAT` (the lower rate), `priceHigh` + `priceHighVAT` (21 %), `price3` + `price3VAT`
  (second reduced rate when a document has one), and `round/priceRound` = rounding; amounts in CZK from the stored
  recap (credit notes / corrections **negative**). Foreign documents → `homeCurrency` in CZK (the stored CZK recap)
  **plus** `foreignCurrency` (`currency/ids`, `rate`, `amount` 1, `priceSum` = total in the currency). A rate that
  maps to none of Pohoda's slots (not 0 / 21 / 12 / a configured third) → the document is skipped and logged (2c rule).
  Rate → slot mapping: 21 % high, 12 % low, 0 % none; other active Settings rates in order to `price3`; documented in
  Clarifications.
- `dat:dataPack/@note` = `Export z Invoice {from}–{to}`.

## UI
- "Export pro účetní" dialog: a format select (CSV / Pohoda XML; Money S3 appears in 3c), default CSV; filename from
  the response.
- Settings → "Účetnictví" tab: Pohoda section — IČO override, a table rows = direction × doc type, columns
  předkontace / členění DPH / číselná řada; Save. (Money S3 columns come in 3c.)

## Tests
- Pure mapping unit tests (doc type → invoiceType, rate slots, negatives, foreign currency, 1250 encoding incl. a
  character outside 1250).
- Integration: a period with every exported type in both directions incl. EUR, validated against the XSD; settings
  codes present / absent; skipped document; settings GET/PUT validation.

## Clarifications (as implemented)
