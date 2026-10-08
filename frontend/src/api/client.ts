import { useAuthStore } from "@/stores/auth";
import type { ApiErrorBody } from "./types";

/** `status` 0 means the request never got a response (network down, CORS, abort). */
export class ApiError extends Error {
  constructor(
    public readonly status: number,
    public readonly code: string,
  ) {
    super(`API error ${status}: ${code}`);
    this.name = "ApiError";
  }
}

export interface RequestOptions extends Omit<RequestInit, "body"> {
  /** Serialized as JSON. */
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

async function errorCode(res: Response): Promise<string> {
  try {
    const body = (await res.json()) as Partial<ApiErrorBody>;
    if (typeof body.code === "string") return body.code;
  } catch {
    // Non-JSON error (e.g. proxy HTML page) — fall through to a generic code.
  }
  return `http_${res.status}`;
}

export async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const { body, token: explicitToken, headers: extraHeaders, ...init } = options;
  const auth = useAuthStore();
  const token = explicitToken ?? auth.token;

  const headers = new Headers(extraHeaders);
  headers.set("Accept", "application/json");
  if (token) headers.set("Authorization", `Bearer ${token}`);
  if (body !== undefined) headers.set("Content-Type", "application/json");

  let res: Response;
  try {
    res = await fetch(path, {
      ...init,
      headers,
      body: body === undefined ? undefined : JSON.stringify(body),
    });
  } catch {
    throw new ApiError(0, "network");
  }

  if (!res.ok) {
    const code = await errorCode(res);
    if (res.status === 401 && explicitToken === undefined) {
      auth.logout();
      onUnauthorized();
    }
    throw new ApiError(res.status, code);
  }

  if (res.status === 204) return undefined as T;
  return (await res.json()) as T;
}
