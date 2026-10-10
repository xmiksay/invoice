import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { createMemoryHistory, createRouter } from "vue-router";
import { contact } from "@/features/documents/testData";
import type { Role } from "@/features/spaces/types";
import { i18n } from "@/i18n";
import { mockFetchRoutes, signIn } from "@/test-utils";
import ContactEditView from "./ContactEditView.vue";

async function mountContact(role: Role) {
  const pinia = createPinia();
  setActivePinia(pinia);
  signIn(role);
  const stub = { template: "<div />" };
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: "/contacts", name: "contacts", component: stub },
      { path: "/contacts/new", name: "contact-new", component: stub },
      { path: "/contacts/:id", name: "contact-edit", component: ContactEditView },
    ],
  });
  await router.push("/contacts/c1");
  const w = mount({ template: "<RouterView />" }, { global: { plugins: [pinia, i18n, router] } });
  await flushPromises();
  return w;
}

describe("ContactEditView roles", () => {
  beforeEach(() => {
    i18n.global.locale.value = "en";
  });
  afterEach(() => vi.unstubAllGlobals());

  it("is read-only for an accountant: disabled fields, no Save, no Delete", async () => {
    const fetch = mockFetchRoutes({ "GET /api/contacts/c1": contact() });
    const w = await mountContact("accountant");
    expect(w.find('[data-test="contact-fields"]').attributes("disabled")).toBeDefined();
    expect(w.find('[data-test="contact-save"]').exists()).toBe(false);
    expect(w.find(".btn-danger").exists()).toBe(false);
    await w.find("form").trigger("submit");
    await flushPromises();
    expect(fetch).toHaveBeenCalledTimes(1);
  });

  it("is editable for a member", async () => {
    mockFetchRoutes({ "GET /api/contacts/c1": contact() });
    const w = await mountContact("member");
    expect(w.find('[data-test="contact-fields"]').attributes("disabled")).toBeUndefined();
    expect(w.find('[data-test="contact-save"]').exists()).toBe(true);
    expect(w.find(".btn-danger").exists()).toBe(true);
  });
});
