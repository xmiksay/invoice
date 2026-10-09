# API — debit notes, DDPP corrections, simplified tax documents (phase 1f-a)

Three more document types complete the ISDOC set (ISDOC `DocumentType` code in brackets). They exist in both
directions, have their own number series and work in every place the existing four do: manual import, received
documents, PDF, lists, filters and Settings → number series. Everything below extends the 1b–1e contracts.

| docType | ISDOC | cs | en | sign | payable / QR |
|---|---|---|---|---|---|
| `invoice` | 1 | Faktura – daňový doklad | Invoice | 1 | yes |
| `credit_note` | 2 | Opravný daňový doklad | Credit note | −1 | refund, no QR |
| `debit_note` | 3 | Opravný daňový doklad – vrubopis | Debit note | 1 | yes |
| `proforma` | 4 | Zálohová faktura | Proforma invoice | 1 | yes |
| `advance_tax_doc` | 5 | Daňový doklad k přijaté platbě | Advance payment tax document | 1 | none (as 1c) |
| `advance_credit_note` | 6 | Opravný daňový doklad k přijaté platbě | Advance payment correction | −1 | refund, no QR |
| `simplified` | 7 | Zjednodušený daňový doklad | Simplified tax document | 1 | yes |

`sign = −1` documents store positive amounts, as credit notes do. A refund is a payment on that document (same
endpoints and `paymentState` rules as credit notes). The SPAYD QR rule from `pdf.md` extends from `{invoice, proforma}`
to `{invoice, proforma, debit_note, simplified}`.

## Number series
New `docType` keys of `/api/settings/number-series`, seeded by a migration and editable like the others:

| key | default pattern | numbers |
|---|---|---|
| `debit_note` | `V{YYYY}{NNNN}` | issued debit notes |
| `advance_credit_note` | `OP{YYYY}{NNNN}` | issued DDPP corrections |
| `simplified` | `ZD{YYYY}{NNNN}` | issued simplified documents |
| `received_debit_note` | `PV{YYYY}{NNNN}` | received debit notes |
| `received_advance_credit_note` | `POP{YYYY}{NNNN}` | received DDPP corrections |
| `received_simplified` | `PZD{YYYY}{NNNN}` | received simplified documents |

The `documents.doc_type` CHECK accepts the three new values (migration, append-only). The counter guard,
`number_taken` and imported-number uniqueness apply per docType exactly as before.

## Correctable documents
"Correctable" means an issued, non-cancelled, non-draft `invoice` or `simplified` document (imported ones included, as in
1e).
- `POST /api/documents/{id}/credit-note` (existing endpoint):
  - On a correctable document it creates a `credit_note`, as in 1c.
  - On an issued, non-cancelled `advance_tax_doc` it creates an `advance_credit_note` (see below).
  - Anything else → 409 `invalid_state`.
- `POST /api/documents/{id}/debit-note` body `{ correctionReason?: string }` → 201 draft `debit_note`. Only on a
  correctable document; anything else → 409 `invalid_state`.

## Debit note (`debit_note`)
- The draft copies the same header fields as a credit note (1c Clarifications): contact, currency, locale, vatMode,
  bank and paymentMethod. `issueDate = taxPointDate = today`. `dueDate` is today + (contact ?? company) due days.
  `relatedDocumentId` points at the original document.
- The exchange rate is the original's (`exchangeRateSource: "original"`; PUT ignores it). The draft has **no lines**:
  the user enters the additional charge.
- Like a credit-note draft, PUT requires the original's `currency`, `contactId` and `vatMode` (else `invalid`).
  `advance` lines → `lines.N.kind: invalid`.
- Issue requires `correctionReason` (≤ 500). It is numbered from `debit_note`. There is no cap.
- Credit-note cap (1c) becomes: for every rate, Σ base of the issued non-cancelled credit notes of the original
  (including the one being issued) ≤ the original's item base + Σ base of its issued non-cancelled debit notes. A
  rate is allowed when it occurs on the original **or on one of those debit notes**.
- A debit note cannot itself be credited or debited (credit-note / debit-note endpoints → 409 `invalid_state`).
  Corrections always reference the original.
- Cancelling a debit note: allowed as for a credit note, unless the credit notes would then exceed the reduced cap.
  In that case → 409 `{"code":"exceeds_original"}`.

## DDPP correction (`advance_credit_note`)
Use it when a received advance is refunded or reduced.
- `POST /api/documents/{ddppId}/credit-note` → 201 draft. It copies the DDPP's header like a credit note does from an
  invoice, plus all its item lines. `relatedDocumentId` = the DDPP. The exchange rate is the DDPP's (`"original"`).
- Refused with 409 when the DDPP is deducted by a non-cancelled invoice:
  - an **issued** invoice → `advance_settled`;
  - a **draft** invoice → `advance_in_use`.
  Checked at create and again at issue (lock the DDPP).
- Issue requires `correctionReason`. It is numbered from `advance_credit_note`.
- Cap per rate: Σ base of the DDPP's issued non-cancelled corrections (including this one) ≤ the DDPP's recap base.
  No other rate is allowed. A violation → 422 `{"fields":{"lines":"exceeds_original"}}`.
- **Exact VAT for a full correction.** Per rate, when the cumulative credited base equals the DDPP's base, this
  note's VAT for that rate is the DDPP's VAT minus the VAT already credited, not `round2(base × rate)`. The CZK
  amounts follow the same rule. This way a fully corrected DDPP nets to exactly zero, matching the 1c rule that a DDPP
  recap is never recomputed. Other rates compute normally. This is applied at issue and in `/compute` with
  `documentId`.
- **Effect on settlement.** The deducted amounts of an `advance` line become the DDPP recap **minus** its issued
  non-cancelled corrections, per rate: base, vat, baseCzk, vatCzk.
  - `settle` skips a DDPP whose net is zero on every rate.
  - An advance line referencing such a DDPP → `lines.N.advanceDocumentId: invalid`.
  - The snapshot on the line is refreshed at issue, as in 1c.
- `DELETE /api/documents/{proformaId}/payments/{paymentId}` whose DDPP has a non-cancelled correction (draft or
  issued) → 409 `advance_in_use`. Nothing changes.
- An `advance_credit_note` is never itself corrected or deducted. Cancelling it is allowed while the DDPP is not
  deducted by a non-cancelled invoice; otherwise → 409 `advance_settled` / `advance_in_use` as above.
- `Document.correctionBlock` (response only, on **every** document, `null` when it does not apply):
  `"advance_settled" | "advance_in_use" | "fully_corrected" | null`. Set only on an issued, non-cancelled DDPP of
  ours: an issued invoice deducts it → `advance_settled`, else a draft invoice does → `advance_in_use`, else its net
  is zero on every rate → `fully_corrected`. The UI disables "Opravný doklad" and shows the reason.
- `credit-note` on a fully corrected DDPP → 409 `invalid_state` (checked under the DDPP lock).
- PDF `reference`: "Opravný daňový doklad k daňovému dokladu k přijaté platbě {number}" (en: "Correction of advance
  payment tax document {number}") + the reason line.

## Simplified tax document (`simplified`)
- Created by `POST /api/documents` with `docType: "simplified"`. It behaves like an invoice except:
  - `contactId` is optional on save **and** issue. Without a contact the `customerSnapshot` is null and the PDF has no
    customer block.
  - No `advance` lines (`lines.N.kind: invalid`). It cannot settle a proforma; `settle` always creates an `invoice`.
  - The 10 000 CZK limit is a **UI warning only**. The editor and detail show it when the total in CZK (`totalCzk`,
    or `total` for CZK) exceeds 10 000. The server never refuses.
- Numbered from `simplified`. Credit notes and debit notes are allowed (see Correctable documents).
- PDF `reference` for its credit/debit notes: "… k zjednodušenému daňovému dokladu {number}" (en: "… to simplified tax
  document {number}").

## Received and imported
- Received documents: `docType` accepts all seven values. Numbering uses the received series of the type (table
  above), and amounts are a VAT recap, as in 1e. Informational `relatedDocumentId` targets extend the 1e table:

  | document | may link to |
  |---|---|
  | `credit_note` | `invoice`, `simplified` |
  | `debit_note` | `invoice`, `simplified` |
  | `advance_credit_note` | `advance_tax_doc` |
  | `simplified` | — |

  The same table applies to imported issued drafts (same direction). Imported documents skip caps and do not
  require a `correctionReason`.
- A received `simplified` document behaves like a received invoice. A received `advance_credit_note` has `sign = −1`.
- Manual import of issued documents (1e) accepts all seven types. An imported `simplified` document may have no
  contact. Imported debit notes / DDPP corrections have no cap and a rate that is manual or from ČNB, like an imported
  credit note.

## Lists, filters, compute
- `GET /api/documents?docType=` accepts the new values. `DocumentSummary.sign` follows the table.
- `POST /api/documents/compute` accepts `docType` `debit_note`, `advance_credit_note` and `simplified`. With
  `documentId` of a debit note or DDPP correction, it uses the original's rate, as for credit notes.
- `relatedDocuments` / `parent` include the new types, with the same shape.

## Errors
No new codes. New uses: 409 `exceeds_original` (cancelling a debit note), and `advance_in_use` / `advance_settled`
(DDPP corrections, payment delete).

## Frontend
- Document type tabs (issued and received) gain the three types. Labels come from the table, through i18n.
- "New document" offers invoice, proforma and simplified (received: all seven).
- On an issued invoice / simplified document the detail offers "Dobropis" and "Vrubopis". On an issued DDPP it offers
  "Opravný doklad" (hidden or disabled with the reason when 409 applies).
- The simplified document editor/detail shows the 10 000 CZK warning. The customer picker is optional for it.
- Settings → Number series lists all fourteen series (grouped issued / received).
- Manual import type selector: all seven types.

## Clarifications 1f-a (as implemented)
Additive details settled during the Phase 1f-a backend; nothing above changed.

Received documents
- (a) A received `advance_credit_note` requires `dueDate`, like a received credit note; only a received
  `advance_tax_doc` may have `dueDate: null`.
- (b) A received `simplified` document still requires its supplier `contactId`; the contact is optional only for
  issued (native and imported) `simplified` documents.

Corrections
- `credit-note` / `debit-note` work on an issued invoice or `simplified` document with `direction: "issued"`, imported
  ones included; `credit-note` on an issued DDPP of ours (imported included) creates the `advance_credit_note`.
  `POST /api/documents` with `docType` `credit_note` / `debit_note` / `advance_credit_note` and `imported` false →
  `docType: invalid` (as before for credit notes).
- Every native correction (credit note, debit note, DDPP correction) keeps the original's rate
  (`exchangeRateSource: "original"`) and is bound to its `currency`, `contactId` and `vatMode` on `PUT`. Imported
  ones are not bound, have no cap and need no `correctionReason` at issue — this now also holds for imported credit
  notes (1e required the reason).
- The debit-note cap re-check on cancel counts the issued credit notes only (drafts are checked at their own issue).
  It applies to imported debit notes linked to the original too, since they raise its cap.
- Issuing a native debit note locks its original and re-checks it is still correctable (cancelled meanwhile → 409
  `invalid_state`). A native credit / debit note of a `simplified` document issued without a customer is bound to
  that null contact and issues without one (`customer: null`).
- An imported `advance_credit_note` linked to a DDPP counts in the DDPP's net, so issuing or cancelling it is
  refused like a native one while the DDPP is deducted by a non-cancelled invoice (`advance_settled` /
  `advance_in_use`); it still has no cap and needs no reason.
- Exact VAT of a DDPP correction is applied whenever its totals are computed: at creation, on `PUT`, in `/compute`
  with its `documentId`, and again at issue against the corrections read under the DDPP lock (the stored draft totals
  are therefore already exact).
  For a foreign currency both `baseCzk` and `vatCzk` of a fully corrected rate are the DDPP's remainder;
  `totalCzk` stays `round2(payable × rate)`.
- "Issued corrections" of a DDPP (cap, net deduction) are the `advance_credit_note` documents with
  `relatedDocumentId` = the DDPP and `status: issued`, imported ones included. Net rows that are zero on all four
  amounts are dropped from an advance line's `recap`; a DDPP with nothing left is skipped by `settle` and
  `lines.N.advanceDocumentId: invalid` on save / issue. A correction issued between an invoice's validation and its
  write → 409 `conflict` (retry), like a changed non-payer proforma.
- Cancelling an imported DDPP that has a non-cancelled correction (draft or issued) → 409 `advance_in_use`.
- A refund on a DDPP correction is a payment on it (same endpoints); `paymentState` as for credit notes.

Simplified documents
- `POST` default `docType` stays `invoice`; `simplified` takes the same defaults as an invoice (tax point = issue
  date). At issue `contactId` may be null → `customer: null`. The PDF title is "Zjednodušený daňový doklad" /
  "Simplified tax document" in every VAT mode.

PDF
- Payload: `docType` takes all seven values; `customer` may be `null` (no customer block, see "1f-a UI follow-ups"
  below). The default template skips a `null` customer; a custom `invoice.typ` must do the same.
- Titles: `debit_note` "Opravný daňový doklad – vrubopis" / "Debit note"; `advance_credit_note` "Opravný daňový
  doklad k přijaté platbě" / "Advance payment correction".
- `reference` (+ "Důvod opravy: …" / "Reason: …" line when set): credit note "Opravný daňový doklad k faktuře {n}" /
  "Credit note for invoice {n}"; debit note "Opravný daňový doklad – vrubopis k faktuře {n}" / "Debit note for
  invoice {n}"; with a simplified original "… k zjednodušenému daňovému dokladu {n}" / "… to simplified tax document
  {n}"; DDPP correction as specified above. A DDPP correction prints its due date (no "payment date" row) and the
  payable row negated, like a credit note.

Number series
- `GET /api/settings/number-series` order: `invoice`, `credit_note`, `debit_note`, `proforma`, `advance_tax_doc`,
  `advance_credit_note`, `simplified`, then `received`, `received_credit_note`, `received_debit_note`,
  `received_proforma`, `received_advance_tax_doc`, `received_advance_credit_note`, `received_simplified`.

1f-a UI follow-ups
- `Document.correctionBlock` is serialised on every document (`null` unless an issued, non-cancelled DDPP is
  blocked); precedence `advance_settled` > `advance_in_use` > `fully_corrected`. Its net counts issued corrections,
  imported ones included. `credit-note` on a fully corrected DDPP → 409 `invalid_state`: no correction could pass the
  cap, so no draft is created.
- PDF `customer` is `null` (no "Odběratel" block) whenever the document has no customer and none is expected: an
  issued document without a customer snapshot (a `simplified` document or one of its credit / debit notes), or a
  draft of a `simplified` document or of one of its corrections. Other drafts without a contact keep the empty party.
