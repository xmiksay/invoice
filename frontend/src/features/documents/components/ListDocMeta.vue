<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import type { Category } from "@/features/settings/types";
import type { DocumentSummary } from "../types";

/** Second line of a list row: category and the "no PDF" hint for received / imported documents. */
const props = defineProps<{ doc: DocumentSummary; categories: Category[] }>();
const { t } = useI18n();

const category = computed(() => props.categories.find((c) => c.id === props.doc.categoryId)?.name ?? null);
const withoutPdf = computed(() => (props.doc.direction === "received" || props.doc.imported) && props.doc.hasPdf === false);
</script>

<template>
  <div v-if="category || withoutPdf" class="mt-1 flex flex-wrap gap-2 text-xs text-gray-500 dark:text-gray-400">
    <span v-if="category" data-test="row-category">{{ category }}</span>
    <span v-if="withoutPdf" class="text-amber-700 dark:text-amber-400" data-test="no-pdf">{{ t("documents.list.noPdf") }}</span>
  </div>
</template>
