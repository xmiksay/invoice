import { afterEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { mockFetch } from "@/test-utils";
import { isdocApi } from "./api";
import { isdocFile } from "./testData";

describe("isdocApi", () => {
  setActivePinia(createPinia());
  afterEach(() => vi.unstubAllGlobals());

  it("preview posts every file as a `files` part", async () => {
    const fetch = mockFetch(200, { entries: [] });
    await isdocApi.preview([isdocFile("a.isdoc"), isdocFile("b.zip")]);
    const [url, init] = fetch.mock.calls[0]!;
    expect(url).toBe("/api/import/isdoc/preview");
    expect(init?.method).toBe("POST");
    const body = init?.body as FormData;
    expect((body.getAll("files") as File[]).map((f) => f.name)).toEqual(["a.isdoc", "b.zip"]);
    expect(body.has("options")).toBe(false);
  });

  it("confirm re-sends the files with the options JSON part", async () => {
    const fetch = mockFetch(200, { results: [] });
    const options = { selected: ["a.isdoc"], markPaid: false, categoryId: "cat1", vatDeductible: true };
    await isdocApi.confirm([isdocFile("a.isdoc")], options);
    const [url, init] = fetch.mock.calls[0]!;
    expect(url).toBe("/api/import/isdoc/confirm");
    const body = init?.body as FormData;
    expect((body.getAll("files") as File[]).map((f) => f.name)).toEqual(["a.isdoc"]);
    expect(JSON.parse(body.get("options") as string)).toEqual(options);
  });

  it("exports one document and the list filter (no paging)", async () => {
    const fetch = mockFetch(200, {});
    await isdocApi.document("d 1");
    await isdocApi.bulk({ direction: "issued", docType: "invoice", overdue: false, q: " acme ", from: "2026-01-01" });
    expect(fetch.mock.calls[0]![0]).toBe("/api/documents/d%201/isdoc");
    const bulk = new URL(String(fetch.mock.calls[1]![0]), "http://x");
    expect(bulk.pathname).toBe("/api/documents/isdoc");
    expect(Object.fromEntries(bulk.searchParams)).toEqual({ direction: "issued", docType: "invoice", q: "acme", from: "2026-01-01" });
  });
});
