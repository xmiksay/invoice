# Accounting export: Money S3 XML (3c)

The `money` format of the 2c/3b accountant export
([accounting.md](accounting.md)): same period (tax date), direction choice, document set (no drafts, no cancelled,
no proformas), 10 000 cap, snapshot transaction and the 3b rules for the whole file (built inside the snapshot before
sending; empty period → 422 `from: empty`; unmappable document → 422 `documents: unexportable` with `detail`, never a
file with documents silently missing). Content = document header + VAT summary per rate in CZK (no lines).

## Route
`GET /api/export/accountant?from&to&direction&format=money` → `200 application/xml; charset=utf-8`,
`Cache-Control: no-store`, filename `money-{from}-{to}.xml`. `format` accepts `csv | pohoda | money`; anything else →
422 `format: invalid`.

## Settings → Accounting
`AccountingSettings` gains the `money` section next to `pohoda` (same endpoint, same table and row, no migration):

```
money: {
  ico: string | null,     // MoneyData/@ICAgendy; null → the company IČO
  codes: [ { direction, docType,
             accounting: string|null,                     // předkontace, PredKontac (≤ 10)
             classificationVat: string|null,              // členění DPH, KodDPH (≤ 10)
             classificationVatNonDeductible: string|null, // KodDPH of a received doc without deduction (≤ 10)
             numberSeries: string|null } ]                // číselná řada, Rada (≤ 5)
}
```
- Same rules as `pohoda`: the 12 direction × type rows always present in both responses (issued first, types in the
  3b order); PUT takes any subset of rows; trimmed, `""` → null.
- **Change to 3b:** each section present in the PUT body replaces that section (`"money": {}` clears it); an
  **absent section is kept** (a PUT with only `pohoda` keeps the stored `money`). A body `{}` therefore changes
  nothing; before 3c it cleared the Pohoda codes. The UI always sends both sections.
- 422 keys mirror 3b: `money.ico: invalid_ico`; `money.codes.N.direction|docType: invalid`;
  `money.codes.N.docType: duplicate`; `money.codes.N.{accounting|classificationVat|classificationVatNonDeductible}:
  too_long` (> 10); `money.codes.N.numberSeries: too_long` (> 5);
  `money.codes.N.classificationVatNonDeductible: invalid` on an issued row.

## Money S3 XML
- Money S3 native XML transfer: root `MoneyData` (attributes `ICAgendy`, `description` = `Export z Invoice
  {from}–{to}`, `ExpDate`, `JazykVerze="CZ"`), **UTF-8** (`<?xml version="1.0" encoding="UTF-8"?>`), no namespace.
- **Validation:** the Money S3 XSDs (`_Document.xsd` + includes) are vendored under `tests/fixtures/money/schema/`
  with a README naming the source (copied from the public `WeblateOrg/website` repo, `schemas/money-s3/`, which
  carries the files Money S3 ships in `Data/XMLDE/Schemas`) and the date. Every test export is validated with
  `xmllint` against `_Document.xsd`.
- **Lists** by direction × doc type (all elements `fakturaType`):

  | doc type | issued | received |
  |---|---|---|
  | invoice, simplified, credit_note, debit_note | `SeznamFaktVyd/FaktVyd` | `SeznamFaktPrij/FaktPrij` |
  | advance_tax_doc, advance_credit_note (DDPP) | `SeznamFaktVyd_DPP/FaktVyd_DPP` | `SeznamFaktPrij_DPP/FaktPrij_DPP` |

  Empty lists are not written. `Druh`: `N` for the invoice lists, `D` for the DDPP lists. `Dobropis` true for
  credit_note and advance_credit_note. `ZjednD` true for an issued simplified document.
- **Header** (XSD element order, empty values left out):
  - `Doklad` (≤ 10): our number (received: the internal number) **when it fits; a longer number → `Doklad` is
    omitted** and Money assigns one from the series (`Rada`) — never cut, never a 422. `Rada` = the series code
    when set.
  - Our number is always carried elsewhere: issued → `EvCisDokl` (≤ 50, the KH evidence number) = our number;
    received → `PrijatDokl` (≤ 50) = the supplier number (else the internal number). `VarSymbol` (≤ 20) = the
    variable symbol. Longer than these limits → unexportable (numbers are never cut).
  - `Popis` (≤ 50) = the 3b `text` (label + number + ` k {original}`), cut to 50.
  - `Vystaveno` = issue date; `PlnenoDPH` = DUZP (received: else the received date); `DatUcPr`: issued = tax date,
    received = received date (else tax date); `Splatno` = due date; `Doruceno` = received date (received only).
  - `PredKontac`, `KodDPH`: from the settings row of the document's direction × type; a non-deductible received
    document takes `classificationVatNonDeductible`, **and when it is empty `KodDPH` is left out** (the accountant
    sets it in Money; nothing else marks the missing deduction).
  - `Uhrada` (≤ 20) from the payment method: bank_transfer `převodem`, cash `hotově`, card `kartou`, other → left out.
  - `SazbaDPH1` = 12, `SazbaDPH2` = 21 (the slots used below).
  - `DodOdb` (`dokladFirmaType`) from the counterparty snapshot: `ObchNazev`, `ObchAdresa` (`Ulice`, `Misto`, `PSC`,
    `Stat`), `ICO`, `DIC`, `PlatceDPH` when known; cut to the XSD lengths; no snapshot (contactless simplified) →
    no element.
- **Summary** `SouhrnDPH` in CZK from the stored recap (4 dp max, written with 2, `.` decimal point; credit notes and
  DDPP corrections **negative**):
  - 0 % / exempt / non-payer bases → `Zaklad0`; 12 % → `Zaklad5` + `DPH5`; 21 % → `Zaklad22` + `DPH22`; any other
    rate → `SeznamDalsiSazby/DalsiSazba` (`Sazba`, `Zaklad`, `DPH`). No rate makes a document unexportable.
  - Exempt / non-payer documents put every base in `Zaklad0`; reverse charge keeps the rate slot with VAT 0 (as 3b).
  - Rounding: as the XSD allows (element and behaviour settled in Clarifications); Money's total must equal our
    payable.
- **Foreign currency:** `Valuty` with `Mena/Kod`, `Mena/Mnozstvi` 1, `Mena/Kurs` (the stored rate) and
  `SouhrnDPH` in the currency; the header `SouhrnDPH` stays in CZK. A foreign recap with neither a CZK amount nor a
  rate → unexportable (as 3b).

## UI
- "Export pro účetní" dialog: format select CSV / Pohoda XML / Money S3 XML.
- Settings → "Účetnictví": a Money S3 section like the Pohoda one (IČO agendy override; table direction × type with
  předkontace / členění DPH / členění bez nároku (received rows only) / číselná řada; limits 10 / 10 / 10 / 5).
  One Save for both sections.

## Tests
- Pure mapping unit tests: lists and `Druh`, `Doklad` fit vs omitted, rate slots incl. `DalsiSazba`, negatives,
  foreign currency, non-deductible with and without the code.
- Integration: a period with every exported type in both directions incl. EUR, validated against the vendored XSD;
  settings codes present / absent; unexportable detail; settings GET/PUT for `money` (validation keys, a PUT of
  one section keeps the other).

## Clarifications (as implemented)
(filled during the 3c implementation)
