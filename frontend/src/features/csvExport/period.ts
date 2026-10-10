import { ApiError } from "@/api/client";
import type { FieldErrors } from "@/api/types";
import { todayIso } from "@/features/documents/form";
import { ACCOUNTANT_FORMATS, type AccountantQuery } from "./types";

/** The previous calendar month as `[from, to]` (`YYYY-MM-DD`, local dates); January → December of last year. */
export function previousMonth(now = new Date()): { from: string; to: string } {
  // Day 0 of the current month is the last day of the previous one; Date rolls the year over.
  const last = new Date(now.getFullYear(), now.getMonth(), 0);
  const first = new Date(last.getFullYear(), last.getMonth(), 1);
  return { from: todayIso(first), to: todayIso(last) };
}

/** Client checks mirroring the server's (both required, from ≤ to); the 366-day cap is left to the server. */
export function periodErrors(from: string, to: string): FieldErrors {
  const errors: FieldErrors = {};
  if (!from) errors.from = "required";
  if (!to) errors.to = "required";
  if (from && to && from > to) errors.to = "invalid";
  return errors;
}

/**
 * 422s that explain the whole export rather than a field → their explanation key:
 * `filter: too_many` (more than 10 000 documents), and from the XML formats `documents: unexportable`
 * (a document the program cannot take; the server's `detail` names it) and `from: empty` (nothing in the period).
 */
export function exportErrorKey(err: unknown): string | null {
  if (!(err instanceof ApiError) || err.status !== 422) return null;
  if (err.fields.filter === "too_many") return "csvExport.tooMany";
  if (err.fields.documents === "unexportable") return "csvExport.unexportable";
  if (err.fields.from === "empty") return "csvExport.empty";
  return null;
}

/** Filename when the response has no Content-Disposition: `ucetni-{from}-{to}.csv`, `pohoda-{from}-{to}.xml`, … */
export function accountantFallbackName({ from, to, format }: AccountantQuery): string {
  const { prefix, ext } = ACCOUNTANT_FORMATS.find((f) => f.format === format) ?? ACCOUNTANT_FORMATS[0];
  return `${prefix}-${from}-${to}.${ext}`;
}
