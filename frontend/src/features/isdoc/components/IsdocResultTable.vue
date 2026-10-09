<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { detailLocation } from "@/features/documents/routes";
import { RESULT_CLASS } from "../badges";
import { useIsdocImportStore } from "../store";
import type { ConfirmResult } from "../types";
import { entryErrorKey } from "../upload";

/** Per-entry outcome of the confirm, linking to each imported document. */
const { t } = useI18n();
const store = useIsdocImportStore();

/** The previewed entry the result belongs to (its number and direction). */
const entryOf = (r: ConfirmResult) => store.entries?.find((e) => e.key === r.key) ?? null;
</script>

<template>
  <div class="card overflow-x-auto !p-0">
    <table class="table" data-test="isdoc-results">
      <thead>
        <tr>
          <th>{{ t("isdoc.preview.status") }}</th>
          <th>{{ t("isdoc.preview.document") }}</th>
          <th>{{ t("isdoc.result.outcome") }}</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="r in store.results ?? []" :key="r.key" data-test="isdoc-result-row">
          <td class="align-top">
            <span class="badge" :class="RESULT_CLASS[r.status]">{{ t(`isdoc.result.status.${r.status}`) }}</span>
          </td>
          <td class="align-top">
            <div class="font-mono font-medium">{{ entryOf(r)?.number ?? "—" }}</div>
            <div class="text-xs break-all text-gray-500">{{ r.key }}</div>
          </td>
          <td class="align-top">
            <RouterLink
              v-if="r.documentId"
              :to="detailLocation(r.documentId, entryOf(r)?.direction ?? 'issued')"
              class="text-blue-600 hover:underline dark:text-blue-400"
              data-test="isdoc-result-link"
            >
              {{ t("isdoc.result.open") }}
            </RouterLink>
            <span v-else-if="r.error" class="text-red-700 dark:text-red-400" data-test="isdoc-result-error">
              {{ t(entryErrorKey(r.error), { code: r.error, number: "" }) }}
            </span>
            <span v-else class="text-gray-500">{{ t("isdoc.result.notSelected") }}</span>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
