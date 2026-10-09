import { ApiError } from "@/api/client";
import type { FieldErrors } from "@/api/types";
import { todayIso } from "@/features/documents/form";

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

/** 422 `filter: too_many` (more than 10 000 documents) → its explanation key. */
export const exportErrorKey = (err: unknown): string | null =>
  err instanceof ApiError && err.status === 422 && err.fields.filter === "too_many" ? "csvExport.tooMany" : null;
