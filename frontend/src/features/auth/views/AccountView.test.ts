import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises } from "@vue/test-utils";
import { i18n } from "@/i18n";
import { useToastStore } from "@/stores/toast";
import { calls, mockFetchRoutes, reply, sentRequest } from "@/test-utils";
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

  async function change(current: string, next: string) {
    const r = await mountView(AccountView, { path: "/account", role: "member" });
    await r.w.find('[data-test="current-password"]').setValue(current);
    await r.w.find('[data-test="new-password"]').setValue(next);
    await r.w.find("form").trigger("submit");
    await flushPromises();
    return r;
  }

  it("shows the user and changes the password", async () => {
    const fetch = mockFetchRoutes({ "POST /api/account/password": reply(204) });
    const { w } = await change("old password!", "new password 12");
    expect(w.find('[data-test="account-email"]').text()).toBe("jana@example.cz");
    expect(sentRequest(fetch).body).toEqual({ currentPassword: "old password!", newPassword: "new password 12" });
    expect(useToastStore().message).toBe(i18n.global.t("account.password.changed"));
    expect((w.find('[data-test="current-password"]').element as HTMLInputElement).value).toBe("");
  });

  it("checks the new password length and shows a wrong current password at its field", async () => {
    const fetch = mockFetchRoutes({});
    let { w } = await change("old", "short");
    expect(fetch).not.toHaveBeenCalled();
    expect(w.find("#account-new-error").text()).toBe(i18n.global.t("validation.too_short"));

    mockFetchRoutes({ "POST /api/account/password": reply(422, { code: "validation", fields: { currentPassword: "invalid" } }) });
    ({ w } = await change("wrong", "new password 12"));
    expect(w.find("#account-current-error").text()).toBe("Wrong password.");
  });

  it("explains a rate-limited password change", async () => {
    mockFetchRoutes({ "POST /api/account/password": reply(429, { code: "rate_limited" }) });
    const { w } = await change("wrong", "new password 12");
    expect(w.find('[data-test="password-error"]').text()).toBe("Too many attempts. Please try again later.");
  });

  it("logs out other sessions after confirmation", async () => {
    const fetch = mockFetchRoutes({ "POST /api/auth/sessions/revoke-others": reply(204) });
    const confirm = vi.spyOn(window, "confirm").mockReturnValueOnce(false).mockReturnValueOnce(true);
    const { w } = await mountView(AccountView, { path: "/account", role: "member" });
    await w.find('[data-test="revoke-others"]').trigger("click");
    await flushPromises();
    expect(fetch).not.toHaveBeenCalled();
    await w.find('[data-test="revoke-others"]').trigger("click");
    await flushPromises();
    expect(confirm).toHaveBeenCalledTimes(2);
    expect(calls(fetch)).toEqual(["POST /api/auth/sessions/revoke-others"]);
    expect(useToastStore().message).toBe(i18n.global.t("account.sessions.revoked"));
  });
});
