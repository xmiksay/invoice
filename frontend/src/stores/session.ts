import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { authApi } from "@/features/auth/api";
import type { LoginBody, Me } from "@/features/auth/types";
import { can as roleCan, type Action } from "@/features/spaces/roles";
import type { AppContext } from "@/features/spaces/types";
import type { Boot } from "@/boot";

/** Host context (from boot) + the signed-in user; the session itself is the HttpOnly cookie. */
export const useSessionStore = defineStore("session", () => {
  const context = ref<AppContext | null>(null);
  /** Why there is no context (unknown host, backend down); shown by the boot-failure page. */
  const bootFailure = ref<Exclude<Boot, { status: "ok" }> | null>(null);
  const me = ref<Me | null>(null);
  /** False until the first `/api/auth/me` answered (the router guard waits for it). */
  const loaded = ref(false);

  const role = computed(() => me.value?.space?.role ?? null);
  const isSpace = computed(() => context.value?.kind === "space");
  const baseUrl = computed(() => context.value?.baseUrl ?? "/");

  function can(action: Action): boolean {
    return roleCan(role.value, action);
  }

  /** Any failure (401, backend down) counts as signed out; the login screen reports the rest. */
  async function loadMe(): Promise<Me | null> {
    try {
      me.value = await authApi.me();
    } catch {
      me.value = null;
    }
    loaded.value = true;
    return me.value;
  }

  async function login(body: LoginBody): Promise<void> {
    await authApi.login(body);
    me.value = await authApi.me();
    loaded.value = true;
  }

  function clear(): void {
    me.value = null;
    loaded.value = true;
  }

  /** Never fails: if the server call does, the user still lands signed out here (best effort). */
  async function logout(): Promise<void> {
    try {
      await authApi.logout();
    } catch {
      // Expired already, or the backend is down; nothing the user can act on.
    }
    clear();
  }

  /** After a rename: the header shows the new name without a reload. */
  function renameSpace(name: string): void {
    if (context.value?.space) context.value.space.name = name;
    if (me.value?.space) me.value.space.name = name;
  }

  /** Applies the result of `loadContext()` (main.ts, tests). */
  function applyBoot(boot: Boot): void {
    context.value = boot.status === "ok" ? boot.context : null;
    bootFailure.value = boot.status === "ok" ? null : boot;
  }

  return { context, bootFailure, applyBoot, me, loaded, role, isSpace, baseUrl, can, loadMe, login, logout, clear, renameSpace };
});
