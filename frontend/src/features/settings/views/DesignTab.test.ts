import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia } from "pinia";
import { i18n } from "@/i18n";
import { calls, mockFetchRoutes, pdfReply, reply } from "@/test-utils";
import DesignTab from "./DesignTab.vue";

const design = {
  designDir: "/srv/design",
  files: [
    { path: "fonts/Inter-Regular.ttf", source: "default", size: 310_000 },
    { path: "invoice.typ", source: "custom", size: 5_120 },
    { path: "logo.svg", source: "custom", size: 812 },
  ],
};

async function mountTab() {
  const w = mount(DesignTab, { global: { plugins: [createPinia(), i18n] } });
  await flushPromises();
  return w;
}

describe("DesignTab", () => {
  let tab: { closed: boolean; location: { href: string }; close: () => void };

  beforeEach(() => {
    i18n.global.locale.value = "en";
    tab = { closed: false, location: { href: "" }, close: vi.fn() };
    vi.stubGlobal("open", vi.fn(() => tab));
    vi.stubGlobal("URL", Object.assign(URL, { createObjectURL: vi.fn(() => "blob:preview"), revokeObjectURL: vi.fn() }));
  });
  afterEach(() => vi.unstubAllGlobals());

  it("lists the effective files with source and size", async () => {
    mockFetchRoutes({ "GET /api/pdf/design": design });
    const w = await mountTab();

    expect(w.find('[data-test="design-dir"]').text()).toBe("/srv/design");
    const rows = w.findAll('[data-test="design-file"]').map((r) => r.findAll("td").map((td) => td.text()));
    expect(rows).toEqual([
      ["fonts/Inter-Regular.ttf", "default", "302.7 kB"],
      ["invoice.typ", "custom", "5 kB"],
      ["logo.svg", "custom", "812 B"],
    ]);
    expect(w.findAll('[data-test="source-custom"]')).toHaveLength(2);
    expect(w.find('[data-test="design-help"]').text()).toContain("INVOICE__DESIGN_DIR");
  });

  it("shows the built-in design when no dir is set", async () => {
    mockFetchRoutes({ "GET /api/pdf/design": { designDir: null, files: [] } });
    const w = await mountTab();
    expect(w.find('[data-test="design-dir"]').text()).toBe("built-in design");
  });

  it("opens the preview per locale", async () => {
    const fetch = mockFetchRoutes({ "GET /api/pdf/design": design, "GET /api/pdf/preview": () => pdfReply() });
    const w = await mountTab();
    await w.find('[data-test="design-preview-en"]').trigger("click");
    await flushPromises();
    await w.find('[data-test="design-preview-cs"]').trigger("click");
    await flushPromises();

    expect(calls(fetch).slice(1)).toEqual(["GET /api/pdf/preview?locale=en", "GET /api/pdf/preview?locale=cs"]);
    expect(tab.location.href).toBe("blob:preview");
  });

  it("shows a render failure with its detail expanded", async () => {
    mockFetchRoutes({
      "GET /api/pdf/design": design,
      "GET /api/pdf/preview": reply(502, { code: "pdf_render_failed", detail: "error: expected expression\n  ┌─ invoice.typ:12:3" }),
    });
    const w = await mountTab();
    await w.find('[data-test="design-preview-cs"]').trigger("click");
    await flushPromises();

    const alert = w.find('[data-test="design-preview-error"]');
    expect(alert.text()).toContain("Rendering the PDF failed.");
    expect(alert.find("details").attributes("open")).toBeDefined();
    expect(alert.find("pre").text()).toBe("error: expected expression\n  ┌─ invoice.typ:12:3");
    expect(tab.close).toHaveBeenCalledOnce();
  });
});
