import { useAuthStore } from "@/stores/auth";
import type { ApiErrorBody, FieldErrors } from "./types";

/**
 * `status` 0 means the request never got a response (network down, CORS, abort).
 * `fields` maps camelCase wire field names to reason codes (422 `validation`).
 * `detail` is a human-readable diagnostic (502 `pdf_render_failed`: the typst error).
 */
export class ApiError extends Error {
  constructor(
    public readonly status: number,
    public readonly code: string,
    public readonly fields: FieldErrors = {},
    public readonly detail: string | null = null,
  ) {
    super(`API error ${status}: ${code}`);
    this.name = "ApiError";
  }
}

export interface RequestOptions extends Omit<RequestInit, "body"> {
  /** Serialized as JSON; a `FormData` goes out as multipart (the browser sets the boundary). */
  body?: unknown;
  /**
   * Explicit token instead of the stored one. Used to validate a candidate
   * token at login, so a 401 then must not log out / redirect.
   */
  token?: string;
}

// Registered by main.ts with the router. Kept as a hook rather than importing
// the router here to avoid a client → router → views → client import cycle.
let onUnauthorized: () => void = () => {};

export function setUnauthorizedHandler(handler: () => void): void {
  onUnauthorized = handler;
}

async function errorBody(res: Response): Promise<ApiErrorBody> {
  try {
    const body = (await res.json()) as Partial<ApiErrorBody>;
    if (typeof body.code === "string") {
      const fields = body.fields && typeof body.fields === "object" ? body.fields : undefined;
      const detail = typeof body.detail === "string" ? body.detail : undefined;
      return { code: body.code, fields, detail };
    }
  } catch {
    // Non-JSON error (e.g. proxy HTML page) — fall through to a generic code.
  }
  return { code: `http_${res.status}` };
}

/** Sends the request with the Bearer header; any non-2xx becomes an `ApiError`. */
async function send(path: string, options: RequestOptions, accept: string): Promise<Response> {
  const { body, token: explicitToken, headers: extraHeaders, ...init } = options;
  const auth = useAuthStore();
  const token = explicitToken ?? auth.token;

  const headers = new Headers(extraHeaders);
  headers.set("Accept", accept);
  if (token) headers.set("Authorization", `Bearer ${token}`);
  const multipart = body instanceof FormData;
  if (body !== undefined && !multipart) headers.set("Content-Type", "application/json");

  let res: Response;
  try {
    res = await fetch(path, {
      ...init,
      headers,
      body: body === undefined ? undefined : multipart ? body : JSON.stringify(body),
    });
  } catch {
    throw new ApiError(0, "network");
  }

  if (!res.ok) {
    const { code, fields, detail } = await errorBody(res);
    if (res.status === 401 && explicitToken === undefined) {
      auth.logout();
      onUnauthorized();
    }
    throw new ApiError(res.status, code, fields, detail ?? null);
  }
  return res;
}

export async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const res = await send(path, options, "application/json");
  if (res.status === 204) return undefined as T;
  return (await res.json()) as T;
}

export interface BlobResponse {
  blob: Blob;
  /** From `Content-Disposition`; null when the header carries none. */
  filename: string | null;
}

/**
 * Binary download (e.g. a PDF). A plain link cannot carry the Bearer token, so files
 * are fetched here and handed to the page as a blob.
 */
export async function requestBlob(path: string, options: RequestOptions = {}): Promise<BlobResponse> {
  const res = await send(path, options, "application/pdf, application/octet-stream, application/json");
  return { blob: await res.blob(), filename: filenameFromDisposition(res.headers.get("Content-Disposition")) };
}

/** `filename*=UTF-8''…` (RFC 5987) wins over `filename="…"`. */
export function filenameFromDisposition(header: string | null): string | null {
  if (!header) return null;
  const extended = /filename\*\s*=\s*(?:UTF-8|utf-8)''([^;]+)/.exec(header);
  if (extended?.[1]) {
    try {
      return decodeURIComponent(extended[1].trim());
    } catch {
      // Malformed escape — fall back to the plain parameter.
    }
  }
  const plain = /filename\s*=\s*(?:"([^"]*)"|([^;]+))/.exec(header);
  const name = (plain?.[1] ?? plain?.[2])?.trim();
  return name || null;
}
