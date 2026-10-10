import type { Direction } from "@/features/documents/types";

/** `direction` of `GET /api/export/accountant`. */
export type AccountantDirection = Direction | "both";

/**
 * `format` of `GET /api/export/accountant`, in the select's order, with the fallback filename
 * (`{prefix}-{from}-{to}.{ext}`) used when the response carries no Content-Disposition.
 * Money S3 (3c) is one more entry here plus its `csvExport.accountant.formats.*` label.
 */
export const ACCOUNTANT_FORMATS = [
  { format: "csv", prefix: "ucetni", ext: "csv" },
  { format: "pohoda", prefix: "pohoda", ext: "xml" },
] as const;
export type AccountantFormat = (typeof ACCOUNTANT_FORMATS)[number]["format"];

/** Query of `GET /api/export/accountant`: a tax-date period, both bounds inclusive. */
export interface AccountantQuery {
  from: string;
  to: string;
  direction: AccountantDirection;
  format: AccountantFormat;
}
