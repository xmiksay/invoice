import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { mountDetail } from "@/features/documents/views/testMount";
import { company, contact, document } from "@/features/documents/testData";
import { i18n } from "@/i18n";
import { calls, mockFetchRoutes, reply } from "@/test-utils";
import IsdocExportButton from "./IsdocExportButton.vue";

const zipReply = (filename?: string) =>
  new Response("PK", {
    status: 200,
    headers: { "Content-Type": "application/zip", ...(filename ? { "Content-Disposition": `attachment; filename="${filename}"` } : {}) },
  });

describe("ISDOC export", () => {
  let downloads: string[];

  beforeEach(() => {
    i18n.global.locale.value = "en";
    vi.stubGlobal("URL", Object.assign(URL, { createObjectURL: vi.fn(() => "blob:x"), revokeObjectURL: vi.fn() }));
    downloads = [];
    vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (this: HTMLAnchorElement) {
      downloads.push(this.download);
    });
  });
  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it("an issued document downloads its ISDOC under the server filename, else {number}.isdocx", async () => {
    let filename: string | undefined = "20260001.isdoc";
    const fetch = mockFetchRoutes({
      "GET /api/documents/d1": document({ status: "cancelled", number: "20260001" }),
      "GET /api/documents/d1/payments": [],
      "GET /api/documents/d1/isdoc": () => zipReply(filename),
    });
    const { w } = await mountDetail();
    const button = w.find('[data-test="isdoc-download"]');
    expect(button.text()).toBe("Download ISDOC");
    await button.trigger("click");
    await flushPromises();
    filename = undefined;
    await button.trigger("click");
    await flushPromises();
    expect(calls(fetch).filter((c) => c.endsWith("/isdoc"))).toHaveLength(2);
    expect(downloads).toEqual(["20260001.isdoc", "20260001.isdocx"]);
  });

  it("a draft has no ISDOC download", async () => {
    mockFetchRoutes({ "GET /api/documents/d1": document(), "GET /api/settings/company": company, "GET /api/contacts/c1": contact() });
    const { w } = await mountDetail();
    expect(w.find('[data-test="pdf-open"]').exists()).toBe(true);
    expect(w.find('[data-test="isdoc-download"]').exists()).toBe(false);
  });

  it("the list export sends the current filter and explains the 1000-document cap", async () => {
    setActivePinia(createPinia());
    let answer: unknown = zipReply();
    const fetch = mockFetchRoutes({ "GET /api/documents/isdoc": () => answer });
    const w = mount(IsdocExportButton, {
      props: { query: () => ({ direction: "issued" as const, docType: "credit_note" as const, from: "2026-01-01" }) },
      global: { plugins: [i18n] },
    });
    await w.find('[data-test="isdoc-export"]').trigger("click");
    await flushPromises();
    expect(String(fetch.mock.calls[0]![0])).toBe("/api/documents/isdoc?direction=issued&docType=credit_note&from=2026-01-01");
    expect(downloads).toEqual(["isdoc-export.zip"]);

    answer = reply(422, { code: "validation", fields: { filter: "too_many" } });
    await w.find('[data-test="isdoc-export"]').trigger("click");
    await flushPromises();
    expect(w.find('[data-test="isdoc-export-error"]').text()).toContain("More than 1000 documents");
  });
});
