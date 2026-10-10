import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { createMemoryHistory, createRouter } from "vue-router";
import ReceivedListView from "@/features/received/views/ReceivedListView.vue";
import { i18n } from "@/i18n";
import { calls, mockFetchRoutes, reply } from "@/test-utils";
import AccountantExportButton from "./AccountantExportButton.vue";
import CsvExportButton from "./CsvExportButton.vue";

const csvReply = (filename?: string) =>
  new Response("﻿direction;doc_type\r\n", {
    status: 200,
    headers: { "Content-Type": "text/csv; charset=utf-8", ...(filename ? { "Content-Disposition": `attachment; filename="${filename}"` } : {}) },
  });
const tooMany = () => reply(422, { code: "validation", fields: { filter: "too_many" } });

describe("CSV export", () => {
  let downloads: string[];

  beforeEach(() => {
    setActivePinia(createPinia());
    i18n.global.locale.value = "en";
    vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(new Date(2026, 9, 9, 12));
    vi.stubGlobal("URL", Object.assign(URL, { createObjectURL: vi.fn(() => "blob:x"), revokeObjectURL: vi.fn() }));
    downloads = [];
    vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (this: HTMLAnchorElement) {
      downloads.push(this.download);
    });
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  describe("Export CSV (list)", () => {
    const mountButton = () =>
      mount(CsvExportButton, {
        props: { query: () => ({ direction: "issued" as const, docType: "invoice" as const, paymentState: "unpaid" as const, q: "" }) },
        global: { plugins: [i18n] },
      });

    it("downloads the current filter under the server filename, else doklady-{direction}-{today}.csv", async () => {
      let filename: string | undefined = "doklady-issued-2026-10-09.csv";
      const fetch = mockFetchRoutes({ "GET /api/export/csv": () => csvReply(filename) });
      const w = mountButton();
      expect(w.find('[data-test="csv-export"]').text()).toBe("Export CSV");
      await w.find('[data-test="csv-export"]').trigger("click");
      await flushPromises();
      filename = undefined;
      await w.find('[data-test="csv-export"]').trigger("click");
      await flushPromises();
      expect(calls(fetch)).toEqual([
        "GET /api/export/csv?direction=issued&docType=invoice&paymentState=unpaid",
        "GET /api/export/csv?direction=issued&docType=invoice&paymentState=unpaid",
      ]);
      expect(downloads).toEqual(["doklady-issued-2026-10-09.csv", "doklady-issued-2026-10-09.csv"]);
      expect(w.find('[data-test="csv-export-error"]').exists()).toBe(false);
    });

    it("explains too_many and other failures", async () => {
      let answer: unknown = tooMany();
      mockFetchRoutes({ "GET /api/export/csv": () => answer });
      const w = mountButton();
      await w.find('[data-test="csv-export"]').trigger("click");
      await flushPromises();
      expect(w.find('[data-test="csv-export-error"]').text()).toContain("More than 10,000 documents");
      expect(downloads).toEqual([]);

      answer = reply(500, { code: "internal" });
      await w.find('[data-test="csv-export"]').trigger("click");
      await flushPromises();
      const text = w.find('[data-test="csv-export-error"]').text();
      expect(text).not.toBe("");
      expect(text).not.toContain("10,000");
    });

    it("the received list exports its own direction, tab and filters", async () => {
      const fetch = mockFetchRoutes({
        "GET /api/documents": { items: [], total: 0 },
        "GET /api/settings/categories": [],
        "GET /api/export/csv": () => csvReply(),
      });
      const stub = { template: "<div />" };
      const router = createRouter({
        history: createMemoryHistory(),
        routes: [
          { path: "/received", name: "received", component: ReceivedListView },
          { path: "/received/new", name: "received-new", component: stub },
          { path: "/import/isdoc", name: "isdoc-import", component: stub },
          { path: "/import/csv", name: "csv-import", component: stub },
          { path: "/received/:id", name: "received-detail", component: stub },
        ],
      });
      await router.push("/received?type=credit_note");
      const pinia = createPinia();
      setActivePinia(pinia);
      const w = mount({ template: "<RouterView />" }, { global: { plugins: [pinia, i18n, router] } });
      await flushPromises();
      await w.find("#filter-from").setValue("2026-01-01");
      await w.find("#filter-from").trigger("change");
      await flushPromises();
      expect(w.find('[data-test="accountant-export"]').exists()).toBe(true);
      await w.find('[data-test="csv-export"]').trigger("click");
      await flushPromises();
      expect(calls(fetch).at(-1)).toBe("GET /api/export/csv?direction=received&docType=credit_note&from=2026-01-01");
      expect(downloads).toEqual(["doklady-received-2026-10-09.csv"]);
    });
  });

  describe("Export pro účetní (dialog)", () => {
    async function openDialog() {
      const w = mount(AccountantExportButton, { global: { plugins: [i18n] }, attachTo: document.body });
      await w.find('[data-test="accountant-export"]').trigger("click");
      return w;
    }
    const value = (w: Awaited<ReturnType<typeof openDialog>>, test: string) =>
      (w.find(`[data-test="${test}"]`).element as HTMLInputElement).value;

    it("prefills the previous calendar month and both directions, downloads and closes", async () => {
      const fetch = mockFetchRoutes({ "GET /api/export/accountant": () => csvReply("ucetni-2026-09-01-2026-09-30.csv") });
      const w = await openDialog();
      expect(value(w, "accountant-from")).toBe("2026-09-01");
      expect(value(w, "accountant-to")).toBe("2026-09-30");
      expect(value(w, "accountant-direction")).toBe("both");
      expect(value(w, "accountant-format")).toBe("csv");
      expect(w.find('[data-test="accountant-download"]').text()).toBe("Download CSV");
      await w.find('[data-test="accountant-download"]').trigger("click");
      await flushPromises();
      expect(calls(fetch)).toEqual(["GET /api/export/accountant?from=2026-09-01&to=2026-09-30&direction=both&format=csv"]);
      expect(downloads).toEqual(["ucetni-2026-09-01-2026-09-30.csv"]);
      expect(w.find('[data-test="accountant-export-dialog"]').exists()).toBe(false);
      w.unmount();
    });

    it("in January defaults to December of the previous year; sends the chosen direction; falls back to ucetni-{from}-{to}.csv", async () => {
      vi.setSystemTime(new Date(2027, 0, 5, 9));
      const fetch = mockFetchRoutes({ "GET /api/export/accountant": () => csvReply() });
      const w = await openDialog();
      expect(value(w, "accountant-from")).toBe("2026-12-01");
      expect(value(w, "accountant-to")).toBe("2026-12-31");
      await w.find('[data-test="accountant-direction"]').setValue("received");
      await w.find('[data-test="accountant-download"]').trigger("click");
      await flushPromises();
      expect(calls(fetch)).toEqual(["GET /api/export/accountant?from=2026-12-01&to=2026-12-31&direction=received&format=csv"]);
      expect(downloads).toEqual(["ucetni-2026-12-01-2026-12-31.csv"]);
      w.unmount();
    });

    it("exports Pohoda XML under the server filename, else pohoda-{from}-{to}.xml", async () => {
      let filename: string | undefined = "pohoda-2026-09-01-2026-09-30.xml";
      const xmlReply = () =>
        new Response("<dat:dataPack/>", {
          status: 200,
          headers: { "Content-Type": "application/xml; charset=windows-1250", ...(filename ? { "Content-Disposition": `attachment; filename="${filename}"` } : {}) },
        });
      const fetch = mockFetchRoutes({ "GET /api/export/accountant": xmlReply });
      let w = await openDialog();
      expect(w.findAll('[data-test="accountant-format"] option').map((o) => o.text())).toEqual(["CSV", "Pohoda XML", "Money S3 XML"]);
      await w.find('[data-test="accountant-format"]').setValue("pohoda");
      expect(w.find('[data-test="accountant-download"]').text()).toBe("Download Pohoda XML");
      expect(w.text()).toContain("Settings → Accounting");
      await w.find('[data-test="accountant-download"]').trigger("click");
      await flushPromises();
      w.unmount();

      filename = undefined;
      w = await openDialog();
      await w.find('[data-test="accountant-direction"]').setValue("issued");
      await w.find('[data-test="accountant-format"]').setValue("pohoda");
      await w.find('[data-test="accountant-download"]').trigger("click");
      await flushPromises();
      expect(calls(fetch)).toEqual([
        "GET /api/export/accountant?from=2026-09-01&to=2026-09-30&direction=both&format=pohoda",
        "GET /api/export/accountant?from=2026-09-01&to=2026-09-30&direction=issued&format=pohoda",
      ]);
      expect(downloads).toEqual(["pohoda-2026-09-01-2026-09-30.xml", "pohoda-2026-09-01-2026-09-30.xml"]);
      w.unmount();
    });

    it("explains an unexportable document with the server's detail, and an empty period", async () => {
      let answer: unknown = reply(422, { code: "validation", fields: { documents: "unexportable" }, detail: "FV2026-0007: VAT rate 15 % has no Pohoda slot" });
      mockFetchRoutes({ "GET /api/export/accountant": () => answer });
      const w = await openDialog();
      await w.find('[data-test="accountant-format"]').setValue("pohoda");
      await w.find('[data-test="accountant-download"]').trigger("click");
      await flushPromises();
      const alert = w.find('[data-test="accountant-export-error"]');
      expect(alert.text()).toContain("A document cannot be exported");
      expect(alert.find("pre").text()).toBe("FV2026-0007: VAT rate 15 % has no Pohoda slot");
      expect(w.findAll('[data-test="field-error"]')).toHaveLength(0);

      answer = reply(422, { code: "validation", fields: { from: "empty" } });
      await w.find('[data-test="accountant-download"]').trigger("click");
      await flushPromises();
      expect(w.find('[data-test="accountant-export-error"]').text()).toBe("There are no documents in the period.");
      expect(w.find('[data-test="error-detail"]').exists()).toBe(false);
      expect(w.findAll('[data-test="field-error"]')).toHaveLength(0);
      expect(downloads).toEqual([]);
      w.unmount();
    });

    it("exports Money S3 XML under the server filename, else money-{from}-{to}.xml", async () => {
      let filename: string | undefined = "money-2026-09-01-2026-09-30.xml";
      const xmlReply = () =>
        new Response("<MoneyData/>", {
          status: 200,
          headers: { "Content-Type": "application/xml; charset=utf-8", ...(filename ? { "Content-Disposition": `attachment; filename="${filename}"` } : {}) },
        });
      const fetch = mockFetchRoutes({ "GET /api/export/accountant": xmlReply });
      let w = await openDialog();
      await w.find('[data-test="accountant-format"]').setValue("money");
      expect(w.find('[data-test="accountant-download"]').text()).toBe("Download Money S3 XML");
      expect(w.text()).toContain("Money S3 XML uses the accounting codes");
      await w.find('[data-test="accountant-download"]').trigger("click");
      await flushPromises();
      w.unmount();

      filename = undefined;
      w = await openDialog();
      await w.find('[data-test="accountant-direction"]').setValue("received");
      await w.find('[data-test="accountant-format"]').setValue("money");
      await w.find('[data-test="accountant-download"]').trigger("click");
      await flushPromises();
      expect(calls(fetch)).toEqual([
        "GET /api/export/accountant?from=2026-09-01&to=2026-09-30&direction=both&format=money",
        "GET /api/export/accountant?from=2026-09-01&to=2026-09-30&direction=received&format=money",
      ]);
      expect(downloads).toEqual(["money-2026-09-01-2026-09-30.xml", "money-2026-09-01-2026-09-30.xml"]);
      w.unmount();
    });

    it("shows the server's format rejection at the format field", async () => {
      mockFetchRoutes({ "GET /api/export/accountant": reply(422, { code: "validation", fields: { format: "invalid" } }) });
      const w = await openDialog();
      await w.find('[data-test="accountant-download"]').trigger("click");
      await flushPromises();
      expect(w.findAll('[data-test="field-error"]').map((e) => e.text())).toEqual(["Invalid value."]);
      expect(w.find("#accountant-format").classes()).toContain("input-error");
      expect(w.find('[data-test="accountant-export-dialog"]').exists()).toBe(true);
      w.unmount();
    });

    it("checks the period before sending", async () => {
      const fetch = mockFetchRoutes({});
      const w = await openDialog();
      await w.find('[data-test="accountant-from"]').setValue("");
      await w.find('[data-test="accountant-to"]').setValue("2026-08-31");
      await w.find('[data-test="accountant-download"]').trigger("click");
      await flushPromises();
      expect(fetch).not.toHaveBeenCalled();
      expect(w.findAll('[data-test="field-error"]').map((e) => e.text())).toEqual(["This field is required."]);

      await w.find('[data-test="accountant-from"]').setValue("2026-09-01");
      await w.find('[data-test="accountant-download"]').trigger("click");
      await flushPromises();
      expect(fetch).not.toHaveBeenCalled();
      expect(w.findAll('[data-test="field-error"]').map((e) => e.text())).toEqual(["Invalid value."]);
      expect(w.find("#accountant-to").classes()).toContain("input-error");
      w.unmount();
    });

    it("shows the server's from/to/direction rejection at the fields and explains too_many; the dialog stays open", async () => {
      let answer: unknown = reply(422, { code: "validation", fields: { from: "invalid", to: "invalid" } });
      mockFetchRoutes({ "GET /api/export/accountant": () => answer });
      const w = await openDialog();
      await w.find('[data-test="accountant-from"]').setValue("2025-01-01");
      await w.find('[data-test="accountant-download"]').trigger("click");
      await flushPromises();
      expect(w.findAll('[data-test="field-error"]')).toHaveLength(2);
      expect(w.find("#accountant-from").classes()).toContain("input-error");
      expect(w.find('[data-test="accountant-export-error"]').exists()).toBe(true);

      answer = reply(422, { code: "validation", fields: { direction: "invalid" } });
      await w.find('[data-test="accountant-download"]').trigger("click");
      await flushPromises();
      expect(w.findAll('[data-test="field-error"]').map((e) => e.text())).toEqual(["Invalid value."]);
      expect(w.find("#accountant-direction").classes()).toContain("input-error");

      answer = tooMany();
      await w.find('[data-test="accountant-download"]').trigger("click");
      await flushPromises();
      expect(w.findAll('[data-test="field-error"]')).toHaveLength(0);
      expect(w.find('[data-test="accountant-export-error"]').text()).toContain("More than 10,000 documents");
      expect(downloads).toEqual([]);
      expect(w.find('[data-test="accountant-export-dialog"]').exists()).toBe(true);

      await w.find('[data-test="accountant-cancel"]').trigger("click");
      expect(w.find('[data-test="accountant-export-dialog"]').exists()).toBe(false);
      w.unmount();
    });
  });
});
