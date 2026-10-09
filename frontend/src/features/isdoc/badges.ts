import type { PreviewStatus, ResultStatus } from "./types";

const GREEN = "!bg-green-100 !text-green-800 dark:!bg-green-900/50 dark:!text-green-300";
const GRAY = "!bg-gray-200 !text-gray-800 dark:!bg-gray-700 dark:!text-gray-200";
const RED = "!bg-red-100 !text-red-800 dark:!bg-red-900/50 dark:!text-red-300";

export const PREVIEW_CLASS: Record<PreviewStatus, string> = { ok: GREEN, duplicate: GRAY, error: RED };
export const RESULT_CLASS: Record<ResultStatus, string> = { imported: GREEN, skipped: GRAY, failed: RED };
export const CONTACT_CLASS = {
  existing: "",
  new: "!bg-amber-100 !text-amber-800 dark:!bg-amber-900/50 dark:!text-amber-300",
} as const;
