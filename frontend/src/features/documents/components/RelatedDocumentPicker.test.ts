import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { i18n } from "@/i18n";
import { mockFetchRoutes } from "@/test-utils";
import RelatedDocumentPicker from "./RelatedDocumentPicker.vue";

const item = (id: string, number: string) => ({ id, number, supplierNumber: null, customerName: "Acme" });

describe("RelatedDocumentPicker", () => {
  beforeEach(() => {
    i18n.global.locale.value = "en";
    setActivePinia(createPinia());
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it("lists issued targets of the counterparty, searches debounced and hints at more matches", async () => {
    vi.useFakeTimers();
    const fetch = mockFetchRoutes({ "GET /api/documents": { items: [item("p1", "Z1"), item("p2", "Z2")], total: 80 } });
    const w = mount(RelatedDocumentPicker, {
      props: { modelValue: null, direction: "received", docType: "advance_tax_doc", contactId: "c1", excludeId: "p2" },
      global: { plugins: [i18n] },
    });
    await flushPromises();
    expect(String(fetch.mock.calls[0]?.[0])).toBe("/api/documents?direction=received&docType=proforma&status=issued&contactId=c1&limit=50&offset=0");
    expect(w.findAll("option").map((o) => o.text())).toEqual(["No link", "Z1 · Acme"]);
    expect(w.find('[data-test="related-more"]').exists()).toBe(true);

    await w.find('[data-test="related-search"]').setValue("Z9");
    expect(fetch).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(300);
    expect(String(fetch.mock.calls[1]?.[0])).toContain("q=Z9");

    await w.find("select").setValue("p1");
    expect(w.emitted("update:modelValue")?.at(-1)).toEqual(["p1"]);
  });

  it("renders nothing for a proforma (no link target)", () => {
    const fetch = mockFetchRoutes({});
    const w = mount(RelatedDocumentPicker, { props: { modelValue: null, direction: "issued", docType: "proforma", contactId: null }, global: { plugins: [i18n] } });
    expect(w.find("select").exists()).toBe(false);
    expect(fetch).not.toHaveBeenCalled();
  });
});
