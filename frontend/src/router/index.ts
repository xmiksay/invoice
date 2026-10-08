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
