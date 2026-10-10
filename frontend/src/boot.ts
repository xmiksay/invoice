import { ApiError } from "@/api/client";
import { spacesApi } from "@/features/spaces/api";
import type { AppContext } from "@/features/spaces/types";

export type Boot =
  | { status: "ok"; context: AppContext }
  /** Unknown host (no such space): the 404 carries no base URL, so it is guessed. */
  | { status: "not_found"; baseUrl: string }
  /** Backend unreachable or broken: nothing to route to yet. */
  | { status: "error" };

/**
 * `firma.invoiceapp.cz:8443` → `https://invoiceapp.cz:8443`: drop the first host label
 * (the unknown slug). A deeper subdomain may guess wrong; it is only a convenience link.
 */
export function guessBaseUrl(loc: { protocol: string; hostname: string; port: string }): string {
  const labels = loc.hostname.split(".");
  const host = labels.length > 1 ? labels.slice(1).join(".") : loc.hostname;
  return `${loc.protocol}//${host}${loc.port ? `:${loc.port}` : ""}`;
}

/** Asks the backend which app this host serves (`GET /api/context`). */
export async function loadContext(loc: Location = window.location): Promise<Boot> {
  try {
    return { status: "ok", context: await spacesApi.context() };
  } catch (err) {
    if (err instanceof ApiError && err.status === 404) return { status: "not_found", baseUrl: guessBaseUrl(loc) };
    return { status: "error" };
  }
}
