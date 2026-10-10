import type { Role } from "@/features/spaces/types";

/** `GET /api/auth/me` */
export interface Me {
  user: { id: string; email: string; displayName: string; emailVerified: boolean };
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

export interface ChangePasswordBody {
  currentPassword: string;
  newPassword: string;
}
