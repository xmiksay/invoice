<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { detailLocation } from "@/features/documents/routes";
import { RESULT_CLASS } from "../badges";
import type { ConfirmResult, PreviewEntry } from "../types";

/** Summary + per-entry outcome of the confirm, linking to each imported document. */
const props = defineProps<{
  results: ConfirmResult[];
  /** The preview the results belong to (number and direction of each entry). */
  entries: PreviewEntry[];
  /** Translated text of a per-entry error code. */
  errorText: (code: string) => string;
  /** How the entry key is shown (default: as is). */
  keyLabel?: (key: string) => string;
}>();

const { t } = useI18n();

const entryOf = (r: ConfirmResult) => props.entries.find((e) => e.key === r.key) ?? null;
const label = (key: string) => props.keyLabel?.(key) ?? key;
const imported = computed(() => props.results.filter((r) => r.status === "imported").length);
</script>

<template>
  <p class="text-sm" data-test="import-summary">{{ t("imports.result.summary", { imported, total: results.length }) }}</p>
  <div class="card overflow-x-auto !p-0">
    <table class="table" data-test="import-results">
      <thead>
        <tr>
          <th>{{ t("imports.preview.status") }}</th>
          <th>{{ t("imports.preview.document") }}</th>
          <th>{{ t("imports.result.outcome") }}</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="r in results" :key="r.key" data-test="import-result-row">
          <td class="align-top">
            <span class="badge" :class="RESULT_CLASS[r.status]">{{ t(`imports.result.status.${r.status}`) }}</span>
          </td>
          <td class="align-top">
            <div class="font-mono font-medium">{{ entryOf(r)?.number ?? "—" }}</div>
            <div class="text-xs break-all text-gray-500">{{ label(r.key) }}</div>
          </td>
          <td class="align-top">
            <RouterLink
              v-if="r.documentId"
              :to="detailLocation(r.documentId, entryOf(r)?.direction ?? 'issued')"
              class="text-blue-600 hover:underline dark:text-blue-400"
              data-test="import-result-link"
            >
              {{ t("imports.result.open") }}
            </RouterLink>
            <span v-else-if="r.error" class="text-red-700 dark:text-red-400" data-test="import-result-error">{{ errorText(r.error) }}</span>
            <span v-else class="text-gray-500">{{ t("imports.result.notSelected") }}</span>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
