import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises } from "@vue/test-utils";
import { i18n } from "@/i18n";
import type { Member } from "@/features/members/types";
import { calls, mockFetchRoutes, reply, sentRequest } from "@/test-utils";
import { mountView } from "@/testMount";
import type { Role, Space } from "../types";
import SpaceTab from "../views/SpaceTab.vue";

vi.mock("@/lib/navigate", () => ({ leaveTo: vi.fn() }));

const space = (requireMfa = false): Space => ({ slug: "firma", name: "Firma s.r.o.", role: "owner", url: "http://firma.localhost:3000", requireMfa });
const member = (userId: string, mfaEnabled: boolean, isSelf = false): Member => ({
  userId,
  email: `${userId}@example.cz`,
  displayName: `User ${userId}`,
  role: isSelf ? "owner" : "member",
  joinedAt: "2026-10-01T10:00:00Z",
  isSelf,
  mfaEnabled,
});

describe("Settings → Space: require two-factor authentication", () => {
  beforeEach(() => {
    i18n.global.locale.value = "en";
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    document.body.innerHTML = "";
  });

  const mountTab = (role: Role = "owner", mfaEnabled = true) => mountView(SpaceTab, { path: "/settings/space", role, mfaEnabled });
  const toggle = (w: Awaited<ReturnType<typeof mountTab>>["w"]) => w.find('[data-test="require-mfa-toggle"]');

  async function flip(w: Awaited<ReturnType<typeof mountTab>>["w"]) {
    const box = toggle(w);
    (box.element as HTMLInputElement).checked = !(box.element as HTMLInputElement).checked;
    await box.trigger("change");
    await flushPromises();
  }

  it("is offered to the owner only", async () => {
    mockFetchRoutes({ "GET /api/space": { ...space(), role: "admin" } });
    const { w } = await mountTab("admin");
    expect(w.find('[data-test="require-mfa"]').exists()).toBe(false);
  });

  it.each([false, true])("is disabled in both directions with a hint while the owner has no TOTP (on: %s)", async (on) => {
    mockFetchRoutes({ "GET /api/space": space(on) });
    const { w } = await mountTab("owner", false);
    expect(toggle(w).attributes("disabled")).toBeDefined();
    expect(w.find('[data-test="require-mfa-blocked"]').text()).toContain("Turn on two-factor authentication in your own account first.");
    expect(w.find('[data-test="require-mfa-blocked"] a').attributes("href")).toBe("/account");
  });

  it("lists the members without TOTP and turns it on with the owner's code", async () => {
    const fetch = mockFetchRoutes({
      "GET /api/space": space(),
      "GET /api/members": [member("me", true, true), member("petr", false), member("eva", true), member("ota", false)],
      "PUT /api/space": space(true),
    });
    const { w } = await mountTab();
    await flip(w);
    // Not on yet: only the confirmation is open.
    expect((toggle(w).element as HTMLInputElement).checked).toBe(false);
    expect(calls(fetch)).toEqual(["GET /api/space", "GET /api/members"]);
    expect(w.findAll('[data-test="require-mfa-member"]').map((m) => m.text())).toEqual(["User petr (petr@example.cz)", "User ota (ota@example.cz)"]);

    await w.find('[data-test="require-mfa-confirm"]').trigger("submit");
    await flushPromises();
    expect(calls(fetch)).toHaveLength(2);
    expect(w.find("#require-mfa-code-error").text()).toBe(i18n.global.t("validation.required"));

    await w.find('[data-test="require-mfa-code"]').setValue(" 123456 ");
    await w.find('[data-test="require-mfa-confirm"]').trigger("submit");
    await flushPromises();
    expect(sentRequest(fetch, 2)).toMatchObject({ method: "PUT", body: { requireMfa: true, code: "123456" } });
    expect(w.find('[data-test="require-mfa-confirm"]').exists()).toBe(false);
    expect((toggle(w).element as HTMLInputElement).checked).toBe(true);
  });

  it("turns it off only after confirming with the code", async () => {
    const fetch = mockFetchRoutes({ "GET /api/space": space(true), "PUT /api/space": space(false) });
    const { w } = await mountTab();
    await flip(w);
    expect(calls(fetch)).toEqual(["GET /api/space"]);
    expect(w.find('[data-test="require-mfa-disable-intro"]').exists()).toBe(true);
    expect(w.find('[data-test="require-mfa-member"]').exists()).toBe(false);
    expect((toggle(w).element as HTMLInputElement).checked).toBe(true);
    await w.find('[data-test="require-mfa-code"]').setValue("abcde-fghij");
    await w.find('[data-test="require-mfa-confirm"]').trigger("submit");
    await flushPromises();
    expect(sentRequest(fetch, 1)).toMatchObject({ method: "PUT", body: { requireMfa: false, code: "abcde-fghij" } });
    expect((toggle(w).element as HTMLInputElement).checked).toBe(false);
  });

  it("maps a wrong code and keeps the confirmation open", async () => {
    mockFetchRoutes({ "GET /api/space": space(true), "PUT /api/space": reply(422, { code: "validation", fields: { code: "invalid" } }) });
    const { w } = await mountTab();
    await flip(w);
    await w.find('[data-test="require-mfa-code"]').setValue("000000");
    await w.find('[data-test="require-mfa-confirm"]').trigger("submit");
    await flushPromises();
    expect(w.find("#require-mfa-code-error").text()).toBe("Wrong code.");
    expect(w.find('[data-test="require-mfa-confirm"]').exists()).toBe(true);
    expect((toggle(w).element as HTMLInputElement).checked).toBe(true);
  });

  it("says when every member is ready, and can be cancelled", async () => {
    const fetch = mockFetchRoutes({ "GET /api/space": space(), "GET /api/members": [member("me", true, true)] });
    const { w } = await mountTab();
    await flip(w);
    expect(w.find('[data-test="require-mfa-all-set"]').exists()).toBe(true);
    await w.find('[data-test="require-mfa-cancel"]').trigger("click");
    expect(w.find('[data-test="require-mfa-confirm"]').exists()).toBe(false);
    expect(calls(fetch)).not.toContain("PUT /api/space");
  });

  it("explains the server's 422 mfa_not_enabled", async () => {
    mockFetchRoutes({
      "GET /api/space": space(),
      "GET /api/members": [],
      "PUT /api/space": reply(422, { code: "validation", fields: { requireMfa: "mfa_not_enabled" } }),
    });
    const { w } = await mountTab();
    await flip(w);
    await w.find('[data-test="require-mfa-code"]').setValue("123456");
    await w.find('[data-test="require-mfa-confirm"]').trigger("submit");
    await flushPromises();
    expect(w.find('[data-test="require-mfa-error"]').text()).toBe("Turn on two-factor authentication in your own account first.");
  });
});

describe("Settings → Space: delete with a step-up code", () => {
  beforeEach(() => {
    i18n.global.locale.value = "en";
  });
  afterEach(() => vi.unstubAllGlobals());

  it("has no code field without TOTP", async () => {
    mockFetchRoutes({ "GET /api/space": space() });
    const { w } = await mountView(SpaceTab, { path: "/settings/space", role: "owner" });
    expect(w.find('[data-test="delete-code"]').exists()).toBe(false);
  });

  it("requires and sends the code with TOTP, and maps a wrong one", async () => {
    const fetch = mockFetchRoutes({ "GET /api/space": space(), "DELETE /api/space": reply(422, { code: "validation", fields: { code: "invalid" } }) });
    const { w } = await mountView(SpaceTab, { path: "/settings/space", role: "owner", mfaEnabled: true });
    const button = () => w.find('[data-test="delete-submit"]');
    await w.find('[data-test="delete-slug"]').setValue("firma");
    await w.find('[data-test="delete-password"]').setValue("my password!");
    expect(button().attributes("disabled")).toBeDefined();
    await w.find('[data-test="delete-code"]').setValue("000000");
    expect(button().attributes("disabled")).toBeUndefined();
    await w.find('[data-test="delete-space"] form').trigger("submit");
    await flushPromises();
    expect(sentRequest(fetch, 1).body).toEqual({ slug: "firma", password: "my password!", code: "000000" });
    expect(w.find("#delete-code-error").text()).toBe("Wrong code.");
  });
});
