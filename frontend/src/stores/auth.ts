import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { request } from "@/api/client";
import { readStorage, writeStorage } from "@/lib/storage";

export const TOKEN_STORAGE_KEY = "invoice.token";

export const useAuthStore = defineStore("auth", () => {
  const token = ref<string | null>(readStorage(TOKEN_STORAGE_KEY));
  const isAuthenticated = computed(() => token.value !== null);

  /** Validates the token against the backend first; throws `ApiError` if rejected. */
  async function login(candidate: string): Promise<void> {
    const trimmed = candidate.trim();
    await request<void>("/api/auth/check", { token: trimmed });
    token.value = trimmed;
    writeStorage(TOKEN_STORAGE_KEY, trimmed);
  }

  function logout(): void {
    token.value = null;
    writeStorage(TOKEN_STORAGE_KEY, null);
  }

  return { token, isAuthenticated, login, logout };
});
