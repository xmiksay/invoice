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
Additive details and deviations settled during the Phase 3c backend. Element names, order and lengths were checked
against the vendored XSDs (`tests/fixtures/money/schema/`, see its README), not taken from the text above.

Route
- `Content-Type: application/xml; charset=utf-8`, `Cache-Control: no-store`. The pipeline is the 3b one
  (`accounting::export`, shared with Pohoda): 2c document set, order and cap, the whole file built inside the
  snapshot. An empty `MoneyData` would pass the XSD, but an empty period is still 422 `from: empty`, as for Pohoda.
- **Unexportable** → 422 `documents: unexportable` with the 3b `detail` (`<number>: <reason>; …`, our number for
  issued, the supplier number for received, ≤ 2000 chars, logged at `warn` as `accountant money`). Reasons:
  - `variable symbol longer than 20`; `number` / `supplier number longer than 50` (our numbers are ≤ 40, so this
    is a guard only); `currency longer than 4`;
  - more than 4 rates other than 0 / 12 / 21 % (`5 VAT rates other than 0 / 12 / 21 %, Money S3 takes at most 4`;
    the XSD annotation of `SeznamDalsiSazby` says "standardně max. 4 sazby");
  - a foreign recap with neither a CZK amount nor a rate, an undecodable snapshot, an unknown stored value (3b).
  - A rate never makes a document unexportable otherwise: Pohoda's third-rate setting is not used (nor loaded)
    by the Money export.

Settings
- `GET` returns both sections, each with the 12 rows. A stored 3b row (no `money`) reads as an empty Money section.
- `PUT` body (OpenAPI `AccountingUpdate`): `pohoda` / `money` optional; a section that is absent **or `null`** is
  kept; `{}` changes nothing. The errors of both sections are reported together. The read-modify-write runs in
  one transaction with the row locked, so two saves of different sections never lose one.
- Both sections share one shape (OpenAPI `ProgramSettings`, formerly `PohodaSettings`). Money limits come from the
  XSD: `PredKontac` / `KodDPH` are `zkratkaType` (`maxLength` 10), `Rada` is `maxLength` 5.

Money S3 XML
- **Root:** `MoneyData` with `ICAgendy` (the settings IČO, else the company IČO, else left out), `description`,
  `ExpDate` = the export day (Prague). **Deviation: no `JazykVerze`** — `_Document.xsd` does not declare it and
  `xmllint` rejects the file with it.
- `MoneyData` and `fakturaType` are `xs:all` (any order); everything is still written in XSD order. Lists:
  `SeznamFaktPrij`, `SeznamFaktVyd`, `SeznamFaktPrij_DPP`, `SeznamFaktVyd_DPP` (empty ones left out), each in the
  2c export order. Item elements: `Doklad`, `EvCisDokl`, `Rada`, `Popis`, `Vystaveno`, `DatUcPr`, `PlnenoDPH`,
  `Splatno`, `Doruceno`, `KodDPH`, `ZjednD`, `VarSymbol`, `PrijatDokl`, `Druh`, `Dobropis`, `Uhrada`,
  `PredKontac`, `SazbaDPH1`, `SazbaDPH2`, `SouhrnDPH`, `Celkem`, `Valuty`, `DodOdb`.
- `Doklad` of a received document = our internal number (e.g. `P20260001`) when it fits. `EvCisDokl` is written
  for issued documents only and `PrijatDokl` for received ones only (the XSD marks them "pouze faktury vydané /
  přijaté").
- `Druh` is always written; `ZjednD` and `Dobropis` only when true (the XSD default is false).
- **Linked original:** the CZ schema has no element for it (`PuvDoklad` is "pouze SK verze", `SeznamVazeb`
  needs the original inside Money), so it stays in `Popis` (` k {original}`), as in Pohoda's `text`.
- `Uhrada` is free text (≤ 20, Money's default `převodem`).
- **`DodOdb`:** `ObchNazev` (no XSD limit, not cut); `ObchAdresa` (only when a part is set): `Ulice` ≤ 50,
  `Misto` ≤ 40, `PSC` ≤ 10 (cut), and **`KodStatu`** (ISO 3166-1, exactly 2) instead of `Stat`, which is the
  country *name* (≤ 20); a country that is not 2 letters is left out. `ICO` ≤ 10, `DIC` ≤ 20 (cut; blank → left
  out, never an empty element), `PlatceDPH` from the snapshot's `vatPayer` when known.
- **Summary:** `Zaklad0`, `Zaklad5`, `Zaklad22`, `DPH5`, `DPH22`, `SeznamDalsiSazby` (XSD order).
  - `DalsiSazba` holds `HladinaDPH` (0 = 0 %, 2 = a rate ≥ 20 %, else 1 = snížená), `Sazba`, `Zaklad`, `DPH` (no
    `Popis`); one per rate, the recap rows of one rate merged.
  - **Whenever `SeznamDalsiSazby` is written, `Zaklad0` is written too** (`0.00` when there is nothing at 0 %).
    Per the `SeznamDalsiSazby` annotation, a summary with none of `Zaklad0` / `Zaklad5` / `Zaklad22` makes Money
    take the zero and standard rates from the list itself; the always-present `Zaklad0` keeps that path off.
  - Amounts are `castkaType` (decimal, ≤ 4 dp), written with 2.
  - `Celkem` is required by the XSD (header and `Valuty`) but "IMPORT: NE": Money computes the total itself. It is
    written as the sum of the summary amounts just written.
- **Rounding (haléřové vyrovnání):** neither `fakturaType` nor `souhrnDPHType` has a rounding element, and Money
  sums the summary on import (`Celkem` is ignored). So the document's `rounding` (signed like the amounts) is
  **added to `Zaklad0`** (the 0 % / non-VAT slot; written even when it is the only amount there). Money's total
  then equals our payable: `ZJ-1` 100 + 12.40 rounded to 112 → `Zaklad0` -0.40, `Celkem` 112.00. The accountant
  sees the rounding in the 0 % column.
- **Foreign currency:** `Valuty`: `Mena/Kod`, `Mnozstvi` 1, `Kurs`, `SouhrnDPH` = the recap in the currency with
  the rounding in its `Zaklad0`, `Celkem` = total + rounding in the currency.
  - `Kurs` = the stored rate **rounded to 4 dp** (`castkaType`; `24.335125` → `24.3351`). Without a stored rate
    (but with CZK recap amounts) it is the **implied rate** = CZK recap total / currency recap total, 4 dp, so
    Money never applies a rate of its own. Neither → unexportable (a zero total leaves it out).
  - The header `SouhrnDPH` is the CZK recap **plus the rounding converted at that `Kurs`** (rounding × `Kurs` /
    `Mnozstvi`, 2 dp, half away from zero) in its `Zaklad0`, so both summaries describe the same payable. The
    header `Celkem` (CZK) can differ from the stored `total_czk` by the per-row rounding of the CZK recap.
- VAT modes as 3b: exempt / non-payer → every base in `Zaklad0`; reverse charge keeps the rate slot with VAT 0.
