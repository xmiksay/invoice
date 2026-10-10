import type { RouteRecordRaw } from "vue-router";

/** Sign-in and account routes every host has. */
export const accountRoutes: RouteRecordRaw[] = [
  {
    path: "/login",
    name: "login",
    component: () => import("@/features/auth/views/LoginView.vue"),
    meta: { public: true, guestOnly: true },
  },
  {
    path: "/forgot",
    name: "forgot",
    component: () => import("@/features/auth/views/ForgotView.vue"),
    meta: { public: true, guestOnly: true },
  },
  {
    path: "/account",
    name: "account",
    component: () => import("@/features/auth/views/AccountView.vue"),
  },
];
