import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { createMemoryHistory, createRouter } from "vue-router";
import { i18n } from "@/i18n";
import { mockFetchRoutes } from "@/test-utils";
import { bankAccounts, company, totals, vatRates } from "../testData";
import InvoiceEditView from "./InvoiceEditView.vue";

async function mountAt(path: string) {
  const pinia = createPinia();
  setActivePinia(pinia);
  const stub = { template: "<div />" };
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: "/invoices", name: "invoices", component: stub },
      { path: "/invoices/new", name: "invoice-new", component: InvoiceEditView },
      { path: "/invoices/:id", name: "invoice-detail", component: stub },
      { path: "/contacts/new", name: "contact-new", component: stub },
    ],
  });
  await router.push(path);
  const w = mount({ template: "<RouterView />" }, { global: { plugins: [pinia, i18n, router] } });
  await flushPromises();
  return w;
}

describe("InvoiceEditView", () => {
  beforeEach(() => {
    i18n.global.locale.value = "en";
    mockFetchRoutes({
      "GET /api/settings/company": company,
      "GET /api/settings/vat-rates": vatRates,
      "GET /api/settings/bank-accounts": bankAccounts,
      "POST /api/documents/compute": { lines: [], totals: totals() },
    });
  });
  afterEach(() => vi.unstubAllGlobals());

  it("new invoice by default", async () => {
    const w = await mountAt("/invoices/new");
    expect(w.find("h1").text()).toBe("New invoice");
    expect(w.find("#doc-taxPointDate").exists()).toBe(true);
  });

  it("?docType=proforma opens the proforma mode without a tax point date", async () => {
    const w = await mountAt("/invoices/new?docType=proforma");
    expect(w.find("h1").text()).toBe("New proforma invoice");
    expect(w.find("#doc-taxPointDate").exists()).toBe(false);
    expect(w.find("a").attributes("href")).toBe("/invoices?type=proforma");
  });
});
