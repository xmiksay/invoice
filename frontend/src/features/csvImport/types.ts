import type { MatchKind, PreviewEntry } from "@/features/imports/types";

/** Wire types for `/api/import/csv/*` (phase 2b, docs/api/csv.md): the ISDOC preview shape plus the row fields. */

export interface CsvPreviewEntry extends PreviewEntry {
  /** 1-based line / sheet row number (header = 1); the key is `row:{row}`. */
  row: number;
  /** The CSV header name an error refers to. */
  field: string | null;
  categoryMatch: MatchKind | null;
}

export interface CsvPreviewResponse {
  entries: CsvPreviewEntry[];
}

/** The JSON `options` part of `confirm`. */
export interface CsvConfirmOptions {
  selected: string[];
}
