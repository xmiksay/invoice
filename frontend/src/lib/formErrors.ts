import { ApiError } from "@/api/client";
import type { FieldErrors } from "@/api/types";

export const REASON_CODES = [
  "required",
  "invalid",
  "too_long",
  "duplicate",
  "invalid_ico",
  "invalid_pattern",
  "below_issued",
] as const;

/** Field errors of a 422 `validation` response, or null for any other failure. */
export function fieldErrorsOf(err: unknown): FieldErrors | null {
  if (err instanceof ApiError && err.status === 422 && Object.keys(err.fields).length > 0) {
    return { ...err.fields };
  }
  return null;
}

/** i18n key for a reason code; unknown codes degrade to the generic "invalid". */
export function reasonKey(code: string): string {
  return (REASON_CODES as readonly string[]).includes(code)
    ? `validation.${code}`
    : "validation.invalid";
}

/** Statuses a reverse proxy / the Vite dev proxy returns when the backend is down. */
const UNREACHABLE = new Set([0, 502, 503, 504]);

const KNOWN_CODES: Record<string, string> = {
  not_found: "errors.notFound",
  conflict: "errors.conflict",
  validation: "errors.validation",
  ares_not_found: "errors.aresNotFound",
  ares_unavailable: "errors.aresUnavailable",
  document_locked: "errors.documentLocked",
  invalid_state: "errors.invalidState",
  cnb_unavailable: "errors.cnbUnavailable",
};

/** i18n key + params for a non-field error message. */
export function errorMessageKey(err: unknown): { key: string; params?: Record<string, string> } {
  if (err instanceof ApiError) {
    const known = KNOWN_CODES[err.code];
    if (known) return { key: known };
    if (UNREACHABLE.has(err.status)) return { key: "errors.unreachable" };
    return { key: "errors.unexpected", params: { code: err.code } };
  }
  return { key: "errors.unexpected", params: { code: "unknown" } };
}

/** Builds field errors from simple predicate checks: `{ field: reason | null }`. */
export function collectErrors(checks: Record<string, string | null | false>): FieldErrors {
  const out: FieldErrors = {};
  for (const [field, reason] of Object.entries(checks)) {
    if (reason) out[field] = reason;
  }
  return out;
}

/** Common rule for a text field: required + max length. */
export function textRule(value: string, opts: { required?: boolean; max?: number }): string | null {
  const trimmed = value.trim();
  if (opts.required && trimmed === "") return "required";
  if (opts.max !== undefined && trimmed.length > opts.max) return "too_long";
  return null;
}

/** "" → null for nullable wire strings. */
export function nullIfEmpty(value: string): string | null {
  const trimmed = value.trim();
  return trimmed === "" ? null : trimmed;
}
