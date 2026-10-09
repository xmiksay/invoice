import type { Direction, DocumentType } from "@/features/documents/types";

/** Wire types for `/api/import/isdoc/*` (phase 1f-b, docs/api/isdoc.md). Decimals are strings, dates `YYYY-MM-DD`. */

export type PreviewStatus = "ok" | "duplicate" | "error";

export interface PreviewEntry {
  /** Path inside the upload (`a.zip/b.zip/x.isdoc`), unique per response (a repeated path gets `#2`, `#3`, …); echoed back in `selected`. */
  key: string;
  status: PreviewStatus;
  error: string | null;
  warnings: string[];
  direction: Direction | null;
  docType: DocumentType | null;
  number: string | null;
  counterparty: { name: string; ico: string | null } | null;
  contactMatch: "existing" | "new" | null;
  issueDate: string | null;
  taxPointDate: string | null;
  dueDate: string | null;
  currency: string | null;
  total: string | null;
  hasPdf: boolean;
  relatedNumber: string | null;
  relatedFound: boolean;
}

export interface PreviewResponse {
  entries: PreviewEntry[];
}

/** The JSON `options` part of `confirm`. `categoryId` / `vatDeductible` apply to received entries only. */
export interface ConfirmOptions {
  selected: string[];
  markPaid: boolean;
  categoryId: string | null;
  vatDeductible: boolean;
}

export type ResultStatus = "imported" | "skipped" | "failed";

export interface ConfirmResult {
  key: string;
  status: ResultStatus;
  documentId: string | null;
  error: string | null;
}

export interface ConfirmResponse {
  results: ConfirmResult[];
}
