import type { RouteRecordRaw } from "vue-router";
import { useSessionStore } from "@/stores/session";
import { accountRoutes } from "./authRoutes";

/** The app on the base host: registration, sign-in and the "Moje spaces" hub. */
export const baseRoutes: RouteRecordRaw[] = [
  ...accountRoutes,
  {
    path: "/",
    name: "home",
    component: () => import("@/features/spaces/views/SpacesView.vue"),
  },
  {
    path: "/register",
    name: "register",
    component: () => import("@/features/auth/views/RegisterView.vue"),
    meta: { public: true, guestOnly: true },
    beforeEnter: () => (useSessionStore().context?.registration ? true : { name: "login" }),
  },
  {
    // With `?token` from the e-mail: confirms; without: "verify your e-mail" + resend.
    path: "/verify",
    name: "verify",
    component: () => import("@/features/auth/views/VerifyView.vue"),
    meta: { public: true },
  },
  {
    path: "/reset",
    name: "reset",
    component: () => import("@/features/auth/views/ResetView.vue"),
    meta: { public: true },
  },
  { path: "/:pathMatch(.*)*", redirect: "/" },
];
