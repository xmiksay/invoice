import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia } from "pinia";
import { i18n } from "@/i18n";
import { calls, mockFetchRoutes, reply } from "@/test-utils";
import { ACCOUNTING_DIRECTIONS, ACCOUNTING_DOC_TYPES, type AccountingSettings } from "../types";
import AccountingTab from "./AccountingTab.vue";

const settings = (): AccountingSettings => ({
  pohoda: {
    ico: null,
    codes: ACCOUNTING_DIRECTIONS.flatMap((direction) =>
      ACCOUNTING_DOC_TYPES.map((docType) => ({ direction, docType, accounting: null, classificationVat: null, numberSeries: null, classificationVatNonDeductible: null })),
    ),
  },
  money: { codes: [{ direction: "issued", docType: "invoice", opaque: 1 }] },
});

const company = { name: "Acme", ico: "27074358" };

async function mountTab() {
  const w = mount(AccountingTab, { global: { plugins: [createPinia(), i18n] } });
  await flushPromises();
  return w;
}
const input = (w: Awaited<ReturnType<typeof mountTab>>, test: string) => w.find(`[data-test="${test}"]`).element as HTMLInputElement;

describe("AccountingTab", () => {
  beforeEach(() => {
    i18n.global.locale.value = "en";
  });
  afterEach(() => vi.unstubAllGlobals());

  it("loads the codes into the direction × doc type table with the company IČO as placeholder", async () => {
    const loaded = settings();
    loaded.pohoda.ico = "11111111";
    loaded.pohoda.codes[7]!.classificationVat = "PD";
    mockFetchRoutes({ "GET /api/settings/accounting": loaded, "GET /api/settings/company": company });
    const w = await mountTab();

    expect(input(w, "pohoda-ico").value).toBe("11111111");
    expect(input(w, "pohoda-ico").placeholder).toBe("27074358");
    expect(w.findAll("thead th").map((th) => th.text())).toEqual([
      "Document",
      "Accounting code (předkontace)",
      "VAT classification (členění DPH)",
      "VAT classification, no deduction (členění DPH – bez nároku na odpočet)",
      "Number series",
    ]);
    expect(w.findAll('[data-test^="pohoda-row-"]')).toHaveLength(12);
    expect(w.find('[data-test="pohoda-row-7"] th').text()).toBe("Credit note");
    expect(input(w, "pohoda-7-classificationVat").value).toBe("PD");
    expect(input(w, "pohoda-7-classificationVat").getAttribute("aria-label")).toBe("Received documents – Credit note: VAT classification (členění DPH)");
    expect(w.find('[data-test="pohoda-0-classificationVatNonDeductible"]').exists()).toBe(false);
    expect(w.find('[data-test="pohoda-0-classificationVatNonDeductible-na"]').exists()).toBe(true);
    expect(w.find('[data-test="pohoda-6-classificationVatNonDeductible"]').exists()).toBe(true);
    expect(w.find("fieldset").attributes("disabled")).toBeUndefined();
  });

  it("saves the edited codes, null for empty, and money as loaded", async () => {
    const saved: unknown[] = [];
    const fetch = mockFetchRoutes({
      "GET /api/settings/accounting": settings(),
      "GET /api/settings/company": company,
      "PUT /api/settings/accounting": (body: unknown) => {
        saved.push(body);
        return body;
      },
    });
    const w = await mountTab();
    await w.find('[data-test="pohoda-ico"]').setValue(" 12345678 ");
    await w.find('[data-test="pohoda-0-accounting"]').setValue("3Fv");
    await w.find('[data-test="pohoda-6-numberSeries"]').setValue("FP");
    await w.find('[data-test="pohoda-6-classificationVatNonDeductible"]').setValue("PN");
    await w.find("form").trigger("submit");
    await flushPromises();

    expect(calls(fetch)).toContain("PUT /api/settings/accounting");
    const body = saved[0] as AccountingSettings;
    expect(body.pohoda.ico).toBe("12345678");
    expect(body.pohoda.codes).toHaveLength(12);
    expect(body.pohoda.codes[0]).toEqual({ direction: "issued", docType: "invoice", accounting: "3Fv", classificationVat: null, numberSeries: null, classificationVatNonDeductible: null });
    expect(body.pohoda.codes[6]).toEqual({
      direction: "received",
      docType: "invoice",
      accounting: null,
      classificationVat: null,
      numberSeries: "FP",
      classificationVatNonDeductible: "PN",
    });
    expect(body.money).toEqual(settings().money);
    expect(w.find('[role="status"]').text()).toBe("Saved.");
  });

  it("does not send money when GET omitted it (before 3c)", async () => {
    const { money: _money, ...loaded } = settings();
    const saved: Record<string, unknown>[] = [];
    mockFetchRoutes({
      "GET /api/settings/accounting": loaded,
      "GET /api/settings/company": company,
      "PUT /api/settings/accounting": (body: unknown) => {
        saved.push(body as Record<string, unknown>);
        return body;
      },
    });
    const w = await mountTab();
    await w.find("form").trigger("submit");
    await flushPromises();
    expect(Object.keys(saved[0]!)).toEqual(["pohoda"]);
  });

  it("shows the server's 422 at the cell and at the IČO", async () => {
    const fetch = mockFetchRoutes({
      "GET /api/settings/accounting": settings(),
      "GET /api/settings/company": company,
      "PUT /api/settings/accounting": reply(422, {
        code: "validation",
        fields: { "pohoda.ico": "invalid_ico", "pohoda.codes.9.numberSeries": "too_long", "pohoda.codes.2.docType": "duplicate" },
      }),
    });
    const w = await mountTab();
    await w.find("form").trigger("submit");
    await flushPromises();

    expect(calls(fetch)).toContain("PUT /api/settings/accounting");
    expect(w.find("#pohoda-ico").classes()).toContain("input-error");
    expect(w.find('[data-test="pohoda-9-numberSeries"]').classes()).toContain("input-error");
    expect(w.find('[data-test="pohoda-9-numberSeries"]').attributes("aria-invalid")).toBe("true");
    expect(w.find("#pohoda-9-numberSeries-error").text()).toBe(i18n.global.t("validation.too_long"));
    expect(w.find('[data-test="pohoda-row-2"] [data-test="field-error"]').text()).toBe(i18n.global.t("validation.duplicate"));
    expect(w.findAll('[data-test="field-error"]')).toHaveLength(3);
    expect(w.find('[data-test="accounting-error"]').exists()).toBe(true);
    expect(w.find('[role="status"]').exists()).toBe(false);
  });

  it("checks code length before sending", async () => {
    const fetch = mockFetchRoutes({ "GET /api/settings/accounting": settings(), "GET /api/settings/company": company });
    const w = await mountTab();
    await w.find('[data-test="pohoda-11-accounting"]').setValue("x".repeat(20));
    await w.find("form").trigger("submit");
    await flushPromises();
    expect(calls(fetch)).not.toContain("PUT /api/settings/accounting");
    expect(w.find('[data-test="pohoda-11-accounting"]').classes()).toContain("input-error");
  });

  it("keeps the form disabled when the load fails; a company failure only drops the placeholder", async () => {
    mockFetchRoutes({ "GET /api/settings/accounting": reply(500, { code: "internal" }), "GET /api/settings/company": reply(500, { code: "internal" }) });
    let w = await mountTab();
    expect(w.find("fieldset").attributes("disabled")).toBeDefined();
    expect(w.find('[data-test="accounting-error"]').exists()).toBe(true);

    mockFetchRoutes({ "GET /api/settings/accounting": settings(), "GET /api/settings/company": reply(500, { code: "internal" }) });
    w = await mountTab();
    expect(w.find("fieldset").attributes("disabled")).toBeUndefined();
    expect(input(w, "pohoda-ico").placeholder).toBe("");
    expect(w.find('[data-test="accounting-error"]').exists()).toBe(false);
  });
});
