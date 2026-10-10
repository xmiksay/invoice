import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises } from "@vue/test-utils";
import { i18n } from "@/i18n";
import { useToastStore } from "@/stores/toast";
import { calls, MFA_OFF, mockFetchRoutes, reply, sentRequest } from "@/test-utils";
import { mountView } from "@/testMount";
import AccountView from "./AccountView.vue";

describe("AccountView", () => {
  beforeEach(() => {
    i18n.global.locale.value = "en";
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  // The account page always loads the TOTP status as well.
  const routes = (more: Parameters<typeof mockFetchRoutes>[0] = {}) => mockFetchRoutes({ "GET /api/account/mfa": MFA_OFF, ...more });

  async function change(current: string, next: string, code?: string) {
    const r = await mountView(AccountView, { path: "/account", role: "member", mfaEnabled: code !== undefined });
    await r.w.find('[data-test="current-password"]').setValue(current);
    await r.w.find('[data-test="new-password"]').setValue(next);
    if (code !== undefined) await r.w.find('[data-test="account-code"]').setValue(code);
    await r.w.find('[data-test="password-form"]').trigger("submit");
    await flushPromises();
    return r;
  }

  it("shows the user and changes the password", async () => {
    const fetch = routes({ "POST /api/account/password": reply(204) });
    const { w } = await change("old password!", "new password 12");
    expect(w.find('[data-test="account-email"]').text()).toBe("jana@example.cz");
    expect(w.find('[data-test="account-code"]').exists()).toBe(false);
    expect(sentRequest(fetch, 1).body).toEqual({ currentPassword: "old password!", newPassword: "new password 12" });
    expect(useToastStore().message).toBe(i18n.global.t("account.password.changed"));
    expect((w.find('[data-test="current-password"]').element as HTMLInputElement).value).toBe("");
  });

  it("checks the new password length and shows a wrong current password at its field", async () => {
    const fetch = routes();
    let { w } = await change("old", "short");
    expect(calls(fetch)).toEqual(["GET /api/account/mfa"]);
    expect(w.find("#account-new-error").text()).toBe(i18n.global.t("validation.too_short"));

    routes({ "POST /api/account/password": reply(422, { code: "validation", fields: { currentPassword: "invalid" } }) });
    ({ w } = await change("wrong", "new password 12"));
    expect(w.find("#account-current-error").text()).toBe("Wrong password.");
  });

  it("explains a rate-limited password change", async () => {
    routes({ "POST /api/account/password": reply(429, { code: "rate_limited" }) });
    const { w } = await change("wrong", "new password 12");
    expect(w.find('[data-test="password-error"]').text()).toBe("Too many attempts. Please try again later.");
  });

  it("logs out other sessions after confirmation", async () => {
    const fetch = routes({ "POST /api/auth/sessions/revoke-others": reply(204) });
    const confirm = vi.spyOn(window, "confirm").mockReturnValueOnce(false).mockReturnValueOnce(true);
    const { w } = await mountView(AccountView, { path: "/account", role: "member" });
    await w.find('[data-test="revoke-others"]').trigger("click");
    await flushPromises();
    expect(calls(fetch)).toEqual(["GET /api/account/mfa"]);
    await w.find('[data-test="revoke-others"]').trigger("click");
    await flushPromises();
    expect(confirm).toHaveBeenCalledTimes(2);
    expect(calls(fetch)).toEqual(["GET /api/account/mfa", "POST /api/auth/sessions/revoke-others"]);
    expect(useToastStore().message).toBe(i18n.global.t("account.sessions.revoked"));
  });

  describe("step-up code (user with TOTP)", () => {
    it("requires the code and sends it with the password change", async () => {
      const fetch = routes({ "POST /api/account/password": reply(204) });
      const { w } = await change("old password!", "new password 12", "");
      expect(calls(fetch)).toEqual(["GET /api/account/mfa"]);
      expect(w.find("#account-code-error").text()).toBe(i18n.global.t("validation.required"));

      await change("old password!", "new password 12", " 123456 ");
      expect(sentRequest(fetch, 2).body).toEqual({ currentPassword: "old password!", newPassword: "new password 12", code: "123456" });
    });

    it.each([
      ["invalid", "Wrong code."],
      ["required", "This field is required."],
      ["expired", "Expired."],
    ])("maps 422 code: %s", async (reason, text) => {
      routes({ "POST /api/account/password": reply(422, { code: "validation", fields: { code: reason } }) });
      const { w } = await change("old password!", "new password 12", "000000");
      expect(w.find("#account-code-error").text()).toBe(text);
    });
  });
});
