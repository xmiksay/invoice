<script setup lang="ts">
import { useI18n } from "vue-i18n";

/** "Import selected (n)" + how many of the previewed entries are selected. */
defineProps<{ count: number; total: number; busy: boolean }>();
defineEmits<{ confirm: [] }>();

const { t } = useI18n();
</script>

<template>
  <div class="flex flex-wrap items-center gap-3">
    <button type="button" class="btn btn-primary" :disabled="busy || count === 0" data-test="import-confirm" @click="$emit('confirm')">
      {{ t("imports.page.confirm", { n: count }) }}
    </button>
    <span class="text-sm text-gray-500">{{ t("imports.page.selected", { n: count, total }) }}</span>
  </div>
</template>
