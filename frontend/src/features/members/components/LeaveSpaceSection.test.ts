import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises } from "@vue/test-utils";
import { i18n } from "@/i18n";
import AccountView from "@/features/auth/views/AccountView.vue";
import type { Role } from "@/features/spaces/types";
import { leaveTo } from "@/lib/navigate";
import { calls, mockFetchRoutes, reply } from "@/test-utils";
import { mountView } from "@/testMount";
import type { Member } from "../types";

vi.mock("@/lib/navigate", () => ({ leaveTo: vi.fn() }));

const owner = (userId: string): Member => ({
  userId,
  email: `${userId}@example.cz`,
  displayName: userId,
  role: "owner",
  joinedAt: "2026-10-01T10:00:00Z",
  isSelf: userId === "me",
});

describe("Leave space (account page)", () => {
  beforeEach(() => {
    i18n.global.locale.value = "en";
    vi.mocked(leaveTo).mockClear();
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  const mountAccount = (role: Role | null) => mountView(AccountView, { path: "/account", role });

  it("is not offered on the base host", async () => {
    mockFetchRoutes({});
    const { w } = await mountAccount(null);
    expect(w.find('[data-test="leave-space"]').exists()).toBe(false);
  });

  it("leaves after confirmation and goes to the base host, without listing members below owner", async () => {
    const fetch = mockFetchRoutes({ "POST /api/space/leave": reply(204) });
    vi.spyOn(window, "confirm").mockReturnValueOnce(false).mockReturnValueOnce(true);
    const { w } = await mountAccount("admin");
    await w.find('[data-test="leave-submit"]').trigger("click");
    await flushPromises();
    expect(fetch).not.toHaveBeenCalled();
    await w.find('[data-test="leave-submit"]').trigger("click");
    await flushPromises();
    expect(calls(fetch)).toEqual(["POST /api/space/leave"]);
    expect(leaveTo).toHaveBeenCalledWith("http://localhost:3000");
  });

  it("hides the button for the last owner and explains why", async () => {
    mockFetchRoutes({ "GET /api/members": [owner("me")] });
    const { w } = await mountAccount("owner");
    expect(w.find('[data-test="leave-last-owner"]').text()).toContain("only owner");
    expect(w.find('[data-test="leave-submit"]').exists()).toBe(false);
  });

  it("lets one of several owners leave", async () => {
    mockFetchRoutes({ "GET /api/members": [owner("me"), owner("o2")] });
    const { w } = await mountAccount("owner");
    expect(w.find('[data-test="leave-submit"]').exists()).toBe(true);
  });

  it("shows the server's last_owner refusal", async () => {
    mockFetchRoutes({ "GET /api/members": reply(500, { code: "internal" }), "POST /api/space/leave": reply(409, { code: "last_owner" }) });
    vi.spyOn(window, "confirm").mockReturnValue(true);
    const { w } = await mountAccount("owner");
    await w.find('[data-test="leave-submit"]').trigger("click");
    await flushPromises();
    expect(w.find('[data-test="leave-error"]').text()).toBe(i18n.global.t("errors.lastOwner"));
    expect(leaveTo).not.toHaveBeenCalled();
  });
});
