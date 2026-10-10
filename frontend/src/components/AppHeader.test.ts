import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises } from "@vue/test-utils";
import type { Role } from "@/features/spaces/types";
import { i18n } from "@/i18n";
import { calls, mockFetchRoutes, reply } from "@/test-utils";
import { mountView } from "@/testMount";
import AppHeader from "./AppHeader.vue";

const navOf = (w: Awaited<ReturnType<typeof mountView>>["w"]) => w.findAll("nav a").map((a) => a.attributes("data-test"));

describe("AppHeader", () => {
  beforeEach(() => {
    i18n.global.locale.value = "en";
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    document.body.innerHTML = "";
  });

  const mountAs = (role: Role | null) => mountView(AppHeader, { path: "/invoices", role });

  it.each<[Role, string[]]>([
    ["accountant", ["nav-invoices", "nav-received", "nav-contacts", "nav-catalog"]],
    ["member", ["nav-invoices", "nav-received", "nav-contacts", "nav-catalog"]],
    ["admin", ["nav-invoices", "nav-received", "nav-contacts", "nav-catalog", "nav-settings"]],
    ["owner", ["nav-invoices", "nav-received", "nav-contacts", "nav-catalog", "nav-settings"]],
  ])("shows %s the links the role may open", async (role, links) => {
    mockFetchRoutes({});
    const { w } = await mountAs(role);
    expect(navOf(w)).toEqual(links);
  });

  it("titles a space with its name and has the space user menu", async () => {
    mockFetchRoutes({});
    const { w } = await mountAs("member");
    expect(w.find('[data-test="header-title"]').text()).toBe("Firma s.r.o.");
    expect(w.find('[data-test="menu-account"]').exists()).toBe(true);
    expect(w.find('[data-test="menu-tokens"]').exists()).toBe(true);
    expect(w.find('[data-test="menu-spaces"]').attributes("href")).toBe("http://localhost:3000");
  });

  it("on the base host has no business links and no space menu items", async () => {
    mockFetchRoutes({});
    const { w } = await mountAs(null);
    expect(navOf(w)).toEqual([]);
    expect(w.find('[data-test="header-title"]').text()).toBe(i18n.global.t("app.name"));
    expect(w.find('[data-test="menu-tokens"]').exists()).toBe(false);
    expect(w.find('[data-test="menu-spaces"]').exists()).toBe(false);
  });

  it("logs out and goes to the login page, even if the server call fails", async () => {
    const fetch = mockFetchRoutes({ "POST /api/auth/logout": reply(500, { code: "internal" }) });
    const { w, router, session } = await mountAs("member");
    await w.find('[data-test="menu-logout"]').trigger("click");
    await flushPromises();
    expect(calls(fetch)).toEqual(["POST /api/auth/logout"]);
    expect(session.me).toBeNull();
    expect(router.currentRoute.value.name).toBe("login");
  });
});
