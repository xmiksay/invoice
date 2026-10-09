import { afterEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { mockFetch, sentRequest } from "@/test-utils";
import { emailApi } from "./api";

describe("emailApi", () => {
  setActivePinia(createPinia());
  afterEach(() => vi.unstubAllGlobals());

  it("addresses the settings routes", async () => {
    const fetch = mockFetch(200, {});
    await emailApi.settings();
    await emailApi.test(null);
    await emailApi.test("x@y.cz");
    await emailApi.templates();
    await emailApi.saveTemplate("en", { subject: "S", body: "B" });
    await emailApi.restoreTemplate("cs");
    await emailApi.previewTemplate("cs", { subject: "S", body: "B" });
    expect([0, 1, 2, 3, 4, 5, 6].map((i) => sentRequest(fetch, i))).toEqual([
      { url: "/api/settings/email", method: "GET", body: undefined },
      { url: "/api/settings/email/test", method: "POST", body: {} },
      { url: "/api/settings/email/test", method: "POST", body: { to: "x@y.cz" } },
      { url: "/api/settings/email/templates", method: "GET", body: undefined },
      { url: "/api/settings/email/templates/en", method: "PUT", body: { subject: "S", body: "B" } },
      { url: "/api/settings/email/templates/cs", method: "DELETE", body: undefined },
      { url: "/api/settings/email/templates/cs/preview", method: "POST", body: { subject: "S", body: "B" } },
    ]);
  });

  it("addresses the document routes", async () => {
    const fetch = mockFetch(200, {});
    await emailApi.prefill("d 1");
    await emailApi.prefill("d1", "en");
    const input = { to: ["a@x.cz"], cc: [], bcc: [], subject: "S", body: "B", attachPdf: true, attachIsdoc: false };
    await emailApi.send("d1", input);
    await emailApi.history("d1");
    expect([0, 1, 2, 3].map((i) => sentRequest(fetch, i))).toEqual([
      { url: "/api/documents/d%201/email", method: "GET", body: undefined },
      { url: "/api/documents/d1/email?locale=en", method: "GET", body: undefined },
      { url: "/api/documents/d1/email", method: "POST", body: input },
      { url: "/api/documents/d1/emails", method: "GET", body: undefined },
    ]);
  });
});
