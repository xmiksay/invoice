import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { createMemoryHistory, createRouter } from "vue-router";
import { i18n } from "@/i18n";
import InvoiceDetailView from "./InvoiceDetailView.vue";

const stub = { template: "<div />" };

/** Mounts the detail view at `/invoices/{id}`; the router is returned to assert navigation. */
export async function mountDetail(id = "d1") {
  const pinia = createPinia();
  setActivePinia(pinia);
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: "/invoices", name: "invoices", component: stub },
      { path: "/invoices/:id", name: "invoice-detail", component: InvoiceDetailView },
      { path: "/invoices/:id/edit", name: "invoice-edit", component: stub },
    ],
  });
  await router.push(`/invoices/${id}`);
  const w = mount({ template: "<RouterView />" }, { global: { plugins: [pinia, i18n, router] } });
  await flushPromises();
  return { w, router };
}
