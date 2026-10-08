import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { TOKEN_STORAGE_KEY, useAuthStore } from "./auth";
import { mockFetch, sentHeaders } from "@/test-utils";

describe("auth store", () => {
  beforeEach(() => {
    localStorage.clear();
    setActivePinia(createPinia());
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("restores the token from localStorage", () => {
    localStorage.setItem(TOKEN_STORAGE_KEY, "saved");
    expect(useAuthStore().token).toBe("saved");
  });

  it("login stores the token after a 204 from /api/auth/check", async () => {
    const fetch = mockFetch(204);
    const auth = useAuthStore();

    await auth.login("  good  ");

    expect(fetch.mock.calls[0]?.[0]).toBe("/api/auth/check");
    expect(sentHeaders(fetch).get("Authorization")).toBe("Bearer good");
    expect(auth.token).toBe("good");
    expect(auth.isAuthenticated).toBe(true);
    expect(localStorage.getItem(TOKEN_STORAGE_KEY)).toBe("good");
  });

  it("login does not store a token rejected with 401", async () => {
    mockFetch(401, { code: "unauthorized" });
    const auth = useAuthStore();

    await expect(auth.login("bad")).rejects.toMatchObject({ status: 401 });

    expect(auth.token).toBeNull();
    expect(localStorage.getItem(TOKEN_STORAGE_KEY)).toBeNull();
  });

  it("logout clears state and storage", async () => {
    mockFetch(204);
    const auth = useAuthStore();
    await auth.login("good");

    auth.logout();

    expect(auth.token).toBeNull();
    expect(auth.isAuthenticated).toBe(false);
    expect(localStorage.getItem(TOKEN_STORAGE_KEY)).toBeNull();
  });

  it("survives localStorage throwing", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    expect(useAuthStore().token).toBeNull();
    vi.restoreAllMocks();
  });
});
