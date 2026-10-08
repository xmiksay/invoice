import { beforeEach, describe, expect, it } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { createMemoryHistory } from "vue-router";
import { createAppRouter } from "./index";
import { useAuthStore } from "@/stores/auth";

describe("router auth guard", () => {
  beforeEach(() => {
    localStorage.clear();
    setActivePinia(createPinia());
  });

  it("redirects to /login without a token and keeps the target", async () => {
    const router = createAppRouter(createMemoryHistory());
    await router.push("/?tab=x");
    expect(router.currentRoute.value.name).toBe("login");
    expect(router.currentRoute.value.query.redirect).toBe("/?tab=x");
  });

  it("allows /login without a token", async () => {
    const router = createAppRouter(createMemoryHistory());
    await router.push("/login");
    expect(router.currentRoute.value.name).toBe("login");
  });

  it("allows protected routes with a token", async () => {
    useAuthStore().token = "t";
    const router = createAppRouter(createMemoryHistory());
    await router.push("/");
    expect(router.currentRoute.value.name).toBe("home");
  });

  it("sends an authenticated user away from /login", async () => {
    useAuthStore().token = "t";
    const router = createAppRouter(createMemoryHistory());
    await router.push("/login");
    expect(router.currentRoute.value.name).toBe("home");
  });

  it("redirects unknown paths to home", async () => {
    useAuthStore().token = "t";
    const router = createAppRouter(createMemoryHistory());
    await router.push("/nope/deep");
    expect(router.currentRoute.value.name).toBe("home");
  });
});
