import type { Role } from "@/features/spaces/types";

/** `GET /api/tokens` row. */
export interface ApiToken {
  id: string;
  name: string;
  /** Public part shown in lists (`inv_{prefix}_…`). */
  prefix: string;
  role: Role;
  /** RFC 3339. */
  createdAt: string;
  /** `YYYY-MM-DD` (valid through the end of that day, UTC) or null = never. */
  expiresAt: string | null;
  lastUsedAt: string | null;
  /** Only in an admin+ listing (every token of the space). */
  user?: { email: string; displayName: string };
}

/** `POST /api/tokens` response: the secret is in here and nowhere else, ever. */
export interface CreatedToken extends ApiToken {
  token: string;
}

export interface CreateTokenBody {
  name: string;
  role: Role;
  expiresAt: string | null;
  /** Step-up, only for a user with TOTP. */
  code?: string;
}
