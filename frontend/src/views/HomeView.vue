<script setup lang="ts">
import { onMounted } from "vue";
import { useI18n } from "vue-i18n";
import { useHealthStore } from "@/stores/health";

const { t } = useI18n();
const health = useHealthStore();

onMounted(() => {
  void health.load();
});
</script>

<template>
  <section class="space-y-4">
    <h1 class="text-2xl font-semibold">{{ t("home.title") }}</h1>
    <div class="rounded-lg border border-dashed border-gray-300 p-8 text-center text-gray-600 dark:border-gray-700 dark:text-gray-400">
      {{ t("home.comingSoon") }}
    </div>
    <p class="text-xs text-gray-500 dark:text-gray-500">
      <template v-if="health.version">{{ t("home.backendVersion", { version: health.version }) }}</template>
      <template v-else-if="health.failed">{{ t("home.backendUnavailable") }}</template>
    </p>
  </section>
</template>
