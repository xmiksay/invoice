/** Server limit for one import upload (413 `too_large` above it). */
export const MAX_UPLOAD_BYTES = 50 * 1024 * 1024;

export const totalBytes = (files: File[]) => files.reduce((sum, f) => sum + f.size, 0);

/** Case-insensitive file name extension check (`.csv`, `.isdocx`, …). */
export const hasExtension = (file: File, extensions: readonly string[]) =>
  extensions.some((ext) => file.name.toLowerCase().endsWith(ext));
