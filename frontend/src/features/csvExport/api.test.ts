import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { mockFetch, sentHeaders } from "@/test-utils";
import { csvExportApi } from "./api";

describe("csvExportApi", () => {
  beforeEach(() => setActivePinia(createPinia()));
  afterEach(() => vi.unstubAllGlobals());

  it("list sends the list filters without paging and accepts text/csv", async () => {
    const fetch = mockFetch(200, "");
    await csvExportApi.list({ direction: "received", docType: "credit_note", overdue: true, q: " acme ", from: "2026-01-01" });
    expect(String(fetch.mock.calls[0]![0])).toBe("/api/export/csv?direction=received&docType=credit_note&from=2026-01-01&overdue=true&q=acme");
    expect(sentHeaders(fetch).get("Accept")).toContain("text/csv");
  });

  it("accountant sends the period and direction", async () => {
    const fetch = mockFetch(200, "");
    await csvExportApi.accountant({ from: "2026-09-01", to: "2026-09-30", direction: "both" });
    expect(String(fetch.mock.calls[0]![0])).toBe("/api/export/accountant?from=2026-09-01&to=2026-09-30&direction=both");
  });
});
