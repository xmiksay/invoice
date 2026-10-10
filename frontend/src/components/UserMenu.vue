<script setup lang="ts">
import { useTemplateRef } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { useSessionStore } from "@/stores/session";

const { t } = useI18n();
const router = useRouter();
const session = useSessionStore();
const menu = useTemplateRef<HTMLDetailsElement>("menu");

function close() {
  if (menu.value) menu.value.open = false;
}

async function logout() {
  close();
  await session.logout();
  await router.push({ name: "login" });
}

const item = "block w-full px-3 py-2 text-left text-sm hover:bg-gray-100 dark:hover:bg-gray-800";
</script>

<template>
  <details ref="menu" class="relative" data-test="user-menu">
    <summary
      class="cursor-pointer list-none rounded-md px-3 py-1 text-sm text-gray-700 hover:bg-gray-100 dark:text-gray-300 dark:hover:bg-gray-800"
      :aria-label="t('auth.menu.label')"
    >
      {{ session.me?.user.displayName }} ▾
    </summary>
    <div class="absolute right-0 z-30 mt-1 w-48 overflow-hidden rounded-md border border-gray-200 bg-white shadow-lg dark:border-gray-800 dark:bg-gray-900">
      <RouterLink :to="{ name: 'account' }" :class="item" data-test="menu-account" @click="close">{{ t("auth.menu.account") }}</RouterLink>
      <template v-if="session.isSpace">
        <RouterLink :to="{ name: 'tokens' }" :class="item" data-test="menu-tokens" @click="close">{{ t("auth.menu.tokens") }}</RouterLink>
        <a :href="session.baseUrl" :class="item" data-test="menu-spaces">{{ t("auth.menu.spaces") }}</a>
      </template>
      <button type="button" :class="item" data-test="menu-logout" @click="logout">{{ t("auth.logout") }}</button>
    </div>
  </details>
</template>
