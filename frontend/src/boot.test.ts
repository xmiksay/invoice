import { afterEach, describe, expect, it, vi } from "vitest";
import { BASE_CONTEXT, mockFetch, SPACE_CONTEXT } from "@/test-utils";
import { guessBaseUrl, loadContext } from "./boot";

const loc = (url: string) => new URL(url) as unknown as Location;

describe("boot", () => {
  afterEach(() => vi.unstubAllGlobals());

  it("returns the base context", async () => {
    const fetch = mockFetch(200, BASE_CONTEXT);
    await expect(loadContext(loc("http://localhost:3000/"))).resolves.toEqual({ status: "ok", context: BASE_CONTEXT });
    expect(fetch.mock.calls[0]?.[0]).toBe("/api/context");
  });

  it("returns the space context", async () => {
    mockFetch(200, SPACE_CONTEXT);
    await expect(loadContext(loc("http://firma.localhost:3000/"))).resolves.toEqual({ status: "ok", context: SPACE_CONTEXT });
  });

  it("maps an unknown host (404) to not_found with a guessed base URL", async () => {
    mockFetch(404, { code: "not_found" });
    await expect(loadContext(loc("https://nope.invoiceapp.cz/invoices"))).resolves.toEqual({ status: "not_found", baseUrl: "https://invoiceapp.cz" });
  });

  it.each([0, 500, 502])("maps a failure (%i) to error", async (status) => {
    if (status === 0) vi.stubGlobal("fetch", vi.fn(async () => Promise.reject(new TypeError("down"))));
    else mockFetch(status, { code: "internal" });
    await expect(loadContext(loc("http://localhost:3000/"))).resolves.toEqual({ status: "error" });
  });

  it("guessBaseUrl drops the first label and keeps scheme and port", () => {
    expect(guessBaseUrl(loc("http://firma.localhost:5173/x"))).toBe("http://localhost:5173");
    expect(guessBaseUrl(loc("https://a.b.invoiceapp.cz/"))).toBe("https://b.invoiceapp.cz");
    expect(guessBaseUrl(loc("http://localhost/"))).toBe("http://localhost");
  });
});
