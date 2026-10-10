# Users, registration, login and sessions (4a)

Password accounts with server-side sessions; hosts and spaces are in [spaces.md](spaces.md). TOTP (#5) and Google
(#6) come later.

## Config
- `INVOICE__REGISTRATION` (`true` / `false`, default `false`): public registration on the base host. `true` requires
  SMTP to be configured (startup fails otherwise) — verification and reset e-mails go through the instance SMTP
  (2a), `From` = the configured sender.
- `INVOICE__TRUST_FORWARDED` (default `false`): take the client IP for rate limits from the first
  `X-Forwarded-For` hop (behind the reverse proxy); otherwise the socket address.
- `INVOICE__API_TOKEN` is removed (startup ignores it; `.env.example` and docs drop it).
- **Bootstrap = registration.** There is no CLI user / space creation; the first user registers like anyone else
  (enable registration, register, disable it if wanted).

## Users
Table `users` (id, email unique lowercased + trimmed, display_name, password_hash argon2id, email_verified_at,
disabled, created_at). Password: 12–200 chars (no other rule). `displayName` 1–100.

## Registration (base host, only when enabled)
- `POST /api/auth/register { email, password, displayName }` → **202** always for a valid body (no account
  enumeration). New e-mail → user created unverified + a verification e-mail with
  `{baseUrl}/verify?token=…`. Existing e-mail → no change; a "you already have an account" e-mail with the reset
  link instead. 422: `email: required | invalid`, `password: too_short | too_long`, `displayName: required |
  too_long`. Disabled → 404.
- `POST /api/auth/verify { token }` → 204 (sets `email_verified_at`); invalid / expired / used → 422
  `token: invalid`. Tokens: 32 random bytes, stored sha256, single use, 24 h.
- `POST /api/auth/verify/resend { email }` → 202 always (rate limited).
- An unverified user can log in to the base host but cannot create a space (403 `email_unverified`) and cannot log
  in to a space host.

## Login and sessions (base host and space hosts)
- `POST /api/auth/login { email, password }` → 204 + session cookie. On a space host the user must be a member of
  that space. Wrong e-mail, wrong password, not a member, disabled → the same 401 `{"code":"invalid_credentials"}`.
  Unverified on a space host → the same 401. Constant-time: an unknown e-mail still runs one argon2 verification.
- Cookie `invoice_session`: 32 random bytes base64url, **host-only** (no `Domain`), `HttpOnly`, `SameSite=Lax`,
  `Path=/`, `Secure` when `INVOICE__PUBLIC_URL` is https. The DB stores sha256 of it (table `sessions`: user_id,
  space_id (null on the base host), created_at, last_seen_at, expires_at, user_agent ≤ 200). A session is valid only
  on the host it was created for (the base host or its space).
- Expiry: idle **14 days** (sliding; `last_seen_at` written at most once a minute), absolute **90 days**.
- `POST /api/auth/logout` → 204, deletes the session, clears the cookie.
- `POST /api/auth/sessions/revoke-others` → 204, deletes all other sessions of the user (all hosts).
- `GET /api/auth/me` → `{ user: { id, email, displayName, emailVerified }, space: { slug, name, role } | null }`
  (session or token; 401 without).
- Moving between spaces = a link to the other host; each host needs its own login (no handoff).

## Password
- `POST /api/account/password { currentPassword, newPassword }` (session only) → 204; wrong current → 422
  `currentPassword: invalid`; revokes the user's other sessions.
- `POST /api/auth/password-reset { email }` (any known host) → 202 always; a known verified-or-not user gets
  `{baseUrl}/reset?token=…` (1 h, single use, sha256 stored).
- `POST /api/auth/password-reset/confirm { token, password }` → 204; sets the password, marks the e-mail verified
  (the link proved it), revokes **all** sessions. API tokens stay valid.

## CSRF
Cookie-authenticated `POST` / `PUT` / `PATCH` / `DELETE` must carry an `Origin` equal to the request's own origin
(`INVOICE__PUBLIC_URL` scheme + the request host + port); missing or different → 403 `{"code":"csrf"}`. Bearer
requests and the unauthenticated auth routes (`login`, `register`, `verify`, `password-reset*`) are checked the same
way when an `Origin` is present (a cross-site form post is refused).

## Rate limits (in memory, per instance)
429 `{"code":"rate_limited"}` with `Retry-After`:
- login: 5 failures / 15 min per e-mail, 20 / 15 min per IP;
- register, verify/resend, password-reset: 5 / hour per IP and 3 / hour per e-mail.

Nothing secret (password, cookie, token, reset / verify token) is ever logged.

## UI
- **Base host:** `/register` (when enabled), `/verify`, `/login`, `/forgot`, `/reset`, `/` = "Moje spaces" (list
  with links to each host, "Nový space" form with slug preview `{slug}.{base host}`), `/account`.
- **Space host:** `/login` (e-mail + password, link "Zapomenuté heslo"), then the existing app. The token-paste login
  screen is removed; the SPA uses the cookie (`credentials: "same-origin"`), never a stored token.
- 401 anywhere → `/login` with a return path; 403 `email_unverified` → a "verify your e-mail" screen with resend.
- E-mails (verify, already-registered, reset) in cs or en by the browser locale sent with the request
  (`locale` field, default cs); plain text templates in code.

## Tests
- Register → verify → login → create space → login on its host; disabled registration → 404; enumeration-safe 202.
- Login failures (wrong password, unknown e-mail, not a member, unverified, disabled) all 401 alike; rate limit 429.
- Session expiry (idle + absolute, by adjusting the stored timestamps), logout, revoke-others, cookie of the base
  host refused on a space host and vice versa.
- CSRF: cookie POST without / with a foreign Origin → 403; Bearer unaffected.
- Password change and reset (sessions revoked, token single use / expired).
- E-mails through the mock SMTP; the verify / reset tokens read from the captured mail.

## Clarifications (as implemented)
(filled during the 4a implementation)
