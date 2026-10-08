<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRoute, useRouter } from "vue-router";
import { ApiError } from "@/api/client";
import { useAuthStore } from "@/stores/auth";

const { t } = useI18n();
const route = useRoute();
const router = useRouter();
const auth = useAuthStore();

const token = ref("");
const submitting = ref(false);
const error = ref<string | null>(null);

// Statuses a reverse proxy / the Vite dev proxy returns when the backend is down.
const UNREACHABLE = new Set([0, 502, 503, 504]);

function errorMessage(err: unknown): string {
  if (err instanceof ApiError) {
    if (err.status === 401) return t("errors.invalidToken");
    if (UNREACHABLE.has(err.status)) return t("errors.unreachable");
    return t("errors.unexpected", { code: err.code });
  }
  return t("errors.unexpected", { code: "unknown" });
}

function redirectTarget(): string {
  const target = route.query.redirect;
  // Only same-app absolute paths; "//host" would be protocol-relative.
  return typeof target === "string" && target.startsWith("/") && !target.startsWith("//")
    ? target
    : "/";
}

async function submit() {
  if (!token.value.trim() || submitting.value) return;
  submitting.value = true;
  error.value = null;
  try {
    await auth.login(token.value);
    await router.replace(redirectTarget());
  } catch (err) {
    error.value = errorMessage(err);
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <div class="flex min-h-[70dvh] items-center justify-center">
    <form
      class="w-full max-w-sm space-y-4 rounded-lg border border-gray-200 bg-white p-6 shadow-sm dark:border-gray-800 dark:bg-gray-900"
      @submit.prevent="submit"
    >
      <h1 class="text-xl font-semibold">{{ t("auth.login.title") }}</h1>
      <div class="space-y-1">
        <label for="token" class="block text-sm font-medium">{{ t("auth.login.tokenLabel") }}</label>
        <input
          id="token"
          v-model="token"
          type="password"
          autocomplete="current-password"
          required
          autofocus
          class="w-full rounded-md border border-gray-300 bg-white px-3 py-2 focus:border-blue-500 focus:ring-2 focus:ring-blue-500/30 focus:outline-none dark:border-gray-700 dark:bg-gray-950"
        />
        <p class="text-xs text-gray-500 dark:text-gray-400">{{ t("auth.login.hint") }}</p>
      </div>
      <p v-if="error" role="alert" class="text-sm text-red-600 dark:text-red-400">{{ error }}</p>
      <button
        type="submit"
        :disabled="submitting"
        class="w-full rounded-md bg-blue-600 px-4 py-2 font-medium text-white hover:bg-blue-700 disabled:opacity-60"
      >
        {{ submitting ? t("auth.login.submitting") : t("auth.login.submit") }}
      </button>
    </form>
  </div>
</template>
