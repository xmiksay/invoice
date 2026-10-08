<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { LOCALES, setLocale, type Locale } from "@/i18n";
import { useAuthStore } from "@/stores/auth";

const { t, locale } = useI18n();
const router = useRouter();
const auth = useAuthStore();

function onLocaleChange(event: Event) {
  setLocale((event.target as HTMLSelectElement).value as Locale);
}

async function logout() {
  auth.logout();
  await router.push({ name: "login" });
}
</script>

<template>
  <header class="border-b border-gray-200 bg-white dark:border-gray-800 dark:bg-gray-900">
    <div class="mx-auto flex max-w-5xl flex-wrap items-center gap-x-6 gap-y-2 px-4 py-3 sm:px-6">
      <RouterLink :to="{ name: 'home' }" class="text-lg font-semibold">
        {{ t("app.name") }}
      </RouterLink>

      <nav :aria-label="t('nav.main')" class="flex flex-1 gap-4 text-sm">
        <RouterLink
          :to="{ name: 'home' }"
          class="text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-gray-100"
          active-class="!text-gray-900 dark:!text-gray-100 font-medium"
        >
          {{ t("nav.invoices") }}
        </RouterLink>
      </nav>

      <div class="flex items-center gap-3">
        <label class="sr-only" for="locale-select">{{ t("locale.label") }}</label>
        <select
          id="locale-select"
          :value="locale"
          class="rounded-md border border-gray-300 bg-white px-2 py-1 text-sm uppercase dark:border-gray-700 dark:bg-gray-900"
          @change="onLocaleChange"
        >
          <option v-for="code in LOCALES" :key="code" :value="code">{{ code }}</option>
        </select>
        <button
          type="button"
          class="rounded-md px-3 py-1 text-sm text-gray-700 hover:bg-gray-100 dark:text-gray-300 dark:hover:bg-gray-800"
          @click="logout"
        >
          {{ t("auth.logout") }}
        </button>
      </div>
    </div>
  </header>
</template>
