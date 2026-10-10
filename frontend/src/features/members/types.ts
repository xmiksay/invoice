import type { Role } from "@/features/spaces/types";

/** `GET /api/members` row (docs/api/members.md). */
export interface Member {
  userId: string;
  email: string;
  displayName: string;
  role: Role;
  /** RFC 3339. */
  joinedAt: string;
  isSelf: boolean;
  /** The member has TOTP on (the listing is admin+ only). */
  mfaEnabled: boolean;
}

/** `GET /api/invites` row: a pending, unexpired invitation. */
export interface Invite {
  id: string;
  email: string;
  role: Role;
  invitedBy: { email: string; displayName: string };
  createdAt: string;
  expiresAt: string;
}

/** `POST /api/invites` and `POST /api/invites/{id}/resend`: the link is returned only here. */
export interface SentInvite extends Invite {
  url: string;
  /** false = no SMTP on the instance; the inviter has to pass the link on. */
  emailSent: boolean;
}

export interface InviteBody {
  email: string;
  role: Role;
  /** Language of the invitation e-mail. */
  locale: string;
}

/** `GET /api/invites/accept?token=…` */
export interface InviteInfo {
  space: { slug: string; name: string };
  email: string;
  role: Role;
  accountExists: boolean;
  /** The space requires TOTP: a member needs it before joining. */
  requireMfa: boolean;
}

export interface AcceptInviteBody {
  token: string;
  password: string;
  /** Only for a new account. */
  displayName?: string;
  /** Step-up, only for an existing account with TOTP. */
  code?: string;
}
