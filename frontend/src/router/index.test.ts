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
    await router.push("/contacts?tab=x");
    expect(router.currentRoute.value.name).toBe("login");
    expect(router.currentRoute.value.query.redirect).toBe("/contacts?tab=x");
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
    expect(router.currentRoute.value.name).toBe("invoices");
  });

  it("sends an authenticated user away from /login", async () => {
    useAuthStore().token = "t";
    const router = createAppRouter(createMemoryHistory());
    await router.push("/login");
    expect(router.currentRoute.value.name).toBe("invoices");
  });

  it("redirects unknown paths to the invoice list", async () => {
    useAuthStore().token = "t";
    const router = createAppRouter(createMemoryHistory());
    await router.push("/nope/deep");
    expect(router.currentRoute.value.name).toBe("invoices");
  });

  it("resolves feature routes and redirects /settings to the company tab", async () => {
    useAuthStore().token = "t";
    const router = createAppRouter(createMemoryHistory());
    await router.push("/settings");
    expect(router.currentRoute.value.name).toBe("settings-company");
    await router.push("/settings/number-series");
    expect(router.currentRoute.value.name).toBe("settings-number-series");
    await router.push("/settings/design");
    expect(router.currentRoute.value.name).toBe("settings-design");
    await router.push("/contacts/new");
    expect(router.currentRoute.value.name).toBe("contact-new");
    await router.push("/contacts/abc");
    expect(router.currentRoute.value.name).toBe("contact-edit");
    expect(router.currentRoute.value.params.id).toBe("abc");
    await router.push("/invoices/new");
    expect(router.currentRoute.value.name).toBe("invoice-new");
    await router.push("/invoices/d1");
    expect(router.currentRoute.value.name).toBe("invoice-detail");
    await router.push("/catalog");
    expect(router.currentRoute.value.name).toBe("catalog-items");
    await router.push("/catalog/groups");
    expect(router.currentRoute.value.name).toBe("catalog-groups");
    await router.push("/invoices/new?docType=proforma");
    expect(router.currentRoute.value.name).toBe("invoice-new");
    await router.push("/invoices/d1/edit");
    expect(router.currentRoute.value.name).toBe("invoice-edit");
    expect(router.currentRoute.value.params.id).toBe("d1");
    await router.push("/import/csv?from=received");
    expect(router.currentRoute.value.name).toBe("csv-import");
  });
});
