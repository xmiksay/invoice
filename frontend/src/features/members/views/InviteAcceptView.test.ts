import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises } from "@vue/test-utils";
import { i18n } from "@/i18n";
import { calls, meFixture, mockFetchRoutes, reply, sentRequest, SPACE_CONTEXT } from "@/test-utils";
import { mountView } from "@/testMount";
import type { InviteInfo } from "../types";
import InviteAcceptView from "./InviteAcceptView.vue";

const info = (accountExists: boolean): InviteInfo => ({
  space: { slug: "firma", name: "Firma s.r.o." },
  email: "petr@example.cz",
  role: "accountant",
  accountExists,
});

describe("InviteAcceptView", () => {
  beforeEach(() => {
    i18n.global.locale.value = "en";
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    document.body.innerHTML = "";
  });

  const mountAccept = (query = "?token=T1") => mountView(InviteAcceptView, { path: `/invite${query}`, context: SPACE_CONTEXT });

  async function submit(w: Awaited<ReturnType<typeof mountAccept>>["w"]) {
    await w.find("form").trigger("submit");
    await flushPromises();
  }

  it("accepts with an existing account's password, signs in and goes home", async () => {
    const fetch = mockFetchRoutes({
      "GET /api/invites/accept": info(true),
      "POST /api/invites/accept": reply(204),
      "GET /api/auth/me": meFixture("accountant"),
    });
    const { w, router, session } = await mountAccept();
    expect(sentRequest(fetch).url).toBe("/api/invites/accept?token=T1");
    expect(w.find('[data-test="accept-intro"]').text()).toBe("You have been invited to the space Firma s.r.o. as Accountant.");
    const email = w.find('[data-test="accept-email"]');
    expect((email.element as HTMLInputElement).value).toBe("petr@example.cz");
    expect(email.attributes("readonly")).toBeDefined();
    expect(w.find('[data-test="accept-name"]').exists()).toBe(false);
    expect(w.find('[data-test="accept-submit"]').text()).toBe("Accept invitation");

    await w.find('[data-test="accept-password"]').setValue("short");
    await submit(w);
    expect(sentRequest(fetch, 1).body).toEqual({ token: "T1", password: "short" });
    expect(calls(fetch).slice(1)).toEqual(["POST /api/invites/accept", "GET /api/auth/me"]);
    expect(session.role).toBe("accountant");
    expect(router.currentRoute.value.name).toBe("home");
  });

  it("explains a wrong password and a rate limit", async () => {
    mockFetchRoutes({ "GET /api/invites/accept": info(true), "POST /api/invites/accept": reply(401, { code: "invalid_credentials" }) });
    const { w, router } = await mountAccept();
    await w.find('[data-test="accept-password"]').setValue("wrong password");
    await submit(w);
    expect(w.find('[data-test="accept-error"]').text()).toBe(i18n.global.t("errors.invalidCredentials"));
    mockFetchRoutes({ "POST /api/invites/accept": reply(429, { code: "rate_limited" }) });
    await submit(w);
    expect(w.find('[data-test="accept-error"]').text()).toBe("Too many attempts. Please try again later.");
    expect(router.currentRoute.value.name).toBe("view");
  });

  it("creates a new account with a name and a 12+ character password", async () => {
    const fetch = mockFetchRoutes({
      "GET /api/invites/accept": info(false),
      "POST /api/invites/accept": reply(204),
      "GET /api/auth/me": meFixture("accountant"),
    });
    const { w, router } = await mountAccept();
    expect(w.find('[data-test="accept-submit"]').text()).toBe("Create account and accept");
    await w.find('[data-test="accept-password"]').setValue("too short");
    await submit(w);
    expect(calls(fetch)).toHaveLength(1);
    expect(w.find("#accept-name-error").text()).toBe(i18n.global.t("validation.required"));
    expect(w.find("#accept-password-error").text()).toBe(i18n.global.t("validation.too_short"));

    await w.find('[data-test="accept-name"]').setValue(" Petr ");
    await w.find('[data-test="accept-password"]').setValue("long enough pw");
    await submit(w);
    expect(sentRequest(fetch, 1).body).toEqual({ token: "T1", password: "long enough pw", displayName: "Petr" });
    expect(router.currentRoute.value.name).toBe("home");
  });

  it.each([
    ["an unknown link", "?token=T1", { "GET /api/invites/accept": reply(404, { code: "not_found" }) }],
    ["a missing token", "", {}],
  ])("shows %s as invalid with a link to the base host", async (_, query, routes) => {
    mockFetchRoutes(routes);
    const { w } = await mountAccept(query);
    expect(w.find('[data-test="accept-invalid"]').text()).toContain("invalid");
    expect(w.find('[data-test="accept-base-link"]').attributes("href")).toBe("http://localhost:3000");
    expect(w.find("form").exists()).toBe(false);
  });

  it("turns a token used up meanwhile (422 token) into the invalid message", async () => {
    mockFetchRoutes({
      "GET /api/invites/accept": info(true),
      "POST /api/invites/accept": reply(422, { code: "validation", fields: { token: "invalid" } }),
    });
    const { w } = await mountAccept();
    await w.find('[data-test="accept-password"]').setValue("my password!");
    await submit(w);
    expect(w.find('[data-test="accept-invalid"]').exists()).toBe(true);
  });
});
