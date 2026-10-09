# E-mail (phase 2a)

Issued documents are sent by e-mail over SMTP (crate `lettre`, tokio + rustls). The subject and body come from
MiniJinja plain-text templates (one set for all document types, per locale), editable in Settings → E-mail.

## Configuration (env only)

| Variable | Meaning |
|---|---|
| `INVOICE__SMTP__HOST` | SMTP server; unset → e-mail not configured (sending → 503 `smtp_not_configured`) |
| `INVOICE__SMTP__PORT` | default by TLS mode: `starttls` 587, `tls` 465, `none` 25 |
| `INVOICE__SMTP__TLS` | `starttls` (default) \| `tls` (implicit TLS) \| `none` (plain, tests / local relay only) |
| `INVOICE__SMTP__USERNAME`, `INVOICE__SMTP__PASSWORD` | both or neither (one alone → `serve` refuses to start) |
| `INVOICE__SMTP__FROM` | sender, `addr@x` or `Name <addr@x>`; required when HOST is set |

- Invalid config (bad FROM, unknown TLS mode, HOST without FROM) → `serve` refuses to start with a clear message.
- Every SMTP exchange (connect + send) has a 30 s total timeout.
- Headers: `From` = FROM, `Reply-To` = company e-mail (Settings → Company) when set, `To`/`Cc` as given, `Bcc` never
  written to the message (envelope only), `Date`, `Message-ID: <{uuid}@{domain of FROM}>`, `MIME-Version`;
  body `text/plain; charset=utf-8`; attachments base64.

## Templates

- One template set for all seven document types: `subject` + `body` per locale (`cs`, `en`). Defaults are embedded
  in the binary (`src/email/defaults/{cs,en}/{subject,body}.txt`); overrides live in Storage under
  `email/templates/{locale}/subject.txt` and `…/body.txt` (both written together on save). Storage unreachable →
  503 `storage_unavailable`.
- MiniJinja, `UndefinedBehavior::Strict`, no HTML auto-escape, `none` renders as an empty string. The rendered
  subject is trimmed and newlines in it are replaced by spaces; the body keeps its text, trailing whitespace trimmed.
- Context (all display values are pre-formatted strings in the template's locale, same formatting as the PDF payload;
  missing values are `none`):

```
doc: {
  type: "invoice" | "proforma" | "credit_note" | "advance_tax_doc" | "debit_note" | "advance_credit_note" | "simplified",
  typeLabel,            // the PDF title, e.g. "Faktura – daňový doklad"
  number, issueDate, taxDate, dueDate,
  total,                // "1 234,50 Kč"
  payable,              // amount left to pay after advances / payments; "0,00 Kč" when settled
  currency,             // "CZK"
  paid: bool, cancelled: bool,
  variableSymbol, bankAccount, iban,
  originalNumber        // corrections: the corrected document's number, else none
}
company: { name, email, phone, web }
contact: { name, email } | none   // none for a contactless simplified document
```

- Defaults (cs; en analogous):
  - subject: `{{ doc.typeLabel }} {{ doc.number }} – {{ company.name }}`
  - body: greeting, "v příloze Vám zasíláme doklad č. {number} ({typeLabel}) na částku {total}", due date + VS
    when payable > 0 and not paid, "Neplaťte – již uhrazeno." when paid, signature `company.name`.
- **Validation** (save and preview): compile both templates, then render them in strict mode on the sample context
  (the PDF preview's sample invoice, with a contact). The first error →
  422 `{"code":"template_invalid","fields":{"subject"|"body":"template_invalid"},"detail":"line 3: undefined value"}`
  (`detail` = line + MiniJinja message; subject checked first). Size limits: subject 500, body 20 000 chars
  (`too_long`), subject empty → `required`.

## Routes

### Settings
- `GET /api/settings/email` → `{ configured: bool, from: string | null, replyTo: string | null }`
  (`replyTo` = company e-mail).
- `POST /api/settings/email/test` `{ to?: string }` (default: company e-mail; neither → 422 `to: required`) → 204;
  sends a short fixed test message (no template), not logged. 503 `smtp_not_configured`, 502 `smtp_failed` + `detail`.
- `GET /api/settings/email/templates` → `{ templates: [ { locale, subject, body, custom: bool } ] }` (cs, en; the
  effective text, `custom` = an override exists).
- `PUT /api/settings/email/templates/{locale}` `{ subject, body }` → 200 the template entry; invalid → 422 as above,
  the active template stays. Unknown locale → 404.
- `DELETE /api/settings/email/templates/{locale}` → 200 the default entry (`custom: false`); removes the override
  (idempotent).
- `POST /api/settings/email/templates/{locale}/preview` `{ subject, body }` → `{ subject, body }` rendered on the
  sample; 422 as above. Nothing stored.

### Sending
- `GET /api/documents/{id}/email?locale=cs|en` → the dialog prefill:
  ```
  { configured: bool, locale, to: [string], cc: [], bcc: [string], subject, body,
    attachments: { pdf: { available: bool, filename }, isdoc: { available: true, filename } } }
  ```
  `locale` defaults to the document's locale; `to` = the contact's e-mail when set (else `[]`); `bcc` = the company
  e-mail when set; `pdf.available` false only for an imported document without an original. Only for an issued
  (non-draft, cancelled included) document of direction `issued`; a draft → 409 `invalid_state`, a received
  document → 404. Template error on real data (cannot normally happen after validation) → 422 `template_invalid`.
- `POST /api/documents/{id}/email` `{ to: [string], cc: [string], bcc: [string], subject, body, attachPdf: bool,
  attachIsdoc: bool }`:
  - 422: `to` empty → `to: required`; an unparsable address → `to.0: invalid` (same for cc/bcc); more than 50
    recipients in total → `to: too_long`; subject empty → `required`, > 500 → `too_long`, newline in subject →
    `invalid`; body > 100 000 → `too_long`; `attachPdf` while `pdf.available` is false → `attachPdf: invalid`.
  - Same state rules as the GET. SMTP not configured → 503 `smtp_not_configured`.
  - Attachments are prepared before sending: `{number}.pdf` (the archive; lazy render like `GET …/pdf`, the imported
    original for imported documents), `{number}.isdoc` (`application/xml`, the plain XML of `GET …/isdoc`, no
    `.isdocx`). Preparation errors (`pdf_unavailable`, `pdf_render_failed`, `storage_unavailable`) abort before
    sending and are **not** logged.
  - Then the message is sent synchronously and the attempt is logged either way:
    - success → 200 the log entry (`ok: true`) and the document's `sentAt` = now (overwrites, like mark-sent);
    - SMTP failure / timeout → 502 `{"code":"smtp_failed","detail":"<SMTP error>"}`, entry logged with `ok: false`,
      `sentAt` unchanged.
  - Re-sending is allowed any number of times.
- `GET /api/documents/{id}/emails` → `[EmailLogEntry]`, newest first (404 for an unknown or received document; a
  draft has none).

```
EmailLogEntry { id, createdAt: datetime, to: [string], cc: [string], bcc: [string], subject, body,
                attachments: [string] /* filenames */, ok: bool, error: string | null, messageId: string | null }
```

## Storage / DB

- Table `document_emails` (append-only log): `id uuid pk`, `document_id uuid fk → documents on delete cascade`,
  `created_at timestamptz`, `to_addrs`, `cc_addrs`, `bcc_addrs text[]`, `subject text`, `body text`,
  `attachments text[]`, `ok bool`, `error text null`, `message_id text null`; index `(document_id, created_at desc)`.
- `smtp_failed.detail` is the SMTP server's response / transport error text (no credentials); everything else follows
  the usual error rules.

## UI

- Document detail (issued direction, non-draft): "Send by e-mail" button → dialog: locale switch (re-fetches the
  prefill; edits are replaced after a confirm if the subject/body were changed), To / Cc / Bcc as chip inputs,
  "Bcc to me" checkbox (adds/removes the company e-mail), subject, body (textarea), attachment checkboxes (PDF
  disabled when unavailable), Send. Disabled with a hint when not configured. Errors shown inline (`smtp_failed`
  with its detail). Success → toast, detail reloads (`sentAt`, history).
- Detail: "E-mail history" section (date, recipients, subject, result; expanding shows body + attachments + error).
- Settings → E-mail tab: configuration status (from, reply-to, configured yes/no), test e-mail form, template editor
  per locale (subject input + body textarea, variable reference list, Preview → rendered subject/body, Save, Restore
  default with confirm), `template_invalid` shown with its line message.

## Tests

- An in-process mock SMTP server (`tests/common/smtp.rs`, plain SMTP, `TLS=none`) captures messages and can be told
  to reject (5xx) — no external SMTP in CI.

## Clarifications (as implemented)
