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
  { path: "/", name: "home", component: () => import("@/views/HomeView.vue") },
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
