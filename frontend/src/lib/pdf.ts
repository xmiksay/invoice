import type { BlobResponse } from "@/api/client";

/**
 * The viewer tab keeps reading the object URL (reload, the PDF viewer's own "save"),
 * so it is revoked only after a grace period rather than once the tab has loaded.
 */
export const REVOKE_AFTER_MS = 5 * 60 * 1000;

function objectUrl(blob: Blob): string {
  const url = URL.createObjectURL(blob);
  setTimeout(() => URL.revokeObjectURL(url), REVOKE_AFTER_MS);
  return url;
}

function clickAnchor(href: string, download?: string): void {
  const a = document.createElement("a");
  a.href = href;
  if (download) a.download = download;
  a.rel = "noopener";
  document.body.appendChild(a);
  a.click();
  a.remove();
}

/**
 * Opens the PDF `load` resolves to in a new tab.
 *
 * The tab is opened synchronously, before the fetch: a `window.open` after an await has
 * lost the click's user activation and a slow render (mdcast, up to 60 s) gets it
 * popup-blocked. If the browser blocks even that (`window.open` → null), the current tab
 * navigates to the PDF; if the user closes the tab before the PDF arrives, nothing happens.
 * On failure the blank tab is closed and the error rethrown.
 */
export async function openPdf(load: () => Promise<BlobResponse>): Promise<void> {
  const win = window.open("", "_blank");
  try {
    const { blob } = await load();
    // The user closed the tab while waiting: they changed their mind, so the app tab stays put.
    if (win?.closed) return;
    const url = objectUrl(blob);
    if (win) win.location.href = url;
    else clickAnchor(url);
  } catch (err) {
    win?.close();
    throw err;
  }
}

/** Saves the file (a PDF, or an ISDOC / ZIP) under the server's filename, else `fallbackName`. */
export async function downloadPdf(load: () => Promise<BlobResponse>, fallbackName: string): Promise<void> {
  const { blob, filename } = await load();
  clickAnchor(objectUrl(blob), filename ?? fallbackName);
}
