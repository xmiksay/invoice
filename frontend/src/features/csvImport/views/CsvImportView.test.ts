import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount, type VueWrapper } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { createMemoryHistory, createRouter } from "vue-router";
import { MAX_UPLOAD_BYTES } from "@/features/imports/upload";
import { i18n } from "@/i18n";
import { mockFetchRoutes, reply } from "@/test-utils";
import { csvEntry, csvFile } from "../testData";
import CsvImportView from "./CsvImportView.vue";

const preview = {
  entries: [
    csvEntry({ key: "row:2", row: 2, number: "20260001", categoryMatch: "existing" }),
    csvEntry({
      key: "row:3",
      row: 3,
      direction: "received",
      number: "FV-7",
      currency: "EUR",
      total: "100.00",
      contactMatch: "new",
      categoryMatch: "new",
      warnings: ["contact_created", "category_created", "related_not_found"],
      relatedNumber: "FV-1",
    }),
    csvEntry({ key: "row:4", row: 4, status: "duplicate" }),
    csvEntry({ key: "row:5", row: 5, status: "error", error: "missing_field", field: "supplier_number", direction: "received", number: null, contactMatch: null }),
    csvEntry({ key: "row:6", row: 6, status: "error", error: "total_mismatch", warnings: ["category_inactive"] }),
    csvEntry({ key: "row:7", row: 7, status: "error", error: "not_allowed", field: "tax_date", docType: "proforma" }),
    csvEntry({ key: "row:8", row: 8, status: "error", error: "not_allowed", field: "currency" }),
  ],
};

async function mountView(path = "/import/csv") {
  setActivePinia(createPinia());
  const stub = { template: "<div />" };
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: "/import/csv", name: "csv-import", component: CsvImportView },
      { path: "/invoices", name: "invoices", component: stub },
      { path: "/invoices/:id", name: "invoice-detail", component: stub },
      { path: "/received", name: "received", component: stub },
      { path: "/received/:id", name: "received-detail", component: stub },
    ],
  });
  await router.push(path);
  const w = mount({ template: "<RouterView />" }, { global: { plugins: [i18n, router] } });
  await flushPromises();
  return w;
}

async function pick(w: VueWrapper, files: File[]) {
  const input = w.find('[data-test="import-input"]');
  Object.defineProperty(input.element, "files", { value: files, configurable: true });
  await input.trigger("change");
  await flushPromises();
}

const rows = (w: VueWrapper) => w.findAll('[data-test="csv-preview-row"]');
const checkbox = (w: VueWrapper, i: number) => rows(w)[i]!.find<HTMLInputElement>('[data-test="csv-select"]');
const alert = (w: VueWrapper) => w.find('[data-test="import-error-alert"]').text();
const importCalls = (fetch: ReturnType<typeof mockFetchRoutes>) => fetch.mock.calls.filter(([url]) => String(url).startsWith("/api/import"));

describe("CsvImportView", () => {
  beforeEach(() => {
    i18n.global.locale.value = "en";
  });
  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it("previews the file: rows, matches, errors with their column and warnings", async () => {
    const fetch = mockFetchRoutes({ "POST /api/import/csv/preview": preview });
    const w = await mountView();
    const file = csvFile("doklady.xlsx", 2048);
    await pick(w, [file]);

    expect((fetch.mock.calls[0]![1]!.body as FormData).getAll("file")).toEqual([file]);
    expect(w.find('[data-test="csv-file"]').text()).toBe("File: doklady.xlsx (2 kB)");
    expect(rows(w).map((r) => r.find('[data-test="csv-row"]').text())).toEqual(["2", "3", "4", "5", "6", "7", "8"]);
    expect(rows(w).map((r) => r.find('[data-test="csv-status"]').text())).toEqual(["OK", "OK", "Duplicate", "Error", "Error", "Error", "Error"]);

    const received = rows(w)[1]!;
    expect(received.find('[data-test="csv-number"]').text()).toBe("FV-7");
    expect(received.find('[data-test="csv-type"]').text()).toBe("Received · Received invoice");
    expect(received.find('[data-test="import-contact"]').text()).toBe("New contact");
    expect(received.find('[data-test="csv-category"]').text()).toBe("New category");
    expect(received.find('[data-test="csv-total"]').text()).toContain("100.00");
    expect(received.findAll('[data-test="csv-warnings"] li').map((l) => l.text())).toEqual([
      "A contact will be created from the row.",
      "The category will be created.",
      "Original document FV-1 was not found; it will not be linked.",
    ]);
    expect(rows(w)[0]!.find('[data-test="csv-category"]').text()).toBe("Existing category");
    expect(rows(w)[2]!.find('[data-test="csv-category"]').exists()).toBe(false);

    const missing = rows(w)[3]!.find('[data-test="csv-error"]');
    expect(missing.text()).toContain("A required value is missing.");
    expect(missing.find('[data-test="csv-field"]').text()).toBe("Column: supplier_number");
    expect(rows(w)[4]!.find('[data-test="csv-error"]').text()).toContain("The total does not match the VAT recap");
    expect(rows(w)[4]!.find('[data-test="csv-field"]').exists()).toBe(false);
    expect(rows(w)[4]!.find('[data-test="csv-warnings"]').text()).toContain("imported without a category");
    expect(rows(w)[5]!.find('[data-test="csv-error"]').text()).toBe("A proforma has no tax point date.Column: tax_date");
    expect(rows(w)[6]!.find('[data-test="csv-error"]').text()).toContain("This combination is not allowed");
  });

  it("preselects the ok rows; others are disabled; select all toggles the ok rows only", async () => {
    mockFetchRoutes({ "POST /api/import/csv/preview": preview });
    const w = await mountView();
    await pick(w, [csvFile()]);
    const confirm = () => w.find<HTMLButtonElement>('[data-test="import-confirm"]');

    expect([0, 1, 2, 3, 4, 5, 6].map((i) => checkbox(w, i).element.checked)).toEqual([true, true, false, false, false, false, false]);
    expect([0, 1, 2, 3, 4, 5, 6].map((i) => checkbox(w, i).element.disabled)).toEqual([false, false, true, true, true, true, true]);
    expect(confirm().text()).toBe("Import selected (2)");

    await checkbox(w, 0).setValue(false);
    expect(confirm().text()).toBe("Import selected (1)");
    expect(w.find<HTMLInputElement>('[data-test="csv-select-all"]').element.checked).toBe(false);
    await w.find('[data-test="csv-select-all"]').setValue(true);
    expect(confirm().text()).toBe("Import selected (2)");
    await w.find('[data-test="csv-select-all"]').setValue(false);
    expect(confirm().element.disabled).toBe(true);
  });

  it("confirms with the same file + selection and links the imported documents", async () => {
    const fetch = mockFetchRoutes({
      "POST /api/import/csv/preview": preview,
      "POST /api/import/csv/confirm": {
        results: [
          { key: "row:2", status: "imported", documentId: "d1", error: null },
          { key: "row:3", status: "imported", documentId: "d2", error: null },
          { key: "row:4", status: "skipped", documentId: null, error: null },
          { key: "row:5", status: "failed", documentId: null, error: "number_taken" },
        ],
      },
    });
    const w = await mountView("/import/csv?from=received");
    const file = csvFile("a.csv");
    await pick(w, [file]);
    await checkbox(w, 1).setValue(false);
    await checkbox(w, 1).setValue(true);
    await w.find('[data-test="import-confirm"]').trigger("click");
    await flushPromises();

    const body = fetch.mock.calls.find(([url]) => url === "/api/import/csv/confirm")![1]!.body as FormData;
    expect(body.getAll("file")).toEqual([file]);
    expect(typeof body.get("options")).toBe("string");
    expect(JSON.parse(body.get("options") as string)).toEqual({ selected: ["row:2", "row:3"] });

    expect(w.find('[data-test="import-summary"]').text()).toBe("Imported 2 of 4.");
    expect(w.findAll('[data-test="import-result-row"]')[0]!.text()).toContain("Row 2");
    expect(w.findAll('[data-test="import-result-link"]').map((l) => l.attributes("href"))).toEqual(["/invoices/d1", "/received/d2"]);
    expect(w.find('[data-test="import-result-error"]').text()).toBe("The number from the received series is already taken. Check the counter in Settings.");
    expect(w.find('[data-test="csv-preview"]').exists()).toBe(false);
    expect(w.find('[data-test="import-back"]').attributes("href")).toBe("/received");

    await w.find('[data-test="import-restart"]').trigger("click");
    expect(w.find('[data-test="import-results"]').exists()).toBe(false);
    expect(w.find('[data-test="import-drop"]').exists()).toBe(true);
  });

  it("checks the pick locally: one supported file within the size limit", async () => {
    const fetch = mockFetchRoutes({});
    const w = await mountView();
    expect(w.find('[data-test="import-back"]').attributes("href")).toBe("/invoices");

    await pick(w, [csvFile("a.pdf")]);
    expect(alert(w)).toBe("Choose a single .csv, .txt or .xlsx file.");
    await pick(w, [csvFile("a.csv"), csvFile("b.csv")]);
    expect(alert(w)).toBe("Choose a single .csv, .txt or .xlsx file.");
    await pick(w, [csvFile("big.csv", MAX_UPLOAD_BYTES + 1)]);
    expect(alert(w)).toBe("The file is too large (50 MB at most).");
    expect(importCalls(fetch)).toHaveLength(0);
  });

  it("explains the file-level errors of the server", async () => {
    let answer = reply(413, { code: "too_large" });
    mockFetchRoutes({ "POST /api/import/csv/preview": () => answer });
    const w = await mountView();
    const cases: [ReturnType<typeof reply>, string][] = [
      [reply(413, { code: "too_large" }), "The file is too large (50 MB at most)."],
      [reply(422, { code: "validation", fields: { file: "invalid" } }), "The file cannot be read. Upload a CSV (UTF-8 or Windows-1250) or an XLSX workbook."],
      [reply(422, { code: "validation", fields: { file: "empty" } }), "The file contains no data rows."],
      [reply(422, { code: "validation", fields: { file: "missing_column" }, detail: "supplier_number" }), "A required column is missing: supplier_number."],
      [reply(422, { code: "validation", fields: { file: "invalid_column" }, detail: "vat_15" }), "Invalid column vat_15 (a vat_ column needs its base_ column, and the rate must be a number)."],
      [reply(422, { code: "validation", fields: { file: "too_many" } }), "Too many rows (500 at most). Split the file into smaller parts."],
      [reply(422, { code: "validation", fields: { file: "too_large" } }), "The XLSX workbook is too large once unpacked (200 MB at most). Split the file into smaller parts."],
      [reply(422, { code: "validation", fields: { file: "brand_new" } }), "The file was rejected (brand_new)."],
    ];
    for (const [next, text] of cases) {
      answer = next;
      await pick(w, [csvFile()]);
      expect(alert(w)).toBe(text);
      expect(w.find('[data-test="csv-preview"]').exists()).toBe(false);
    }
  });

  it("downloads the sample with the token under the server's filename", async () => {
    vi.stubGlobal("URL", Object.assign(URL, { createObjectURL: vi.fn(() => "blob:x"), revokeObjectURL: vi.fn() }));
    const downloads: string[] = [];
    vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (this: HTMLAnchorElement) {
      downloads.push(this.download);
    });
    let sample: () => Response = () =>
      new Response("direction;doc_type\r\n", {
        status: 200,
        headers: { "Content-Type": "text/csv", "Content-Disposition": 'attachment; filename="import-sample.csv"' },
      });
    const fetch = mockFetchRoutes({ "GET /api/import/csv/sample": () => sample() });
    const w = await mountView();

    await w.find('[data-test="csv-sample"]').trigger("click");
    await flushPromises();
    expect(fetch.mock.calls.map(([url]) => url)).toEqual(["/api/import/csv/sample"]);
    expect(downloads).toEqual(["import-sample.csv"]);

    sample = () => new Response(JSON.stringify({ code: "internal" }), { status: 500 });
    await w.find('[data-test="csv-sample"]').trigger("click");
    await flushPromises();
    expect(w.find('[data-test="csv-sample-error"]').exists()).toBe(true);
    expect(downloads).toHaveLength(1);
  });
});
