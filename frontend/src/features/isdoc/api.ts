import { request, requestBlob } from "@/api/client";
import { filterParams, type DocumentFilterQuery } from "@/features/documents/api";
import type { ConfirmOptions, ConfirmResponse, PreviewResponse } from "./types";

function upload(files: File[], options?: ConfirmOptions): FormData {
  const body = new FormData();
  for (const file of files) body.append("files", file, file.name);
  if (options) body.append("options", JSON.stringify(options));
  return body;
}

export const isdocApi = {
  preview: (files: File[]) => request<PreviewResponse>("/api/import/isdoc/preview", { method: "POST", body: upload(files) }),
  /** Stateless: the server parses the same files again, so the caller re-sends them. */
  confirm: (files: File[], options: ConfirmOptions) =>
    request<ConfirmResponse>("/api/import/isdoc/confirm", { method: "POST", body: upload(files, options) }),
  /** `.isdocx` (ISDOC + PDF), or a plain `.isdoc` when there is no PDF to bundle. */
  document: (id: string) => requestBlob(`/api/documents/${encodeURIComponent(id)}/isdoc`),
  /** ZIP of every issued non-draft document matching the list filters (`direction=issued` is forced server-side). */
  bulk: (query: DocumentFilterQuery) => requestBlob(`/api/documents/isdoc?${filterParams(query).toString()}`),
};
