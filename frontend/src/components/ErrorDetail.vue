<script setup lang="ts">
import { useI18n } from "vue-i18n";

/** Collapsible server diagnostic (e.g. the typst error of `pdf_render_failed`). */
defineProps<{ detail: string | null; open?: boolean }>();
const { t } = useI18n();
</script>

<template>
  <details v-if="detail" class="mt-1" :open="open" data-test="error-detail">
    <summary class="cursor-pointer select-none">{{ t("common.details") }}</summary>
    <pre class="mt-1 max-h-80 overflow-auto rounded bg-red-100/60 p-2 text-xs whitespace-pre-wrap dark:bg-red-950">{{ detail }}</pre>
  </details>
</template>
