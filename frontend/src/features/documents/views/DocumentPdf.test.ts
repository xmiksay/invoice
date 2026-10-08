import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises } from "@vue/test-utils";
import { i18n } from "@/i18n";
import { calls, mockFetchRoutes, pdfReply, reply } from "@/test-utils";
import { company, contact, document } from "../testData";
import type { Document } from "../types";
import { mountDetail } from "./testMount";

const draftRoutes = { "GET /api/settings/company": company, "GET /api/contacts/c1": contact() };
const issued = (overrides: Partial<Document> = {}) =>
  document({ status: "issued", number: "20260001", paymentState: "unpaid", pdf: { sha256: "ab", renderedAt: "2026-10-08T10:15:00Z" }, ...overrides });

describe("detail PDF buttons", () => {
  let tab: { closed: boolean; location: { href: string }; close: () => void };
  let downloads: string[];

  beforeEach(() => {
    i18n.global.locale.value = "en";
    tab = { closed: false, location: { href: "" }, close: vi.fn() };
    vi.stubGlobal("open", vi.fn(() => tab));
    vi.stubGlobal("confirm", () => true);
    vi.stubGlobal("URL", Object.assign(URL, { createObjectURL: vi.fn(() => "blob:pdf"), revokeObjectURL: vi.fn() }));
    downloads = [];
    vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (this: HTMLAnchorElement) {
      downloads.push(this.download);
    });
  });
  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it("a draft offers a watermarked preview and opens it in the new tab", async () => {
    const fetch = mockFetchRoutes({ ...draftRoutes, "GET /api/documents/d1": document(), "GET /api/documents/d1/pdf": () => pdfReply("draft-d1.pdf") });
    const { w } = await mountDetail();

    expect(w.find('[data-test="pdf-open"]').text()).toBe("PDF preview");
    expect(w.find('[data-test="pdf-draft-hint"]').text()).toContain("DRAFT");
    expect(w.find('[data-test="pdf-archived"]').exists()).toBe(false);

    await w.find('[data-test="pdf-open"]').trigger("click");
    await flushPromises();
    expect(calls(fetch)).toContain("GET /api/documents/d1/pdf");
    expect(tab.location.href).toBe("blob:pdf");
  });

  it.each(["invoice", "proforma", "credit_note", "advance_tax_doc"] as const)("issued %s: PDF + download + archive time", async (docType) => {
    mockFetchRoutes({
      "GET /api/documents/d1": issued({ docType }),
      "GET /api/documents/d1/payments": [],
    });
    const { w } = await mountDetail();
    expect(w.find('[data-test="pdf-open"]').text()).toBe("PDF");
    expect(w.find('[data-test="pdf-download"]').text()).toBe("Download PDF");
    expect(w.find('[data-test="pdf-draft-hint"]').exists()).toBe(false);
    expect(w.find('[data-test="pdf-archived"]').text()).toMatch(/^PDF archived 8 Oct 2026/);
  });

  it("downloads with ?download=1 under the server filename, else {number}.pdf", async () => {
    let disposition: string | undefined = "20260001-server.pdf";
    const fetch = mockFetchRoutes({
      "GET /api/documents/d1": issued({ pdf: null }),
      "GET /api/documents/d1/payments": [],
      "GET /api/documents/d1/pdf": () => pdfReply(disposition, "attachment"),
    });
    const { w } = await mountDetail();
    expect(w.find('[data-test="pdf-archived"]').exists()).toBe(false);

    await w.find('[data-test="pdf-download"]').trigger("click");
    await flushPromises();
    disposition = undefined;
    await w.find('[data-test="pdf-download"]').trigger("click");
    await flushPromises();

    expect(calls(fetch).filter((c) => c.includes("/pdf"))).toEqual(["GET /api/documents/d1/pdf?download=1", "GET /api/documents/d1/pdf?download=1"]);
    expect(downloads).toEqual(["20260001-server.pdf", "20260001.pdf"]);
  });

  it.each(["pdf-open", "pdf-download"])("%s of an unarchived issued doc reloads it to show the archive time", async (button) => {
    let archived = false;
    const fetch = mockFetchRoutes({
      "GET /api/documents/d1": () => issued({ docType: "advance_tax_doc", pdf: archived ? { sha256: "ab", renderedAt: "2026-10-08T10:15:00Z" } : null }),
      "GET /api/documents/d1/pdf": () => {
        archived = true;
        return pdfReply("DP20260001.pdf");
      },
    });
    const { w } = await mountDetail();
    expect(w.find('[data-test="pdf-archived"]').exists()).toBe(false);

    await w.find(`[data-test="${button}"]`).trigger("click");
    await flushPromises();

    expect(calls(fetch).filter((c) => c === "GET /api/documents/d1")).toHaveLength(2);
    expect(w.find('[data-test="pdf-archived"]').text()).toMatch(/^PDF archived 8 Oct 2026/);
  });

  it("does not reload an already archived document", async () => {
    const fetch = mockFetchRoutes({
      "GET /api/documents/d1": issued(),
      "GET /api/documents/d1/payments": [],
      "GET /api/documents/d1/pdf": () => pdfReply(),
    });
    const { w } = await mountDetail();
    await w.find('[data-test="pdf-open"]').trigger("click");
    await flushPromises();
    expect(calls(fetch).filter((c) => c === "GET /api/documents/d1")).toHaveLength(1);
  });

  it("a failed open closes the tab and shows the render detail", async () => {
    mockFetchRoutes({
      "GET /api/documents/d1": issued(),
      "GET /api/documents/d1/payments": [],
      "GET /api/documents/d1/pdf": reply(502, { code: "pdf_render_failed", detail: "error: file not found: logo.png" }),
    });
    const { w } = await mountDetail();
    await w.find('[data-test="pdf-open"]').trigger("click");
    await flushPromises();

    expect(tab.close).toHaveBeenCalledOnce();
    const alert = w.find('[data-test="pdf-error"]');
    expect(alert.text()).toContain("Rendering the PDF failed.");
    expect(alert.find("pre").text()).toBe("error: file not found: logo.png");
  });

  it.each([
    [503, { code: "pdf_unavailable" }, "The PDF service is unavailable – the document was not issued.", false],
    [502, { code: "pdf_render_failed", detail: "unknown variable: x" }, "Rendering the PDF failed – the document was not issued.", true],
  ])("issue failing with %i explains that nothing was issued", async (status, body, message, hasDetail) => {
    mockFetchRoutes({ ...draftRoutes, "GET /api/documents/d1": document(), "POST /api/documents/d1/issue": reply(status, body) });
    const { w } = await mountDetail();
    await w.find('[data-test="issue"]').trigger("click");
    await flushPromises();

    const alert = w.find('[data-test="action-error"]');
    expect(alert.find("p").text()).toBe(message);
    expect(alert.find('[data-test="error-detail"]').exists()).toBe(hasDetail);
    if (hasDetail) expect(alert.find("pre").text()).toBe("unknown variable: x");
    expect(w.find('[data-test="issue"]').exists()).toBe(true);
  });
});
