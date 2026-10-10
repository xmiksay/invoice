<script setup lang="ts">
import { useI18n } from "vue-i18n";
import type { FieldErrors } from "@/api/types";
import { reasonKey } from "@/lib/formErrors";
import { codeApplies, POHODA_CODE_FIELDS, type PohodaRowDraft } from "../accounting";
import { ACCOUNTING_DIRECTIONS } from "../types";

const rows = defineModel<PohodaRowDraft[]>({ required: true });
defineProps<{ errors: FieldErrors }>();

const { t } = useI18n();

// Row index into the model (= the `N` of the server's `pohoda.codes.N.*` keys), grouped by direction.
const groups = ACCOUNTING_DIRECTIONS.map((direction) => ({
  direction,
  indexes: rows.value.flatMap((r, i) => (r.direction === direction ? [i] : [])),
}));

const cellId = (i: number, field: string) => `pohoda-${i}-${field}`;
const rowLabel = (row: PohodaRowDraft) =>
  `${t(`accounting.directions.${row.direction}`)} – ${t(`accounting.docTypes.${row.docType}`)}`;
const rowError = (i: number, errors: FieldErrors) => errors[`pohoda.codes.${i}.docType`] ?? errors[`pohoda.codes.${i}.direction`];
</script>

<template>
  <div class="overflow-x-auto">
    <table class="table">
      <thead>
        <tr>
          <th>{{ t("accounting.pohoda.document") }}</th>
          <th v-for="field in POHODA_CODE_FIELDS" :key="field">{{ t(`accounting.pohoda.columns.${field}`) }}</th>
        </tr>
      </thead>
      <tbody v-for="group in groups" :key="group.direction">
        <tr>
          <th :colspan="POHODA_CODE_FIELDS.length + 1" scope="colgroup" class="bg-gray-50 text-left dark:bg-gray-900">{{ t(`accounting.directions.${group.direction}`) }}</th>
        </tr>
        <tr v-for="i in group.indexes" :key="i" :data-test="`pohoda-row-${i}`">
          <th scope="row" class="text-left font-normal whitespace-nowrap">
            {{ t(`accounting.docTypes.${rows[i]!.docType}`) }}
            <p v-if="rowError(i, errors)" class="text-xs text-red-600 dark:text-red-400" data-test="field-error">
              {{ t(reasonKey(rowError(i, errors)!)) }}
            </p>
          </th>
          <td v-for="field in POHODA_CODE_FIELDS" :key="field" class="align-top">
            <span v-if="!codeApplies(field, rows[i]!.direction)" class="text-gray-400" :data-test="`${cellId(i, field)}-na`">—</span>
            <input
              v-else
              :id="cellId(i, field)"
              v-model="rows[i]![field]"
              class="input min-w-28"
              :class="{ 'input-error': errors[`pohoda.codes.${i}.${field}`] }"
              :aria-label="`${rowLabel(rows[i]!)}: ${t(`accounting.pohoda.columns.${field}`)}`"
              :aria-invalid="errors[`pohoda.codes.${i}.${field}`] ? 'true' : undefined"
              :aria-describedby="errors[`pohoda.codes.${i}.${field}`] ? `${cellId(i, field)}-error` : undefined"
              :data-test="cellId(i, field)"
            />
            <p
              v-if="codeApplies(field, rows[i]!.direction) && errors[`pohoda.codes.${i}.${field}`]"
              :id="`${cellId(i, field)}-error`"
              class="mt-1 text-xs text-red-600 dark:text-red-400"
              data-test="field-error"
            >
              {{ t(reasonKey(errors[`pohoda.codes.${i}.${field}`]!)) }}
            </p>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
