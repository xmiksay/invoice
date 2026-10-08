/** Server limit for an uploaded original PDF (413 `too_large` above it). */
export const MAX_ORIGINAL_BYTES = 20 * 1024 * 1024;

const isPdf = (file: File) => file.type === "application/pdf" || file.name.toLowerCase().endsWith(".pdf");

/** The checks the browser can make before uploading; the server still verifies the `%PDF-` header. */
export function checkOriginalFile(file: File): "notPdf" | "tooLarge" | null {
  if (!isPdf(file)) return "notPdf";
  return file.size > MAX_ORIGINAL_BYTES ? "tooLarge" : null;
}
