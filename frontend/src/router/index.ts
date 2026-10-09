import {
  createRouter,
  type RouteLocationNormalized,
  type RouteLocationRaw,
  type RouterHistory,
} from "vue-router";
import { useAuthStore } from "@/stores/auth";

declare module "vue-router" {
  interface RouteMeta {
    /** Reachable without a token. */
    public?: boolean;
  }
}

const routes = [
  {
    path: "/login",
    name: "login",
    component: () => import("@/views/LoginView.vue"),
    meta: { public: true },
  },
  { path: "/", name: "home", redirect: { name: "invoices" } },
  {
    path: "/invoices",
    name: "invoices",
    component: () => import("@/features/documents/views/InvoicesListView.vue"),
  },
  {
    path: "/invoices/new",
    name: "invoice-new",
    component: () => import("@/features/documents/views/InvoiceEditView.vue"),
  },
  {
    path: "/invoices/:id",
    name: "invoice-detail",
    component: () => import("@/features/documents/views/InvoiceDetailView.vue"),
  },
  {
    path: "/invoices/:id/edit",
    name: "invoice-edit",
    component: () => import("@/features/documents/views/InvoiceEditView.vue"),
  },
  {
    path: "/received",
    name: "received",
    component: () => import("@/features/received/views/ReceivedListView.vue"),
  },
  {
    path: "/received/new",
    name: "received-new",
    component: () => import("@/features/received/views/ReceivedEditView.vue"),
  },
  {
    path: "/received/:id",
    name: "received-detail",
    component: () => import("@/features/received/views/ReceivedDetailView.vue"),
  },
  {
    path: "/received/:id/edit",
    name: "received-edit",
    component: () => import("@/features/received/views/ReceivedEditView.vue"),
  },
  {
    path: "/import/isdoc",
    name: "isdoc-import",
    component: () => import("@/features/isdoc/views/IsdocImportView.vue"),
  },
  {
    path: "/contacts",
    name: "contacts",
    component: () => import("@/features/contacts/views/ContactsListView.vue"),
  },
  {
    path: "/contacts/new",
    name: "contact-new",
    component: () => import("@/features/contacts/views/ContactEditView.vue"),
  },
  {
    path: "/contacts/:id",
    name: "contact-edit",
    component: () => import("@/features/contacts/views/ContactEditView.vue"),
  },
  {
    path: "/catalog",
    component: () => import("@/features/catalog/views/CatalogView.vue"),
    children: [
      { path: "", redirect: { name: "catalog-items" } },
      {
        path: "items",
        name: "catalog-items",
        component: () => import("@/features/catalog/views/CatalogItemsTab.vue"),
      },
      {
        path: "groups",
        name: "catalog-groups",
        component: () => import("@/features/catalog/views/CatalogGroupsTab.vue"),
      },
    ],
  },
  {
    path: "/settings",
    component: () => import("@/features/settings/views/SettingsView.vue"),
    children: [
      { path: "", redirect: { name: "settings-company" } },
      {
        path: "company",
        name: "settings-company",
        component: () => import("@/features/settings/views/CompanyTab.vue"),
      },
      {
        path: "bank-accounts",
        name: "settings-bank-accounts",
        component: () => import("@/features/settings/views/BankAccountsTab.vue"),
      },
      {
        path: "vat-rates",
        name: "settings-vat-rates",
        component: () => import("@/features/settings/views/VatRatesTab.vue"),
      },
      {
        path: "number-series",
        name: "settings-number-series",
        component: () => import("@/features/settings/views/NumberSeriesTab.vue"),
      },
      {
        path: "categories",
        name: "settings-categories",
        component: () => import("@/features/settings/views/CategoriesTab.vue"),
      },
      {
        path: "custom-fields",
        name: "settings-custom-fields",
        component: () => import("@/features/settings/views/CustomFieldsTab.vue"),
      },
      {
        path: "design",
        name: "settings-design",
        component: () => import("@/features/settings/views/DesignTab.vue"),
      },
    ],
  },
  { path: "/:pathMatch(.*)*", redirect: "/" },
];

export function authGuard(to: RouteLocationNormalized): RouteLocationRaw | true {
  const auth = useAuthStore();
  if (to.meta.public) {
    return to.name === "login" && auth.isAuthenticated ? { name: "home" } : true;
  }
  if (!auth.isAuthenticated) {
    return { name: "login", query: { redirect: to.fullPath } };
  }
  return true;
}

export function createAppRouter(history: RouterHistory) {
  const router = createRouter({ history, routes });
  router.beforeEach(authGuard);
  return router;
}
