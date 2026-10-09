import { ApiError } from "@/api/client";
import { hasExtension } from "@/features/imports/upload";

export const ACCEPTED_EXTENSIONS = [".csv", ".txt", ".xlsx"] as const;
export const ACCEPT_ATTR = ACCEPTED_EXTENSIONS.join(",");
export const SAMPLE_NAME = "import-sample.csv";

/** The one file to upload, or null when the pick is not exactly one supported file. */
export function pickFile(files: File[]): File | null {
  const [file, ...rest] = files;
  return file && rest.length === 0 && hasExtension(file, ACCEPTED_EXTENSIONS) ? file : null;
}

/** Row errors of the preview, plus the ones a confirm can still fail with. */
export const ROW_ERRORS = [
  "missing_field",
  "invalid_value",
  "invalid_date",
  "invalid_amount",
  "no_vat_rows",
  "total_mismatch",
  "not_allowed",
  "duplicate",
  "number_taken",
  "storage_unavailable",
  "internal",
] as const;

export const WARNINGS = ["contact_created", "category_created", "category_inactive", "related_not_found"] as const;

/** `fields.file` reasons of the file-level 422. */
export const FILE_ERRORS = ["invalid", "empty", "missing_column", "invalid_column", "too_many", "too_large"] as const;

const known = (list: readonly string[], code: string) => list.includes(code);

/** `not_allowed` columns with an unambiguous reason; any other column keeps the generic text. */
function notAllowedReason(field: string): string | null {
  if (field === "tax_date") return "taxDate";
  if (field === "paid_date") return "paidDate";
  if (field === "counterparty_name") return "counterparty";
  return /^(vat|base)_/.test(field) ? "vat" : null;
}

/** i18n key for a row error code (preview or confirm); unknown codes get the generic text with the code. */
export function rowErrorKey(code: string, field: string | null = null): string {
  const reason = code === "not_allowed" && field ? notAllowedReason(field) : null;
  if (reason) return `csvImport.notAllowed.${reason}`;
  return `csvImport.rowError.${known(ROW_ERRORS, code) ? code : "unknown"}`;
}

export const warningKey = (code: string) => `csvImport.warning.${known(WARNINGS, code) ? code : "unknown"}`;

/** i18n key + params for a rejected upload (413, or the file-level 422), or null for any other failure. */
export function uploadError(err: unknown): { key: string; params: Record<string, string> } | null {
  if (!(err instanceof ApiError)) return null;
  if (err.status === 413) return { key: "csvImport.upload.tooLarge", params: {} };
  const reason = err.status === 422 ? err.fields.file : undefined;
  if (!reason) return null;
  const key = `csvImport.fileError.${known(FILE_ERRORS, reason) ? reason : "unknown"}`;
  return { key, params: { code: reason, column: err.detail ?? "" } };
}
