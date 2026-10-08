# PDF (phase 1d)

Documents are rendered to PDF by the mdcast service (`POST {INVOICE__MDCAST_URL}/v1/render/template`, crate
`mdcast-client` 0.4.2): one typst template (`invoice.typ`) + a JSON payload the template reads with
`json("/data.json")` + an asset bundle (design files, fonts, the generated QR code).

## Design files

- The default design is embedded in the binary from the repo directory `design/`: `invoice.typ`, `fonts/Inter-*.ttf`
  (+ `fonts/OFL.txt`). Minimalist layout, black/grey, one accent colour, no logo.
- `INVOICE__DESIGN_DIR` (optional) overrides it **file by file**: the effective set is embedded ∪ dir, the dir wins on
  the same relative path. The dir is re-read on every render (design edits apply without a restart). Hidden files
  (`.`-prefixed path segments) are ignored; a file > 10 MB → render fails (`pdf_render_failed`). Set but not an
  existing directory → server refuses to start.
- Every `fonts/**/*.ttf|*.otf` is registered with the typst font book (request `fonts`, sorted by path); every other
  file goes into the bundle under its relative path, so `#image("logo.png")` / `#import "parts.typ"` resolve like on a
  filesystem.
- Optional well-known files: `logo.svg` or `logo.png` (svg wins), `signature.png` (stamp/signature above the
  "Issued by" line). The template cannot test file existence, so the payload carries `assets.logo` /
  `assets.signature` = the relative path or `null`.
- `qr.svg` is reserved: generated per render, it replaces any design file of that name.

## Payload (`/data.json`)

Every displayed value is a **pre-formatted string** in the document's locale (the template does no number/date
formatting); labels are resolved server-side, so the template carries no hardcoded text. Missing values are `null`.

```
{
  locale: "cs" | "en",
  draft: boolean,                       // true → template draws the "NÁVRH" / "DRAFT" watermark
  docType: "invoice" | "proforma" | "credit_note" | "advance_tax_doc",
  vatMode: "standard" | "reverse_charge" | "exempt" | "non_payer",
  title: string,                        // see Titles
  number: string | null,                // null on a draft
  supplier: Party, customer: Party,     // issued: the snapshots; draft: live company / contact
  dates: [ { label, value } ],          // ordered rows: issue date, tax point (DUZP), due date, payment date (DDPP)
  payment: [ { label, value } ],        // ordered rows: method, account number, IBAN, BIC, VS, KS, order ref
  headerNote: string | null, footerNote: string | null,
  reference: string | null,             // credit note: "Opravný daňový doklad k faktuře 20260001" + reason line;
                                        // DDPP: "K zálohové faktuře Z20260003"; settled invoice lists nothing here
  showVat: boolean,                     // false for non_payer: VAT columns + recap hidden
  columns: { description, quantity, unitPrice, discount, vatRate, base, … },   // header labels
  hasDiscount: boolean,                 // false → discount column hidden
  lines: [ Line ],
  vatRecap: { title, rows: [ { rate, base, vat, total } ], total: { base, vat, total } } | null,
  vatRecapCzk: { title, rateNote, rows: [ … ], total } | null,   // foreign currency + standard VAT only (§ 37)
  totals: [ { label, value, strong: boolean } ],  // total excl. VAT, VAT, total, advances deducted, rounding, payable
  legalNote: string | null,             // by vatMode, see Legal texts
  paidNote: string | null,              // DDPP: "Neplaťte – již uhrazeno."
  qr: { image: "qr.svg", label: "QR platba" } | null,
  assets: { logo: string | null, signature: string | null },
  footer: { registration: string | null, pageLabel: "Strana" | "Page", issuedBy: "Vystavil" | "Issued by" }
}
Party { title /* "Dodavatel"/"Odběratel" */, name, lines: [string] /* street, "zip city", country name unless CZ */,
        ids: [string] /* "IČO: …", "DIČ: …" — only present ones */, contact: [string] /* email, phone, web */ }
Line  { kind: "item" | "text" | "subtotal" | "advance", description, quantity /* "2 ks" */, unitPrice, discount,
        vatRate /* "21 %" */, base, strong: boolean }
```

- Lines follow the document order; members of a **collapsed** subtotal are omitted, the subtotal itself is printed
  as a regular row (`kind: "subtotal"`, its quantity/unit price `null`). Non-collapsed subtotals print as a bold
  summary row after their members. `text` lines carry only `description`. Advance (deduction) lines print with a
  negative base. Credit-note amounts print **negated** (the stored positive values × sign).
- Money: `cs` → `1 234,50 Kč` (NBSP thousands, comma decimal, currency symbol for CZK/EUR/USD else the code after),
  `en` → `CZK 1,234.50`. Quantities trimmed of trailing zeros. Dates: `cs` `8. 10. 2026`, `en` `8 Oct 2026`.
- Exchange rate note (`vatRecapCzk.rateNote`): `Kurz ČNB 24,400 CZK/EUR ze dne 7. 10. 2026` (source `manual` →
  "Kurz 24,400 CZK/EUR", `original` → same as cnb of the original).

### Titles
| docType | cs | en |
|---|---|---|
| invoice (standard / reverse_charge / exempt) | Faktura – daňový doklad | Invoice – tax document |
| invoice (non_payer) | Faktura | Invoice |
| proforma | Zálohová faktura | Proforma invoice |
| credit_note | Opravný daňový doklad | Credit note |
| advance_tax_doc | Daňový doklad k přijaté platbě | Tax document for a received payment |

The tax point (DUZP) row is omitted for proformas and non-payers; the due date row is omitted for DDPPs.

### Legal texts
- `reverse_charge`: cs "Daň odvede zákazník (přenesení daňové povinnosti, § 92a zákona o DPH)." / en "Reverse charge –
  VAT to be accounted for by the customer."
- `exempt`: cs "Plnění osvobozené od DPH." / en "VAT exempt supply."
- `non_payer`: cs "Dodavatel není plátcem DPH." / en "The supplier is not a VAT payer."

### SPAYD QR
Present only when **all** hold: not a draft, `docType ∈ {invoice, proforma}`, `paymentMethod = bank_transfer`, the bank
snapshot has an IBAN, `payable > 0`. Generated server-side as SVG (crate `qrcode`), string:
`SPD*1.0*ACC:{IBAN}[+{BIC}]*AM:{payable, 2 dp, '.'}*CC:{currency}*DT:{dueDate YYYYMMDD}[*X-VS:{vs}][*X-KS:{ks}]*MSG:{title number, ≤60 chars}`;
`*` inside values is stripped, MSG is ASCII-folded (no diacritics).

## Archive

- Columns on `documents`: `pdf_path text null` (relative to `INVOICE__STORAGE_DIR`, `documents/{numberYear}/{id}.pdf`),
  `pdf_sha256 text null`, `pdf_rendered_at timestamptz null`. Written to a temp file in the same dir, fsync, rename.
- **Issue** (`POST /api/documents/{id}/issue`, all doc types) renders and archives inside the issue transaction after
  the number and snapshots are assigned; render failure → transaction rolls back, nothing is issued (503
  `pdf_unavailable` / 502 `pdf_render_failed`); commit failure → the written file is removed (best effort).
- **DDPP** (auto-issued with a payment): the payment never fails because of PDF. After the payment transaction
  commits, the handler tries to render + archive once (failure logged). A DDPP without an archive is rendered and
  archived on its first `GET …/pdf` (concurrent first downloads: `UPDATE … WHERE pdf_path IS NULL`, the loser serves
  its own bytes).
- The archive is **immutable**: never re-rendered, also not after cancel or a design change.

## Routes

- `GET /api/documents/{id}/pdf?download=1` → `200 application/pdf`, `Content-Disposition: inline` (`attachment` with
  `download=1`), filename `{number}.pdf` (draft: `draft-{first 8 chars of id}.pdf`), `Cache-Control: no-store`.
  - draft → rendered live, `draft: true`, never stored;
  - issued / cancelled with an archive → the stored file (sha256 not re-checked);
  - issued / cancelled without an archive (DDPP only) → render + archive, as above;
  - archive file missing on disk → 500 (logged).
- `GET /api/pdf/preview?locale=cs|en` → `200 application/pdf` of a sample invoice (never stored): supplier = the
  company profile (placeholder name when empty), fictional customer, CZK, standard VAT, items at 21 % and 12 %, one
  with a 10 % discount, a collapsed subtotal group, a text line, bank = default CZK account (if any → QR shown),
  `draft: false`, number `{YYYY}0001`-style sample. `locale` missing → company `defaultLocale`; invalid → 422.
- `GET /api/pdf/design` → `{ designDir: string | null, files: [ { path, source: "custom" | "default", size } ] }`
  (effective set, sorted by path).

Fetch the PDF with the Bearer header (blob → object URL); a plain link cannot carry the token.

## Errors
- 503 `{"code":"pdf_unavailable"}` — mdcast unreachable, token rejected, gateway 502–504.
- 502 `{"code":"pdf_render_failed","detail":"…"}` — mdcast answered but the template failed (typst diagnostics), or a
  design file is too large. `detail` is the mdcast message (the template is the user's own; no DB/internal data).
- Document `pdf` field (added to the Document DTO): `{ sha256, renderedAt } | null`.

## Configuration
| Env var | Default | Notes |
|---|---|---|
| `INVOICE__MDCAST_URL` | `https://mdcast.nexial.cz` | empty → default |
| `INVOICE__MDCAST_TOKEN` | — | optional Bearer for mdcast |
| `INVOICE__DESIGN_DIR` | — | optional override dir (must exist when set) |
| `INVOICE__STORAGE_DIR` | `./data` | created at start; Docker image uses `/data` (volume) |

mdcast HTTP timeout 60 s.

## Clarifications (as implemented)

- **Party snapshots** gained optional `email`, `phone`, `web` (company: all three, contact: email + phone); snapshots
  from before 1d read them as `null`. `Party.contact` is built from them; `Party.lines` adds the country name (16
  common codes in cs/en, else the code) unless `CZ`. A draft without a contact gets an empty customer party.
- **Extra payload fields** (so the template stays free of text): `watermark` (`"NÁVRH"`/`"DRAFT"` on a draft,
  `"STORNO"`/`"CANCELLED"` on a cancelled document rendered without an archive, else `null`); recaps carry `columns: { rate, base, vat, total }` header labels and `rateNote` (`null` on `vatRecap`);
  a recap `total` row's `rate` holds the "Celkem"/"Total" label; `columns.base` is "Základ"/"Net", for a non-payer
  "Celkem"/"Amount".
- **Empty env values** mean the default: `INVOICE__MDCAST_URL`, `INVOICE__MDCAST_TOKEN`, `INVOICE__DESIGN_DIR`,
  `INVOICE__STORAGE_DIR`.
- **Formatting**: negative money `-1 234,50 Kč` / `CZK -1,234.50`; `en` percent without a space (`21%`), `cs` with
  NBSP (`21 %`); dates use plain spaces; exchange rates print with at least 3 decimals.
- **Lines**: collapsed subtotals hide their members recursively (a member that is itself a subtotal hides its own
  members too). `vatRate` is `null` when `showVat` is false; `discount` is `null` for 0 %. An advance line prints as
  one row (description + negative base, no rate). Credit notes negate unit price, base, recaps and totals; the
  quantity stays positive.
- **Totals rows**: stored totals are net of advance deductions, so with advance lines the rows show total excl. VAT /
  VAT / total incl. VAT *before* the deduction, then "Odpočet záloh" (negative gross), then rounding (only when
  non-zero) and the payable (`strong`). Non-payer: one "Celkem" row instead of the three. DDPP: excl. VAT, VAT,
  total incl. VAT (`strong`), no payable row. The payable label is "K úhradě"/"Amount due" for every type (negative
  on a credit note).
- **Rows**: `payment` lists account number / IBAN (grouped by 4) / BIC only for `bank_transfer`; VS, KS and order
  reference whenever set. The DDPP "payment date" row is its tax point date.
- **Exchange-rate note** for `original` (credit notes): the original invoice's source and date are used (ČNB → dated
  note, manual → "Kurz …").
- **Archive on first download** applies to any issued/cancelled document without an archive (DDPPs, and documents
  issued before 1d), not only DDPPs. A cancelled one renders with the "STORNO" watermark and never a QR (the QR rule
  requires status `issued`). The DDPP archive after a payment runs in a background task once the payment has
  committed — the 201 is not delayed; a failure is only logged. The loser of the `WHERE pdf_path IS NULL` race
  serves the winner's stored file (not its own render), so every download returns the archived bytes.
- **Download**: `download=1` or `download=true` → `attachment`. The filename stem is reduced to `[A-Za-z0-9._-]`
  (others → `_`). A missing archive file → 500 `{"code":"internal"}`.
- **Error detail**: `detail` is only returned for genuine template failures (mdcast `render_failed`, i.e. typst
  diagnostics of the user's template) and our own design checks (file > 10 MB, non-UTF-8 `invoice.typ`). Any other
  mdcast failure (its 500 / `internal`, `payload_too_large`, an undecodable or unexpected answer) is 502
  `pdf_render_failed` **without** `detail`; the upstream body is logged only.
- **Design**: font files are bundled too (request `fonts` are keys into the bundle); `invoice.typ` is sent as the
  template source, not as an asset; `GET /api/pdf/design` also lists files over 10 MB (they only fail a render). An
  unreadable design dir → 500 `internal`. Symlinks are followed (files and dirs), each real directory is entered
  once, so a link back into the tree (`shared -> .`) cannot loop. Embedded default files are passed to the bundle
  without copying in release builds; only `INVOICE__DESIGN_DIR` files are read per render. (The client still
  hashes every asset per render to build the manifest — inherent to `AssetBundle`.) A non-UTF-8 `invoice.typ` → `pdf_render_failed`.
- **mdcast token**: without `INVOICE__MDCAST_TOKEN` a placeholder Bearer is sent (`mdcast-client` requires one; a
  server without a gate ignores it). `https://mdcast.nexial.cz` requires a real token for renders — without it every
  render is 503 `pdf_unavailable` (401 maps there).
- **Preview**: VAT mode `standard` and CZK regardless of the company's payer flag; due date = today + 14 days;
  supplier placeholder "Vaše firma s.r.o." / "Your Company Ltd." when the company name is empty; the bank is the
  default CZK account; VS = the sample number.
