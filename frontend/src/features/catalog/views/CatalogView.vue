<script setup lang="ts">
import { useI18n } from "vue-i18n";

const { t } = useI18n();

const tabs = [
  { name: "catalog-items", label: "catalog.tabs.items" },
  { name: "catalog-groups", label: "catalog.tabs.groups" },
] as const;
</script>

<template>
  <section class="space-y-4">
    <h1 class="text-2xl font-semibold">{{ t("catalog.title") }}</h1>
    <nav :aria-label="t('catalog.title')" class="-mx-4 overflow-x-auto [scrollbar-width:none] border-b border-gray-200 px-4 sm:mx-0 sm:px-0 dark:border-gray-800">
      <ul class="flex gap-1 whitespace-nowrap">
        <li v-for="tab in tabs" :key="tab.name">
          <RouterLink
            :to="{ name: tab.name }"
            class="inline-block border-b-2 border-transparent px-3 py-2 text-sm text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-gray-100"
            exact-active-class="!border-blue-600 !text-gray-900 font-medium dark:!text-gray-100"
          >
            {{ t(tab.label) }}
          </RouterLink>
        </li>
      </ul>
    </nav>
    <RouterView />
  </section>
</template>
