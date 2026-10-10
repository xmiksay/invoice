import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { createMemoryHistory, createRouter } from "vue-router";
import type { Role } from "@/features/spaces/types";
import { i18n } from "@/i18n";
import { signIn } from "@/test-utils";
import InvoiceDetailView from "./InvoiceDetailView.vue";

const stub = { template: "<div />" };

/** Mounts the detail view at `/invoices/{id}` as `role`; the router is returned to assert navigation. */
export async function mountDetail(id = "d1", role: Role = "owner") {
  const pinia = createPinia();
  setActivePinia(pinia);
  signIn(role);
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
