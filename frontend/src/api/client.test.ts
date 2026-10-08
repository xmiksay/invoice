import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { ApiError, request, setUnauthorizedHandler } from "./client";
import { useAuthStore } from "@/stores/auth";
import { mockFetch, sentHeaders } from "@/test-utils";

describe("api client", () => {
  beforeEach(() => {
    localStorage.clear();
    setActivePinia(createPinia());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    setUnauthorizedHandler(() => {});
  });

  it("adds the stored Bearer token", async () => {
    useAuthStore().token = "secret";
    const fetch = mockFetch(200, { status: "ok", version: "1.0.0" });

    const res = await request<{ version: string }>("/api/health");

    expect(res.version).toBe("1.0.0");
    expect(sentHeaders(fetch).get("Authorization")).toBe("Bearer secret");
  });

  it("omits Authorization without a token and returns undefined on 204", async () => {
    const fetch = mockFetch(204);
    await expect(request("/api/x")).resolves.toBeUndefined();
    expect(sentHeaders(fetch).has("Authorization")).toBe(false);
  });

  it("serializes a JSON body", async () => {
    const fetch = mockFetch(200, {});
    await request("/api/x", { method: "POST", body: { a: 1 } });
    expect(fetch.mock.calls[0]?.[1]?.body).toBe('{"a":1}');
    expect(sentHeaders(fetch).get("Content-Type")).toBe("application/json");
  });

  it("parses the error code into ApiError", async () => {
    mockFetch(404, { code: "not_found" });
    await expect(request("/api/x")).rejects.toEqual(new ApiError(404, "not_found"));
  });

  it("falls back to http_<status> for non-JSON errors", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => new Response("<html>", { status: 502 })));
    await expect(request("/api/x")).rejects.toMatchObject({ status: 502, code: "http_502" });
  });

  it("maps a network failure to status 0", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => Promise.reject(new TypeError("Failed to fetch"))));
    await expect(request("/api/x")).rejects.toMatchObject({ status: 0, code: "network" });
  });

  it("on 401 clears the token and calls the unauthorized handler", async () => {
    const auth = useAuthStore();
    auth.token = "stale";
    const handler = vi.fn();
    setUnauthorizedHandler(handler);
    mockFetch(401, { code: "unauthorized" });

    await expect(request("/api/x")).rejects.toMatchObject({ status: 401, code: "unauthorized" });
    expect(auth.token).toBeNull();
    expect(handler).toHaveBeenCalledOnce();
  });

  it("on 401 with an explicit token leaves the session alone", async () => {
    const auth = useAuthStore();
    auth.token = "current";
    const handler = vi.fn();
    setUnauthorizedHandler(handler);
    const fetch = mockFetch(401, { code: "unauthorized" });

    await expect(request("/api/auth/check", { token: "candidate" })).rejects.toBeInstanceOf(ApiError);
    expect(sentHeaders(fetch).get("Authorization")).toBe("Bearer candidate");
    expect(auth.token).toBe("current");
    expect(handler).not.toHaveBeenCalled();
  });
});
