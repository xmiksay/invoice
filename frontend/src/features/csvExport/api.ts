import { requestBlob } from "@/api/client";
import { filterParams, type DocumentFilterQuery } from "@/features/documents/api";
import type { AccountantQuery } from "./types";

export const csvExportApi = {
  /** Every non-draft, non-cancelled document matching the list filters (`limit` / `offset` are not sent). */
  list: (query: DocumentFilterQuery) => requestBlob(`/api/export/csv?${filterParams(query).toString()}`),
  /** Both directions (or one) by tax date, proformas excluded; CSV or an accounting program's XML. */
  accountant: ({ from, to, direction, format }: AccountantQuery) =>
    requestBlob(`/api/export/accountant?${new URLSearchParams({ from, to, direction, format }).toString()}`),
};
