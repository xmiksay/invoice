import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises } from "@vue/test-utils";
import { i18n } from "@/i18n";
import { calls, meFixture, mockFetchRoutes, reply, sentRequest, SPACE_CONTEXT } from "@/test-utils";
import { mountView } from "@/testMount";
import LoginView from "./LoginView.vue";

type Wrapper = Awaited<ReturnType<typeof mountView>>["w"];

describe("LoginView with two-factor authentication", () => {
  beforeEach(() => {
    i18n.global.locale.value = "en";
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    document.body.innerHTML = "";
  });

  async function password(w: Wrapper) {
    await w.find('[data-test="login-email"]').setValue("jana@example.cz");
    await w.find('[data-test="login-password"]').setValue("correct horse battery");
    await w.find("form").trigger("submit");
    await flushPromises();
  }

  async function code(w: Wrapper, value: string) {
    await w.find('[data-test="login-code"]').setValue(value);
    await w.find('[data-test="login-code-step"]').trigger("submit");
    await flushPromises();
  }

  it("asks for the app code after the password, then signs in and follows ?return", async () => {
    const fetch = mockFetchRoutes({
      "POST /api/auth/login": { mfa: "required" },
      "POST /api/auth/login/mfa": reply(204),
      "GET /api/auth/me": meFixture("member", true, true),
    });
    const { w, router, session } = await mountView(LoginView, { path: "/login?return=/invoices" });
    await password(w);
    expect(calls(fetch)).toEqual(["POST /api/auth/login"]);
    expect(session.me).toBeNull();
    expect(w.find('[data-test="login-password"]').exists()).toBe(false);
    expect(w.find('label[for="login-code"]').text()).toBe("Code from the app");
    expect(w.find('[data-test="login-code"]').attributes("inputmode")).toBe("numeric");

    await code(w, "");
    expect(calls(fetch)).toHaveLength(1);
    expect(w.find("#login-code-error").text()).toBe(i18n.global.t("validation.required"));

    await code(w, " 123456 ");
    expect(sentRequest(fetch, 1)).toMatchObject({ url: "/api/auth/login/mfa", body: { code: "123456" } });
    expect(session.mfaEnabled).toBe(true);
    expect(router.currentRoute.value.path).toBe("/invoices");
  });

  it("accepts a recovery code instead", async () => {
    const fetch = mockFetchRoutes({
      "POST /api/auth/login": { mfa: "required" },
      "POST /api/auth/login/mfa": reply(204),
      "GET /api/auth/me": meFixture("member", true, true),
    });
    const { w, router } = await mountView(LoginView, { path: "/login" });
    await password(w);
    await w.find('[data-test="login-recovery-toggle"]').trigger("click");
    expect(w.find('label[for="login-code"]').text()).toBe("Recovery code");
    expect(w.find('[data-test="login-code"]').attributes("inputmode")).toBe("text");
    expect(w.find('[data-test="login-recovery-toggle"]').text()).toBe("Use a code from the app");
    await code(w, "abcde-fghij");
    expect(sentRequest(fetch, 1).body).toEqual({ code: "abcde-fghij" });
    expect(router.currentRoute.value.path).toBe("/");
  });

  it("explains a wrong code and stays on the code step", async () => {
    mockFetchRoutes({ "POST /api/auth/login": { mfa: "required" }, "POST /api/auth/login/mfa": reply(401, { code: "mfa_invalid" }) });
    const { w, router } = await mountView(LoginView, { path: "/login" });
    await password(w);
    await code(w, "000000");
    expect(w.find('[data-test="login-code-error"]').text()).toBe("Wrong code.");
    expect(w.find('[data-test="login-code"]').exists()).toBe(true);
    expect(router.currentRoute.value.name).toBe("login");
  });

  it("explains a rate limit on the code step", async () => {
    mockFetchRoutes({ "POST /api/auth/login": { mfa: "required" }, "POST /api/auth/login/mfa": reply(429, { code: "rate_limited" }) });
    const { w } = await mountView(LoginView, { path: "/login" });
    await password(w);
    await code(w, "000000");
    expect(w.find('[data-test="login-code-error"]').text()).toBe("Too many attempts. Please try again later.");
  });

  it("goes back to the password step when the pending login expired", async () => {
    mockFetchRoutes({ "POST /api/auth/login": { mfa: "required" }, "POST /api/auth/login/mfa": reply(401, { code: "invalid_credentials" }) });
    const { w } = await mountView(LoginView, { path: "/login" });
    await password(w);
    await code(w, "123456");
    expect(w.find('[data-test="login-code"]').exists()).toBe(false);
    expect((w.find('[data-test="login-password"]').element as HTMLInputElement).value).toBe("");
    expect((w.find('[data-test="login-email"]').element as HTMLInputElement).value).toBe("jana@example.cz");
    expect(w.find('[data-test="login-error"]').text()).toBe("The sign-in has expired. Enter your password again.");
  });

  it("explains a space requiring TOTP with a link to the account on the base host", async () => {
    mockFetchRoutes({ "POST /api/auth/login": reply(403, { code: "mfa_required" }) });
    const { w } = await mountView(LoginView, { path: "/login", context: SPACE_CONTEXT });
    await password(w);
    expect(w.find('[data-test="login-error"]').text()).toContain("This space requires two-factor authentication");
    expect(w.find('[data-test="login-mfa-setup"]').attributes("href")).toBe("http://localhost:3000/account");
  });

  it("shows no setup link for other errors", async () => {
    mockFetchRoutes({ "POST /api/auth/login": reply(401, { code: "invalid_credentials" }) });
    const { w } = await mountView(LoginView, { path: "/login" });
    await password(w);
    expect(w.find('[data-test="login-error"]').text()).toBe("Wrong e-mail or password.");
    expect(w.find('[data-test="login-mfa-setup"]').exists()).toBe(false);
  });
});
