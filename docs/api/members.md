# Members and invitations (4b)

Membership management for a space (#8); spaces, roles and tokens are in [spaces.md](spaces.md), accounts and
sessions in [auth.md](auth.md). All routes are on a **space host**.

## Who may do what
- **admin and owner** manage members and invitations; everyone else gets 403 `forbidden`.
- An **admin** may invite with, and assign, the roles `accountant`, `member`, `admin`; may change or remove members
  whose current role is not `owner`. Only an **owner** may grant `owner` or change / remove an owner.
- The **last owner** cannot be demoted, removed or leave (422 `role: last_owner` / 409 `{"code":"last_owner"}` —
  see routes).
- Any member may **leave** the space (except the last owner).
- Member management needs a **session or a token** with an effective role ≥ admin; the role ceiling of tokens
  applies as everywhere.

## Members
- `GET /api/members` (admin+) → `[ { userId, email, displayName, role, joinedAt, isSelf } ]`, by role (owner,
  admin, member, accountant) then e-mail.
- `PUT /api/members/{userId} { role }` (admin+) → the item. 422 `role: invalid | too_high` (granting above what the
  caller may grant), `role: last_owner` (demoting the last owner). Changing an owner as an admin, or an unknown /
  foreign user → 404 for unknown, 403 `forbidden` for an owner the caller may not touch. Changing one's own role is
  allowed under the same rules (an owner may step down while another owner exists).
- `DELETE /api/members/{userId}` (admin+) → 204. Self → use leave. Removing the last owner → 409 `last_owner`.
- `POST /api/space/leave` (any role, session only) → 204; last owner → 409 `last_owner`. The cookie is cleared.
- **Removal / leave revokes access at once:** the member's sessions on this space host and their API tokens in this
  space are deleted in the same transaction. A **role change** keeps sessions; tokens stay but are capped by the
  new role (4a rule). Documents the member created stay.

## Invitations
Table `space_invites` (id, space_id, email lowercased, role, token sha256, invited_by user, created_at,
expires_at; unique (space_id, email)).

- `GET /api/invites` (admin+) → pending, unexpired invitations `[ { id, email, role, invitedBy: { email,
  displayName }, createdAt, expiresAt } ]`, newest first. Expired rows are not listed (and are purged lazily).
- `POST /api/invites { email, role, locale? }` (admin+) → `201 { …item, url }`:
  - `url` = `{space url}/invite?token=…` — shown to the inviter once (copy button) **and** sent by e-mail through
    the instance SMTP (cs / en by `locale`, default cs; subject and text name the space and the inviter). No SMTP
    configured → the invitation is still created, the response carries `emailSent: false`; the e-mail is sent in
    a spawned task otherwise (`emailSent: true` = queued; failures logged).
  - Token: 32 random bytes base64url, stored sha256, **valid 7 days**, single use.
  - 422: `email: required | invalid | already_member`, `role: required | invalid | too_high`.
  - A pending invitation for the same e-mail is **replaced** (new role, new token, new expiry; the old link stops
    working).
- `POST /api/invites/{id}/resend` (admin+) → `200 { …item, url }`: new token + new 7-day expiry, e-mail again.
- `DELETE /api/invites/{id}` (admin+) → 204 (revoke). Unknown / foreign → 404.

## Accepting (space host, no auth)
- `GET /api/invites/accept?token=…` → `200 { space: { slug, name }, email, role, accountExists: boolean }`;
  invalid / expired / used → 404 `not_found`. (Rate limited per IP like the other token routes.)
- `POST /api/invites/accept { token, password, displayName? }` → **204 + session cookie** on this host:
  - `accountExists` true → `password` must be that account's password (failures count in the login bucket, 401
    `invalid_credentials` / 429); `displayName` ignored. A disabled account → 401 `invalid_credentials`.
  - `accountExists` false → creates the user with that e-mail (**verified** — the link proved it), `displayName`
    required (1–100), `password` 12–200. Works even when `INVOICE__REGISTRATION` is false.
  - Creates the membership with the invited role, deletes the invitation, creates the session — one transaction.
    Already a member meanwhile → the invitation is consumed, the role is not changed, the session is created.
  - Invalid / expired / used token → 422 `token: invalid`.
  - The CSRF Origin rule applies as for the other unauthenticated auth routes.
- The invitation is bound to its e-mail: the accepting account is always the one with that e-mail; there is no
  "accept with another account".

## UI (space host)
- Settings → **Členové** tab (admin+): members table (role select per row within the caller's rights, remove with
  confirm, "(vy)" marker), "Pozvat" form (e-mail, role) → shows the link with copy + "e-mail odeslán / neodeslán",
  pending invitations table (resend, revoke).
- Account page: **"Opustit space"** with confirm (hidden for the last owner, who sees why).
- `/invite?token=…` page (no login): space name + role; existing account → e-mail (read-only) + password →
  "Přijmout pozvánku"; new account → name + password (≥ 12) → "Vytvořit účet a přijmout". Success → the app home.
  Invalid link → a readable message with a link to the base host.

## Tests
- Integration: invite → accept with an existing account and with a new account (registration off), wrong
  password, expired / used / replaced token, already member; role rules (admin vs owner, too_high, last owner on
  demote / remove / leave); removal and leave delete the member's sessions and tokens in that space only (other
  spaces untouched); role change caps tokens; e-mail through the mock SMTP and `emailSent: false` without SMTP;
  isolation (another space's invite id / member id → 404).
- Frontend: members tab (rights-based controls), invite flow with link copy, invite accept page both branches,
  leave.

## Clarifications (as implemented)
(filled during the 4b implementation)
