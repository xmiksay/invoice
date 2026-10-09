# MCP endpoint (phase 3a)

A Model Context Protocol server so an AI client (Claude Desktop / Code, …) can read documents and prepare and issue
invoices. It is a thin layer over the same services as the REST API — no business rule lives only here.

## Transport and auth

- **Streamable HTTP, stateless** at `POST /api/mcp` (crate `rmcp`, server side, `stateful_mode = false`): no
  `Mcp-Session-Id`, no server → client notifications, every request stands alone (works behind a proxy / with
  replicas). `GET` / `DELETE /api/mcp` → 405.
- Behind the same Bearer token as every `/api/*` route (`src/auth.rs`); without it → 401 before MCP handling.
- `initialize` answers `serverInfo { name: "invoice", version: <crate version> }`, capabilities `tools` only, and
  `instructions`: a short English description of the domain (directions, document types, statuses, money as decimal
  strings, how to issue an invoice: find / create the contact → create a draft → check totals → issue).
- Protocol version: whatever `rmcp` negotiates (≥ 2025-03-26).

## Tool conventions

- Tool names snake_case, English descriptions. Input schemas generated from Rust types (`schemars`), camelCase field
  names, same validation as REST.
- Results: one `text` content item holding the JSON (the REST wire shape, pretty-printed) **and** the same value as
  `structuredContent`. Money / rates are decimal strings as in REST.
- Errors: a tool error result (`isError: true`) whose text is the REST error JSON (`{"code":"…","fields":{…}}`,
  `detail` when present); protocol errors (unknown tool, schema mismatch) are JSON-RPC errors as `rmcp` produces them.
  Internal errors are logged and returned as `{"code":"internal"}`.
- Annotations: read tools `readOnlyHint: true`; write tools `readOnlyHint: false`; `issue_document`
  `destructiveHint: true` (assigns the number, renders and archives the PDF — irreversible); the others
  `destructiveHint: false`; `idempotentHint` true only for `mark_sent` and `update_draft`.
- **PDF is not returned through MCP.** Document results carry `pdfUrl: "/api/documents/{id}/pdf"` (relative to the
  server origin); the client downloads it with the same Bearer token.

## Tools

### Read (`readOnlyHint`)
| Tool | Input | Result |
|---|---|---|
| `list_documents` | `direction` (required), `docType?`, `status?`, `paymentState?`, `overdue?`, `contactId?`, `q?`, `from?`, `to?`, `limit?` (default 20, ≤ 100), `offset?` | `{ items: DocumentSummary[] (+ pdfUrl), total }` — as `GET /api/documents` |
| `get_document` | `id` | Document (+ `pdfUrl`, + `payments`) — issued or received |
| `list_contacts` | `q?`, `limit?` (≤ 100), `offset?` | `{ items: Contact[], total }` |
| `get_contact` | `id` | Contact |
| `lookup_ares` | `ico` | the ARES result as `GET /api/ares/{ico}` (404 / 502 as tool errors) |
| `list_catalog_items` | `q?` | active CatalogItem[] |
| `get_settings` | — | `{ company, bankAccounts, vatRates (active), numberSeries }` — what a draft needs (read only) |
| `compute_document` | as `POST /api/documents/compute` | lines with bases + totals (no write) |

### Write
| Tool | Input | Result |
|---|---|---|
| `create_contact` | the REST `ContactInput` | Contact (ARES is not called implicitly; use `lookup_ares` first) |
| `create_draft` | the REST `DocumentInput` for `docType` `invoice` \| `proforma` \| `simplified`, `direction` `issued` (others → `invalid_value` on `docType`); defaults as the UI: `issueDate` today, `dueDate` from the contact / company due days, `currency` CZK, `locale` from the contact / company, `vatMode` from the company, default bank account of the currency, `paymentMethod` `bank_transfer` | Document (draft) |
| `update_draft` | `id` + the full `DocumentInput` (replaces, as `PUT`) | Document; non-draft → `document_locked` |
| `issue_document` | `id` | Document (issued, with `number`, `pdfUrl`); every REST issue error as a tool error (`validation`, `pdf_unavailable`, `cnb_unavailable`, …) |
| `add_payment` | `id`, `date`, `amount`, `note?` | the payment + the document's new `paymentState` (DDPP creation for a proforma as in REST) |
| `mark_sent` | `id`, `sentAt?` | Document |

Not exposed (by decision): cancel, delete, credit / debit notes, settlement, e-mail sending, received documents
create / edit, imports / exports, settings changes.

## Implementation notes

- `src/mcp/`: the server (`rmcp` `ServerHandler` + `#[tool]` router), one file per tool group, sharing the REST
  handlers' service functions (extract them where a handler still holds logic). Errors map through `AppError`.
- Mounted in `app.rs` under the auth layer; the request body limit as for JSON routes.
- OpenAPI lists `POST /api/mcp` as an opaque JSON-RPC endpoint.

## Tests

- Integration tests drive `/api/mcp` over HTTP with raw JSON-RPC (`initialize`, `tools/list`, `tools/call`):
  auth required; `tools/list` names + annotations; each tool's happy path; a full flow (ARES-less contact → draft →
  compute → issue with the mdcast mock → payment → mark sent → get with `pdfUrl` → download the PDF over REST);
  validation errors surface as `isError` with the REST JSON; unknown tool → JSON-RPC error.

## Clarifications (as implemented)
