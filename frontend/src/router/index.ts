import {
  createRouter,
  type RouteLocationNormalized,
  type RouteLocationRaw,
  type RouterHistory,
} from "vue-router";
import type { Boot } from "@/boot";
import type { Action } from "@/features/spaces/roles";
import { useSessionStore } from "@/stores/session";
import type { RouteRecordRaw } from "vue-router";
import { baseRoutes } from "./baseRoutes";
import { spaceRoutes } from "./spaceRoutes";

declare module "vue-router" {
  interface RouteMeta {
    /** Reachable without a session. */
    public?: boolean;
    /** A signed-in user is sent home instead (login, register, forgot). */
    guestOnly?: boolean;
    /** Minimum permission; the route (and its children) is hidden from lower roles. */
    can?: Action;
  }
}

export async function authGuard(to: RouteLocationNormalized): Promise<RouteLocationRaw | true> {
  if (to.name === "boot-failure") return true;
  const session = useSessionStore();
  if (!session.loaded) await session.loadMe();
  if (to.meta.public) {
    return to.meta.guestOnly && session.me ? { name: "home" } : true;
  }
  if (!session.me) return { name: "login", query: { return: to.fullPath } };
  if (to.meta.can && !session.can(to.meta.can)) return { name: "home" };
  return true;
}

/** Unknown host or unreachable backend: one page for every path. */
const failureRoutes: RouteRecordRaw[] = [
  {
    path: "/:pathMatch(.*)*",
    name: "boot-failure",
    component: () => import("@/features/spaces/views/BootFailureView.vue"),
    meta: { public: true },
  },
];

export function routesFor(boot: Boot) {
  if (boot.status !== "ok") return failureRoutes;
  return boot.context.kind === "space" ? spaceRoutes : baseRoutes;
}

export function createAppRouter(history: RouterHistory, boot: Boot) {
  const router = createRouter({ history, routes: routesFor(boot) });
  router.beforeEach(authGuard);
  return router;
}
