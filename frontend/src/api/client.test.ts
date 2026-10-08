import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { ApiError, filenameFromDisposition, request, requestBlob, setUnauthorizedHandler } from "./client";
import { useAuthStore } from "@/stores/auth";
import { mockFetch, mockFetchRoutes, pdfReply, reply, sentHeaders } from "@/test-utils";

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

  it("carries 422 validation fields", async () => {
    mockFetch(422, { code: "validation", fields: { ico: "invalid_ico", name: "required" } });
    await expect(request("/api/x")).rejects.toMatchObject({
      status: 422,
      code: "validation",
      fields: { ico: "invalid_ico", name: "required" },
    });
  });

  it("defaults fields to an empty object", async () => {
    mockFetch(409, { code: "conflict" });
    await expect(request("/api/x")).rejects.toMatchObject({ fields: {} });
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

  describe("requestBlob", () => {
    it("sends the Bearer token and returns the blob with the filename", async () => {
      useAuthStore().token = "secret";
      const fetch = mockFetchRoutes({ "GET /api/documents/d1/pdf": pdfReply("20260001.pdf", "attachment") });

      const res = await requestBlob("/api/documents/d1/pdf?download=1");

      expect(fetch.mock.calls[0]?.[0]).toBe("/api/documents/d1/pdf?download=1");
      expect(sentHeaders(fetch).get("Authorization")).toBe("Bearer secret");
      expect(sentHeaders(fetch).get("Accept")).toContain("application/pdf");
      expect(res.blob.type).toBe("application/pdf");
      expect(await res.blob.text()).toBe("%PDF-1.7");
      expect(res.filename).toBe("20260001.pdf");
    });

    it("has a null filename without Content-Disposition", async () => {
      mockFetchRoutes({ "GET /api/pdf/preview": pdfReply() });
      expect((await requestBlob("/api/pdf/preview?locale=cs")).filename).toBeNull();
    });

    it("turns a JSON error into ApiError with its detail", async () => {
      mockFetchRoutes({ "GET /api/pdf/preview": reply(502, { code: "pdf_render_failed", detail: "error: unknown variable: foo" }) });
      await expect(requestBlob("/api/pdf/preview")).rejects.toEqual(
        new ApiError(502, "pdf_render_failed", {}, "error: unknown variable: foo"),
      );
    });

    it("leaves detail null when the body has none", async () => {
      mockFetchRoutes({ "GET /api/pdf/preview": reply(503, { code: "pdf_unavailable" }) });
      await expect(requestBlob("/api/pdf/preview")).rejects.toMatchObject({ status: 503, code: "pdf_unavailable", detail: null });
    });
  });

  it("filenameFromDisposition prefers filename* and tolerates missing parts", () => {
    expect(filenameFromDisposition(null)).toBeNull();
    expect(filenameFromDisposition("inline")).toBeNull();
    expect(filenameFromDisposition('attachment; filename="a b.pdf"')).toBe("a b.pdf");
    expect(filenameFromDisposition("attachment; filename=plain.pdf")).toBe("plain.pdf");
    expect(filenameFromDisposition("attachment; filename=\"x.pdf\"; filename*=UTF-8''%C5%BE.pdf")).toBe("ž.pdf");
  });
});
