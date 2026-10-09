import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia } from "pinia";
import { i18n } from "@/i18n";
import { mockFetchRoutes, reply } from "@/test-utils";
import { logEntry } from "../testData";
import EmailHistory from "./EmailHistory.vue";

async function mountHistory() {
  const w = mount(EmailHistory, { props: { documentId: "d1" }, global: { plugins: [createPinia(), i18n] } });
  await flushPromises();
  return w;
}

describe("EmailHistory", () => {
  beforeEach(() => {
    i18n.global.locale.value = "en";
  });
  afterEach(() => vi.unstubAllGlobals());

  it("lists the attempts with recipients, subject and result", async () => {
    mockFetchRoutes({
      "GET /api/documents/d1/emails": [
        logEntry({ id: "e2", ok: false, error: "550 mailbox unavailable", messageId: null, to: ["a@x.cz", "b@x.cz"], subject: "Second" }),
        logEntry(),
      ],
    });
    const w = await mountHistory();
    const rows = w.findAll('[data-test="email-entry"]').map((r) => r.findAll("td").slice(1).map((td) => td.text()));
    expect(rows).toEqual([
      ["a@x.cz, b@x.cz", "Second", "failed"],
      ["buyer@acme.cz", "Faktura 20260001 – Firma", "delivered to server"],
    ]);
    expect(w.find('[data-test="email-detail"]').exists()).toBe(false);
  });

  it("expands an entry to its body, attachments and error", async () => {
    mockFetchRoutes({ "GET /api/documents/d1/emails": [logEntry({ ok: false, error: "550 mailbox unavailable", attachments: [] })] });
    const w = await mountHistory();
    const toggle = w.find('[data-test="email-toggle"]');
    await toggle.trigger("click");

    expect(toggle.attributes("aria-expanded")).toBe("true");
    expect(w.find('[data-test="email-entry-body"]').text()).toBe("Dobrý den,\nv příloze…");
    expect(w.find('[data-test="email-attachments"]').text()).toBe("none");
    expect(w.find('[data-test="email-entry-error"]').text()).toBe("550 mailbox unavailable");
    expect(w.find('[data-test="email-detail"]').text()).toContain("me@firma.cz");

    await toggle.trigger("click");
    expect(w.find('[data-test="email-detail"]').exists()).toBe(false);
  });

  it("says when nothing was sent", async () => {
    mockFetchRoutes({ "GET /api/documents/d1/emails": [] });
    const w = await mountHistory();
    expect(w.find('[data-test="email-history-empty"]').text()).toBe("No e-mail has been sent yet.");
  });

  it("keeps a load failure inside the section", async () => {
    mockFetchRoutes({ "GET /api/documents/d1/emails": reply(500, { code: "internal" }) });
    const w = await mountHistory();
    expect(w.find('[role="alert"]').text()).toBe("Unexpected error (internal).");
  });
});
