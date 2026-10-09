import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises } from "@vue/test-utils";
import { i18n } from "@/i18n";
import { calls, mockFetchRoutes, reply } from "@/test-utils";
import { document, totals } from "../testData";
import type { Document } from "../types";
import { mountDetail } from "./testMount";

const ACTIONS = ["credit-note", "debit-note", "ddpp-correction"] as const;
const shown = (w: Awaited<ReturnType<typeof mountDetail>>["w"]) => ACTIONS.filter((a) => w.find(`[data-test="${a}"]`).exists());

async function mountDoc(doc: Document, routes: Record<string, unknown> = {}) {
  const fetch = mockFetchRoutes({
    [`GET /api/documents/${doc.id}`]: doc,
    [`GET /api/documents/${doc.id}/payments`]: [],
    "GET /api/settings/company": {},
    "GET /api/contacts/c1": {},
    "GET /api/settings/categories": [],
    "GET /api/settings/custom-fields": [],
    ...routes,
  });
  return { fetch, ...(await mountDetail(doc.id)) };
}

describe("correction actions (1f-a)", () => {
  beforeEach(() => {
    i18n.global.locale.value = "en";
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it.each([
    ["issued invoice", { status: "issued" }, ["credit-note", "debit-note"]],
    ["issued imported simplified", { docType: "simplified", status: "issued", imported: true }, ["credit-note", "debit-note"]],
    ["issued DDPP", { docType: "advance_tax_doc", status: "issued" }, ["ddpp-correction"]],
    ["cancelled invoice", { status: "cancelled" }, []],
    ["draft simplified", { docType: "simplified", status: "draft" }, []],
    ["issued debit note", { docType: "debit_note", status: "issued" }, []],
    ["issued credit note", { docType: "credit_note", status: "issued", sign: -1 }, []],
    ["issued DDPP correction", { docType: "advance_credit_note", status: "issued", sign: -1 }, []],
    ["issued proforma", { docType: "proforma", status: "issued", taxPointDate: null }, []],
  ] as const)("%s: correction actions", async (_, overrides, expected) => {
    const { w } = await mountDoc(document({ number: "X1", ...(overrides as Partial<Document>) }));
    expect(shown(w)).toEqual(expected);
  });

  it("debit note: prompted reason posted to /debit-note, the draft opens in the editor", async () => {
    vi.stubGlobal("prompt", () => " extra work ");
    const { w, fetch, router } = await mountDoc(document({ status: "issued", number: "20260001" }), {
      "POST /api/documents/d1/debit-note": reply(201, document({ id: "vn1", docType: "debit_note", lines: [] })),
    });
    await w.find('[data-test="debit-note"]').trigger("click");
    await flushPromises();
    const post = fetch.mock.calls.find(([url, init]) => init?.method === "POST" && String(url).endsWith("/debit-note"));
    expect(JSON.parse(String(post?.[1]?.body))).toEqual({ correctionReason: "extra work" });
    expect(router.currentRoute.value).toMatchObject({ name: "invoice-edit", params: { id: "vn1" } });
  });

  it.each([
    ["advance_settled", "already deducted by an issued final invoice"],
    ["advance_in_use", "deducted by a draft final invoice"],
    ["invalid_state", "This document cannot be corrected."],
  ])("DDPP correction refused with 409 %s explains why", async (code, text) => {
    vi.stubGlobal("prompt", () => "refund");
    const { w, fetch } = await mountDoc(document({ id: "dd1", docType: "advance_tax_doc", status: "issued", number: "DP1" }), {
      "POST /api/documents/dd1/credit-note": reply(409, { code }),
    });
    await w.find('[data-test="ddpp-correction"]').trigger("click");
    await flushPromises();
    expect(calls(fetch)).toContain("POST /api/documents/dd1/credit-note");
    expect(w.find('[data-test="action-error"]').text()).toContain(text);
  });

  it("issuing a DDPP correction draft refused with 409 advance_in_use", async () => {
    vi.stubGlobal("confirm", () => true);
    const { w } = await mountDoc(document({ id: "op1", docType: "advance_credit_note", sign: -1 }), {
      "POST /api/documents/op1/issue": reply(409, { code: "advance_in_use" }),
    });
    await w.find('[data-test="issue"]').trigger("click");
    await flushPromises();
    expect(w.find('[data-test="action-error"]').text()).toContain("deducted by a draft final invoice");
  });

  it.each([
    ["advance_in_use", ["draft final invoice", "correction"]],
    ["advance_settled", ["cannot be cancelled", "issued final invoice"]],
  ])("cancelling an imported DDPP refused with 409 %s names the causes", async (code, texts) => {
    vi.stubGlobal("prompt", () => "");
    const { w } = await mountDoc(document({ id: "dd1", docType: "advance_tax_doc", status: "issued", number: "DP1", imported: true }), {
      "POST /api/documents/dd1/cancel": reply(409, { code }),
    });
    await w.find('[data-test="cancel"]').trigger("click");
    await flushPromises();
    const text = w.find('[data-test="action-error"]').text();
    for (const t of texts) expect(text).toContain(t);
  });

  it.each([
    ["issue", { status: "draft" }, "advance_settled", "already deducted by an issued final invoice"],
    ["cancel", { status: "issued", number: "OP1" }, "advance_settled", "The correction cannot be cancelled: its advance payment tax document is already deducted"],
    ["cancel", { status: "issued", number: "OP1" }, "advance_in_use", "The correction cannot be cancelled: its advance payment tax document is deducted by a draft"],
  ] as const)("imported DDPP correction: %s refused with 409 %s", async (action, overrides, code, text) => {
    vi.stubGlobal("confirm", () => true);
    vi.stubGlobal("prompt", () => "");
    const doc = document({ id: "op1", docType: "advance_credit_note", sign: -1, imported: true, number: "EXT-OP1", ...overrides });
    const { w } = await mountDoc(doc, { [`POST /api/documents/op1/${action}`]: reply(409, { code }) });
    await w.find(`[data-test="${action}"]`).trigger("click");
    await flushPromises();
    expect(w.find('[data-test="action-error"]').text()).toContain(text);
  });

  it.each([
    ["advance_settled", "already deducted by an issued final invoice"],
    ["advance_in_use", "deducted by a draft final invoice"],
    ["fully_corrected", "already fully corrected"],
  ] as const)("DDPP with correctionBlock %s: correction disabled with the reason", async (block, text) => {
    const { w } = await mountDoc(document({ id: "dd1", docType: "advance_tax_doc", status: "issued", number: "DP1", correctionBlock: block }));
    const button = w.find('[data-test="ddpp-correction"]');
    expect(button.attributes("disabled")).toBeDefined();
    expect(button.attributes("title")).toContain(text);
    expect(w.find('[data-test="ddpp-correction-block"]').text()).toContain(text);
  });

  it("DDPP without a correctionBlock: correction enabled, no reason shown", async () => {
    const { w } = await mountDoc(document({ id: "dd1", docType: "advance_tax_doc", status: "issued", number: "DP1" }));
    expect(w.find('[data-test="ddpp-correction"]').attributes("disabled")).toBeUndefined();
    expect(w.find('[data-test="ddpp-correction-block"]').exists()).toBe(false);
  });

  it.each([
    ["debit_note", { docType: "debit_note" }],
    ["simplified", { docType: "simplified" }],
    ["invoice", {}],
  ] as const)("the cancel button is generic on a %s", async (_, overrides) => {
    const { w } = await mountDoc(document({ status: "issued", number: "X1", ...overrides }));
    expect(w.find('[data-test="cancel"]').text()).toBe("Cancel document");
  });

  it("cancelling a debit note refused with 409 exceeds_original", async () => {
    vi.stubGlobal("prompt", () => "");
    const { w } = await mountDoc(document({ id: "vn1", docType: "debit_note", status: "issued", number: "V1" }), {
      "POST /api/documents/vn1/cancel": reply(409, { code: "exceeds_original" }),
    });
    await w.find('[data-test="cancel"]').trigger("click");
    await flushPromises();
    expect(w.find('[data-test="action-error"]').text()).toContain("credit notes would exceed the original document plus its debit notes");
  });

  it("DDPP correction: negated totals with its own badge label", async () => {
    const { w } = await mountDoc(document({ id: "op1", docType: "advance_credit_note", status: "issued", number: "OP1", sign: -1 }));
    expect(w.find("h1").text()).toContain("Advance payment correction");
    expect(w.find('[data-test="credit-note-badge"]').text()).toBe("Advance payment correction");
    expect(w.find('[data-test="payable"]').text()).toMatch(/^-/);
  });

  it("simplified document above 10 000 CZK shows the warning", async () => {
    const big = { ...totals("12100.00") };
    const { w } = await mountDoc(document({ docType: "simplified", status: "issued", number: "ZD1", contactId: null, totals: big }));
    expect(w.find('[data-test="simplified-limit"]').text()).toContain("10,000");
  });

  it("simplified document within the limit shows no warning", async () => {
    const { w } = await mountDoc(document({ docType: "simplified", status: "issued", number: "ZD1" }));
    expect(w.find('[data-test="simplified-limit"]').exists()).toBe(false);
  });
});
