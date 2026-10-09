import { request, requestBlob } from "@/api/client";
import type { ConfirmResponse } from "@/features/imports/types";
import type { CsvConfirmOptions, CsvPreviewResponse } from "./types";

function upload(file: File, options?: CsvConfirmOptions): FormData {
  const body = new FormData();
  body.append("file", file, file.name);
  if (options) body.append("options", JSON.stringify(options));
  return body;
}

export const csvImportApi = {
  preview: (file: File) => request<CsvPreviewResponse>("/api/import/csv/preview", { method: "POST", body: upload(file) }),
  /** Stateless: the server parses the same file again, so the caller re-sends it. */
  confirm: (file: File, options: CsvConfirmOptions) =>
    request<ConfirmResponse>("/api/import/csv/confirm", { method: "POST", body: upload(file, options) }),
  /** The header with the current VAT rate columns and three example rows. */
  sample: () => requestBlob("/api/import/csv/sample"),
};
