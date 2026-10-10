import { afterEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { mockFetch, sentHeaders } from "@/test-utils";
import { csvImportApi } from "./api";
import { csvFile } from "./testData";

describe("csvImportApi", () => {
  setActivePinia(createPinia());
  afterEach(() => vi.unstubAllGlobals());

  it("preview posts the file as one `file` part", async () => {
    const fetch = mockFetch(200, { entries: [] });
    await csvImportApi.preview(csvFile("a.csv"));
    const [url, init] = fetch.mock.calls[0]!;
    expect(url).toBe("/api/import/csv/preview");
    expect(init?.method).toBe("POST");
    const body = init?.body as FormData;
    expect((body.getAll("file") as File[]).map((f) => f.name)).toEqual(["a.csv"]);
    expect(body.has("options")).toBe(false);
  });

  it("confirm re-sends the file with the options as a text part", async () => {
    const fetch = mockFetch(200, { results: [] });
    await csvImportApi.confirm(csvFile("a.xlsx"), { selected: ["row:2"] });
    const [url, init] = fetch.mock.calls[0]!;
    expect(url).toBe("/api/import/csv/confirm");
    const body = init?.body as FormData;
    expect((body.getAll("file") as File[]).map((f) => f.name)).toEqual(["a.xlsx"]);
    expect(typeof body.get("options")).toBe("string");
    expect(JSON.parse(body.get("options") as string)).toEqual({ selected: ["row:2"] });
  });

  it("downloads the sample with the session cookie, accepting CSV", async () => {
    const fetch = mockFetch(200, {});
    await csvImportApi.sample();
    expect(fetch.mock.calls[0]![0]).toBe("/api/import/csv/sample");
    expect(sentHeaders(fetch).get("Accept")).toContain("text/csv");
    expect(fetch.mock.calls[0]![1]?.credentials).toBe("same-origin");
  });
});
