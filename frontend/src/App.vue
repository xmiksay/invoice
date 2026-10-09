<script setup lang="ts">
import { computed, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useRoute } from "vue-router";
import AppHeader from "@/components/AppHeader.vue";
import AppToast from "@/components/AppToast.vue";
import { useHealthStore } from "@/stores/health";

const { t } = useI18n();
const route = useRoute();
const health = useHealthStore();
const showHeader = computed(() => route.name !== undefined && route.name !== "login");

watch(
  showHeader,
  (shown) => {
    if (shown && !health.version) void health.load();
  },
  { immediate: true },
);
</script>

<template>
  <div class="flex min-h-dvh flex-col">
    <AppHeader v-if="showHeader" />
    <main class="mx-auto w-full max-w-5xl flex-1 px-4 py-6 sm:px-6">
      <RouterView />
    </main>
    <footer v-if="showHeader" class="mx-auto w-full max-w-5xl px-4 pb-4 text-xs text-gray-500 sm:px-6">
      <template v-if="health.version">{{ t("app.backendVersion", { version: health.version }) }}</template>
      <template v-else-if="health.failed">{{ t("app.backendUnavailable") }}</template>
    </footer>
    <AppToast />
  </div>
</template>
