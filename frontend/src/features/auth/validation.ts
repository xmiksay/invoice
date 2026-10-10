import type { FieldErrors } from "@/api/types";
import { collectErrors, textRule } from "@/lib/formErrors";

export const PASSWORD_MIN = 12;
export const PASSWORD_MAX = 200;

export function passwordRule(value: string): string | null {
  if (value.length < PASSWORD_MIN) return value === "" ? "required" : "too_short";
  if (value.length > PASSWORD_MAX) return "too_long";
  return null;
}

// Deliberately loose: the server decides; this only catches obvious typos before a round trip.
export function emailRule(value: string): string | null {
  const trimmed = value.trim();
  if (trimmed === "") return "required";
  return /^[^\s@]+@[^\s@]+$/.test(trimmed) ? null : "invalid";
}

export function validateRegistration(form: { email: string; password: string; displayName: string }): FieldErrors {
  return collectErrors({
    email: emailRule(form.email),
    password: passwordRule(form.password),
    displayName: textRule(form.displayName, { required: true, max: 100 }),
  });
}

/**
 * The server answers a wrong password with the generic `invalid` reason on a password field
 * (`currentPassword`, `password`); there it can only mean the password was wrong.
 */
export function passwordFieldReason(reason: string | undefined): string | undefined {
  return reason === "invalid" ? "wrong_password" : reason;
}

/** Same for a step-up `code` field: `invalid` there means a wrong TOTP / recovery code. */
export function codeFieldReason(reason: string | undefined): string | undefined {
  return reason === "invalid" ? "wrong_code" : reason;
}

/** Only presence is checked: a TOTP and a recovery code differ in format, the server decides. */
export function codeRule(value: string): string | null {
  return value.trim() === "" ? "required" : null;
}
