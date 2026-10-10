# Spaces, roles, data scoping and API tokens (4a)

The instance moves from one static Bearer token to **spaces** (one space = one own company = one accounting unit),
**users** with memberships and **personal API tokens**. `INVOICE__API_TOKEN` is removed. Login, registration and
sessions are in [auth.md](auth.md).

## Hosts
- `INVOICE__PUBLIC_URL` (required, e.g. `https://invoiceapp.cz`; dev `http://localhost:3000`) gives the scheme, the
  **base host** and an optional port. A space lives at `{scheme}://{slug}.{base host}{:port}`
  (`https://firma.invoiceapp.cz`; dev `http://firma.localhost:3000`). DNS / wildcard TLS / the reverse proxy are
  deployment concerns; the proxy must pass the original `Host`.
- Every request is classified by its `Host` (lowercased, port ignored):
  - **base host** → the *base context*: registration, login, the "my spaces" hub, account routes ([auth.md](auth.md));
    business routes → 404 `not_found`.
  - **`{slug}.{base host}`** of an existing space → the *space context*: every business route of phases 1–3, the
    space's login, account and token routes.
  - anything else (unknown slug, deeper subdomain, foreign host) → 404 `{"code":"not_found"}` for every `/api` route.
  - `/api/health` and `/api/openapi.json` answer on any host, without auth.
- The SPA is served on every known host; it asks `GET /api/context` to know which app to show.

`GET /api/context` (no auth) → `200 { kind: "base" | "space", space: { slug, name } | null,
registration: boolean, baseUrl: string }` — `registration` = the instance switch ([auth.md](auth.md)); `baseUrl` =
`INVOICE__PUBLIC_URL`. Unknown host → 404.

## Spaces
Table `spaces` (id uuid, slug unique, name, created_at). Slug: `[a-z0-9-]`, 3–30 chars, no leading / trailing `-`,
not reserved (`www`, `api`, `app`, `admin`, `mail`, `smtp`, `static`, `assets`, `cdn`, `status`, `docs`, `help`,
`support`, `blog`), **immutable**. `name` 1–200 chars.

Base context (session of a verified user, [auth.md](auth.md)):
- `GET /api/spaces` → `[ { slug, name, role, url } ]` — the user's memberships, by name.
- `POST /api/spaces { slug, name }` → `201 { slug, name, role: "owner", url }`. 422: `slug: required | invalid |
  reserved | taken`, `name: required | too_long`. No limit on the number of spaces per user. Creation also creates
  everything a fresh instance used to get from migrations (company profile row, default VAT rates, number series,
  e-mail templates, accounting settings, … — every former global seed, now per space).

Space context:
- `GET /api/space` (any role) → `{ slug, name, url, role }` (the caller's role).
- `PUT /api/space { name }` (admin+) → the same shape.
- `DELETE /api/space { slug, password }` (owner, **session only** — a token gets 403 `forbidden`) → `204`.
  `slug` must equal the space slug (422 `slug: mismatch`), `password` must be the caller's password (422
  `password: invalid`). Deletes in one transaction every row of the space (all tables, memberships, tokens, its
  sessions), then deletes the storage prefix `spaces/{id}/`. A storage failure after the commit is logged at
  `error` and does not fail the request (orphaned files only). The slug becomes free again.

## Memberships and roles
Table `space_members` (space_id, user_id, role, created_at; unique pair). The creator is `owner`. Invitations and
member management are phase 4b; in 4a a space has exactly its creator.

| role | may |
|---|---|
| `accountant` | read everything, PDFs, ISDOC / CSV / Pohoda / Money exports; own tokens (role `accountant`) |
| `member` | + create / change documents, contacts, catalog, payments, imports, e-mails, MCP writes |
| `admin` | + everything under Settings (company, accounts, VAT, series, categories, custom fields, design, e-mail, accounting), `PUT /api/space`, list / revoke everyone's tokens |
| `owner` | + `DELETE /api/space` (members management comes in 4b) |

- Enforcement is per route by an extractor (minimum role); GET / HEAD routes need `accountant`, every mutation needs
  at least `member`, settings mutations `admin`. POST routes that only read (`/compute`, previews) need `member`.
  Insufficient role → **403 `{"code":"forbidden"}`**.
- MCP: read tools need `accountant`, write tools `member`; a forbidden tool call returns the MCP tool error
  `{"code":"forbidden"}`.
- The effective role of a request = the membership role (session) or `min(token role, membership role)` (token).
  No membership (removed, or the user disabled) → 401.

## Data scoping
- Every **root** table gets `space_id uuid NOT NULL REFERENCES spaces ON DELETE CASCADE`: company (one row per
  space), bank_accounts, vat_rates, number_series, contacts, documents, catalog_items, catalog_groups, categories,
  custom_fields, e-mail templates / settings / log, accounting_settings (one row per space), import batches if
  stored, and any other table not owned by a parent row. **Child** tables (document lines, VAT recap, payments,
  number series counters, catalog group members, …) are scoped through their parent and get no `space_id`.
- Global (no space): `exchange_rates` (ČNB), ARES lookups, SMTP / mdcast configuration, `users`, `sessions`.
- Unique indexes become per space (document number per series, contact IČO, VAT rate, series code, category name,
  custom field key, …).
- **Repo API shape enforces it:** every repo function on space data takes a `SpaceId` (from the request extractor,
  never from client input); root queries filter `space_id`, child queries join or check through the parent. A
  referenced id from the client (contactId, bankAccountId, categoryId, catalog item, original document, …) that is
  not in the current space behaves exactly like a non-existent id (same 404 / 422 as today). Another space's id in
  a URL → 404.
- Migration: append-only; adds the new tables and the NOT NULL columns. It **aborts with a clear error if any
  space-owned table has rows** (there is no default space; reset the dev database). Former global seed rows are
  removed by the migration and created per space instead.

## Storage
All keys move under `spaces/{space_id}/`: `…/documents/{year}/…` (issued-PDF archive, received originals) and
`…/design/…`. **The mdcast design overlay is per space:** a render reads `spaces/{id}/design/` over the embedded
default, file by file, exactly as 1d / #17 did instance-wide (Settings → Design works per space unchanged).
`invoice design push|pull|ls` gain a required `--space <slug>`. No file migration (no existing data).

## Personal API tokens
For MCP and scripts. A token belongs to **one user in one space** and works only on that space's host.

- Format `inv_{prefix}_{secret}`: `prefix` 8 chars, secret 32 random bytes base64url; stored as sha256 of the whole
  token plus the prefix (shown in lists). Shown **once** at creation.
- `Authorization: Bearer <token>` on the space host → user + space + effective role. Wrong host, unknown, revoked or
  expired token → 401. Bearer requests are exempt from the CSRF Origin check ([auth.md](auth.md)).
- Routes (space context, session or token):
  - `GET /api/tokens` → own tokens `[ { id, name, prefix, role, createdAt, expiresAt, lastUsedAt } ]`; admin+ get
    every token in the space with `user: { email, displayName }`.
  - `POST /api/tokens { name, role, expiresAt? }` → `201 { …, token }`. `role` ≤ the caller's role (422
    `role: too_high`), `name` 1–100, `expiresAt` a future date or null.
  - `DELETE /api/tokens/{id}` → 204; own token, or any token in the space for admin+; else 404.
- `lastUsedAt` is updated at most once a minute per token.

## UI (space host)
- Header: space name, user menu (Account, API tokens, "Moje spaces" → the base host hub, Log out).
- Account page: change password, log out other sessions ([auth.md](auth.md)).
- API tokens page: list, create (name, role, expiry; the token shown once with a copy button), revoke.
- Settings → Space (admin+): name; owner: "Smazat space" with the archiving warning (tax documents must be kept),
  a link to the accountant export, confirmation by typing the slug + password.
- Routes the role may not use are hidden; a 403 still shows a readable message.

## Tests
- Isolation: two spaces with identical data never see each other's records (list, get, update, delete, number
  allocation, exports, MCP, PDFs, storage keys); a foreign id in a URL or a reference → as non-existent.
- Hosts: base vs space vs unknown; token on a foreign space host → 401.
- Roles: the matrix above per route group (incl. MCP tools); token role ceiling.
- Space create (slug rules, seeds present), delete (mismatch, wrong password, token → 403, data and storage gone).
- Every existing integration test runs inside a fresh space through a token or session (`tests/common`).

## Clarifications (as implemented)
(filled during the 4a implementation)
