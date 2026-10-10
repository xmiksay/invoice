import type { Role } from "@/features/spaces/types";

/** `GET /api/auth/me` */
export interface Me {
  user: { id: string; email: string; displayName: string; emailVerified: boolean; mfaEnabled: boolean };
  /** Null on the base host. */
  space: { slug: string; name: string; role: Role } | null;
}

export interface LoginBody {
  email: string;
  password: string;
}

export interface RegisterBody {
  email: string;
  password: string;
  displayName: string;
  /** Language of the verification e-mail. */
  locale: string;
}

/** `POST /api/auth/login` 200: the password was right, a TOTP / recovery code must follow. */
export interface LoginMfaRequired {
  mfa: "required";
}

export interface ChangePasswordBody {
  currentPassword: string;
  newPassword: string;
  /** Step-up, only for a user with TOTP. */
  code?: string;
}

/** `GET /api/account/mfa` */
export interface MfaStatus {
  enabled: boolean;
  recoveryCodesLeft: number;
  /** The user's spaces whose policy requires TOTP. */
  requiredBy: { slug: string; name: string }[];
}

/** `POST /api/account/mfa/setup` */
export interface MfaSetup {
  /** Base32, for manual entry. */
  secret: string;
  otpauthUri: string;
}

/** `POST /api/account/mfa/enable` and `/recovery-codes`: shown once, never stored by the client. */
export interface RecoveryCodes {
  recoveryCodes: string[];
}

/** `POST /api/account/mfa/disable` and `/recovery-codes`. */
export interface MfaConfirmBody {
  password: string;
  code: string;
}
