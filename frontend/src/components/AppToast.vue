<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { useToastStore } from "@/stores/toast";

const { t } = useI18n();
const toast = useToastStore();
</script>

<template>
  <div aria-live="polite" class="pointer-events-none fixed inset-x-0 bottom-4 z-50 flex justify-center px-4">
    <div
      v-if="toast.message"
      class="pointer-events-auto flex items-center gap-3 rounded-md bg-gray-900 px-4 py-2 text-sm text-white shadow-lg dark:bg-gray-100 dark:text-gray-900"
      role="status"
      data-test="toast"
    >
      <span>{{ toast.message }}</span>
      <button type="button" class="text-xs underline" @click="toast.dismiss">{{ t("common.close") }}</button>
    </div>
  </div>
</template>
