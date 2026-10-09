import type { PreviewEntry } from "@/features/imports/types";

/** Wire types for `/api/import/isdoc/*` (phase 1f-b, docs/api/isdoc.md); the preview / result shapes are shared. */

export type { ConfirmResponse, ConfirmResult, PreviewEntry, PreviewStatus, ResultStatus } from "@/features/imports/types";

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
