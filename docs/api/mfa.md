# TOTP two-factor authentication (4c)

Optional TOTP second factor per **user** (global, like the password), and a per-**space** policy that requires it
(#5). Accounts and sessions: [auth.md](auth.md); spaces and roles: [spaces.md](spaces.md); members and invitations:
[members.md](members.md).

## Config
- `INVOICE__SECRET_KEY` (**required**): 32 bytes, base64. Encrypts TOTP secrets at rest (AES-256-GCM, random
  96-bit nonce per value) and keys the HMAC of recovery codes. Missing / wrong length → startup fails. Losing or
  changing it makes every enrolled TOTP unusable (users recover with a recovery code, then enrol again). No
  rotation in 4c.

## TOTP parameters
RFC 6238: SHA-1, 6 digits, 30 s step, 20-byte random secret (base32 in the URI). Accepted window: the current step
±1. **Replay protection:** the user's last accepted step is stored; a code for a step ≤ that one is refused.
`otpauth://totp/Invoice%20({base host}):{email}?secret=…&issuer=Invoice%20({base host})&digits=6&period=30`.

## Recovery codes
10 codes `xxxxx-xxxxx` (base32 lowercase, 50 bits), shown **once** at enable / regenerate; stored as
HMAC-SHA256(secret key, normalized code) — normalized = lowercase, without `-` and spaces. Each works once wherever
a TOTP code is accepted (login, step-up). Regenerating replaces all.

## Enrolment (`/api/account/mfa`, session only — a token gets 403; on the base host or any space host)
- `GET /api/account/mfa` → `{ enabled, recoveryCodesLeft, requiredBy: [ { slug, name } ] }` (`requiredBy` = the
  user's spaces with the policy on).
- `POST /api/account/mfa/setup { password }` → `{ secret, otpauthUri }`; stores the secret as *pending* (encrypted,
  10 min); a new setup replaces it. Already enabled → 409 `{"code":"mfa_enabled"}`. Wrong password → 422
  `password: invalid` (login bucket).
- `POST /api/account/mfa/enable { code }` → `{ recoveryCodes: [10] }`; the pending secret becomes active. No / expired
  pending → 422 `code: expired`; wrong code → 422 `code: invalid`.
- `POST /api/account/mfa/disable { password, code }` → 204. Removes the secret and recovery codes. **Allowed even
  when a space requires MFA**: the user's sessions on those spaces' hosts are deleted and their tokens in those
  spaces stop working at once (401) — the UI lists `requiredBy` before confirming.
- `POST /api/account/mfa/recovery-codes { password, code }` → `{ recoveryCodes }`.
- Failed passwords and codes count in the user's login bucket (5 / 15 min → 429), [auth.md](auth.md).

## Login
- `POST /api/auth/login { email, password }`:
  - user without TOTP → as before (204 + session), except on a space with the policy → **403
    `{"code":"mfa_required"}`** (only after a correct password; nothing is created).
  - user with TOTP → **200 `{ "mfa": "required" }`** + a pending cookie `invoice_mfa` (host-only, HttpOnly,
    SameSite=Lax, Secure as the session cookie; 32 random bytes, stored sha256 with user + host, 5 min, single use).
    No session yet.
- `POST /api/auth/login/mfa { code }` (`code` = TOTP or a recovery code) → 204 + session cookie, pending cookie
  cleared. No / expired pending → 401 `invalid_credentials`. Wrong code → 401 `{"code":"mfa_invalid"}`; failures
  count in the login bucket per e-mail and IP; the pending login dies after 5 failures.
- The `me` response gains `user.mfaEnabled`.

## Space policy
- `GET /api/space` gains `requireMfa: boolean`. `PUT /api/space { name?, requireMfa? }`: `requireMfa` is **owner
  only** (an admin sending it → 403), and turning it on requires the owner's own TOTP (422 `requireMfa:
  mfa_not_enabled`).
- Effect: **from the next login**. Existing sessions and API tokens of members without TOTP keep working until they
  end / are revoked; a new login on that host without TOTP → 403 `mfa_required` (above). A user who later
  **disables** TOTP loses sessions and tokens in such spaces at once (above).
- `GET /api/members` items gain `mfaEnabled` (admin+), so the owner sees who would be locked out.

## Invitations into a space with the policy
- `GET /api/invites/accept` gains `requireMfa`.
- Existing account without TOTP → `POST /api/invites/accept` → 403 `mfa_required`; the invitation stays valid. The
  user enables TOTP on the base host (`/account`) and opens the link again.
- New account → the account is created (verified, with the given name and password) but **no membership and no
  session**; 403 `{"code":"mfa_required","detail":"account_created"}`; the invitation stays valid. The user logs
  in on the base host, enables TOTP, then opens the link again (now an existing account).
- Existing account with TOTP → `code` is required in the accept body (step-up, below).

## Step-up (re-confirm with a code)
For a user with TOTP these also require `code` (TOTP or recovery) next to the password they already take:
`POST /api/account/password` (`currentPassword` + `code`), `DELETE /api/space` (`slug`, `password`, `code`),
`POST /api/invites/accept` (existing account), `POST /api/account/mfa/disable`, `/recovery-codes`.
`POST /api/tokens` takes `code` too (no password) for a user with TOTP. Missing → 422 `code: required`, wrong → 422
`code: invalid` (login bucket). Users without TOTP: unchanged.

## UI
- Login: after the password, a code step ("Kód z aplikace" + "Použít záchranný kód"); `mfa_required` → message
  with a link to `{baseUrl}/account`.
- Account → "Dvoufázové ověření": off → enable (password → QR rendered in the browser from `otpauthUri` + the
  manual key → code → the 10 recovery codes with copy / download as .txt and an "uložil(a) jsem je" confirmation);
  on → codes left, regenerate, disable (lists `requiredBy`).
- Code fields on the step-up forms (password change, space delete, token create, invite accept) when
  `user.mfaEnabled`.
- Settings → Space (owner): "Vyžadovat dvoufázové ověření" toggle; disabled with a hint when the owner has no TOTP;
  before enabling, lists members without TOTP (from `mfaEnabled`) who will be refused at their next login.
- Settings → Členové: an "MFA" column.
- Invite page: `requireMfa` hint; the two `mfa_required` outcomes explained with a link to the base host.

## Tests
- Unit: RFC 6238 test vectors (SHA-1), window ±1, replay, recovery code normalisation / HMAC, AES-GCM round trip and
  tamper detection, secret key parsing.
- Integration: enrol (setup → enable, expired pending, wrong code), login with TOTP / recovery code / wrong code /
  replayed code / expired pending / rate limit, disable (incl. loss of sessions + tokens in requiring spaces),
  regenerate, space policy (owner only, owner needs TOTP, next login refused, existing sessions keep working), invite
  accept outcomes, step-up on every listed route.

## Clarifications (as implemented)
(filled during the 4c implementation)
