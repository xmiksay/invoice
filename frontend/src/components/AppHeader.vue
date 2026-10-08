<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { useRoute, useRouter } from "vue-router";
import { LOCALES, setLocale, type Locale } from "@/i18n";
import { useAuthStore } from "@/stores/auth";

const { t, locale } = useI18n();
const router = useRouter();
const route = useRoute();

const links = [
  { to: "/invoices", label: "nav.invoices" },
  { to: "/contacts", label: "nav.contacts" },
  { to: "/catalog", label: "nav.catalog" },
  { to: "/settings", label: "nav.settings" },
] as const;

// Path prefix match: "/contacts/123" keeps "Contacts" highlighted.
function isActive(to: string): boolean {
  return route.path === to || route.path.startsWith(`${to}/`);
}
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

      <nav :aria-label="t('nav.main')" class="order-last flex w-full gap-4 text-sm sm:order-none sm:w-auto sm:flex-1">
        <RouterLink
          v-for="link in links"
          :key="link.to"
          :to="link.to"
          class="text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-gray-100"
          :class="{ '!text-gray-900 dark:!text-gray-100 font-medium': isActive(link.to) }"
        >
          {{ t(link.label) }}
        </RouterLink>
      </nav>

      <div class="ml-auto flex items-center gap-3">
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
