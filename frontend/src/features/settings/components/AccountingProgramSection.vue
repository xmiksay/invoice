<script setup lang="ts">
import { useI18n } from "vue-i18n";
import type { FieldErrors } from "@/api/types";
import FormField from "@/components/form/FormField.vue";
import { reasonKey } from "@/lib/formErrors";
import { codeApplies, CODE_FIELDS, type ProgramDraft } from "../accounting";
import { ACCOUNTING_DIRECTIONS, type AccountingProgram } from "../types";

const draft = defineModel<ProgramDraft>({ required: true });
const props = defineProps<{ program: AccountingProgram; errors: FieldErrors; icoPlaceholder: string }>();

const { t } = useI18n();

// Row index into the model (= the `N` of the server's `<program>.codes.N.*` keys), grouped by direction.
const groups = ACCOUNTING_DIRECTIONS.map((direction) => ({
  direction,
  indexes: draft.value.rows.flatMap((r, i) => (r.direction === direction ? [i] : [])),
}));

const icoId = `${props.program}-ico`;
const cellId = (i: number, field: string) => `${props.program}-${i}-${field}`;
const cellError = (i: number, field: string) => props.errors[`${props.program}.codes.${i}.${field}`];
const rowError = (i: number) => cellError(i, "docType") ?? cellError(i, "direction");
const rowLabel = (i: number) => {
  const row = draft.value.rows[i]!;
  return `${t(`accounting.directions.${row.direction}`)} – ${t(`accounting.docTypes.${row.docType}`)}`;
};
</script>

<template>
  <section class="card space-y-4" :data-test="`${program}-section`">
    <div class="space-y-1">
      <h2 class="text-lg font-semibold">{{ t(`accounting.${program}.title`) }}</h2>
      <p class="text-sm text-gray-600 dark:text-gray-400">{{ t(`accounting.${program}.intro`) }}</p>
    </div>
    <FormField class="max-w-xs" :label="t(`accounting.${program}.ico`)" :for="icoId" :error="errors[`${program}.ico`]" :hint="t(`accounting.${program}.icoHint`)">
      <input
        :id="icoId"
        v-model="draft.ico"
        inputmode="numeric"
        class="input"
        :class="{ 'input-error': errors[`${program}.ico`] }"
        :placeholder="icoPlaceholder"
        :data-test="icoId"
      />
    </FormField>
    <div class="overflow-x-auto">
      <table class="table">
        <thead>
          <tr>
            <th>{{ t("accounting.document") }}</th>
            <th v-for="field in CODE_FIELDS" :key="field">{{ t(`accounting.columns.${field}`) }}</th>
          </tr>
        </thead>
        <tbody v-for="group in groups" :key="group.direction">
          <tr>
            <th :colspan="CODE_FIELDS.length + 1" scope="colgroup" class="bg-gray-50 text-left dark:bg-gray-900">{{ t(`accounting.directions.${group.direction}`) }}</th>
          </tr>
          <tr v-for="i in group.indexes" :key="i" :data-test="`${program}-row-${i}`">
            <th scope="row" class="text-left font-normal whitespace-nowrap">
              {{ t(`accounting.docTypes.${draft.rows[i]!.docType}`) }}
              <p v-if="rowError(i)" class="text-xs text-red-600 dark:text-red-400" data-test="field-error">
                {{ t(reasonKey(rowError(i)!)) }}
              </p>
            </th>
            <td v-for="field in CODE_FIELDS" :key="field" class="align-top">
              <span v-if="!codeApplies(field, draft.rows[i]!.direction)" class="text-gray-400" :data-test="`${cellId(i, field)}-na`">—</span>
              <input
                v-else
                :id="cellId(i, field)"
                v-model="draft.rows[i]![field]"
                class="input min-w-28"
                :class="{ 'input-error': cellError(i, field) }"
                :aria-label="`${rowLabel(i)}: ${t(`accounting.columns.${field}`)}`"
                :aria-invalid="cellError(i, field) ? 'true' : undefined"
                :aria-describedby="cellError(i, field) ? `${cellId(i, field)}-error` : undefined"
                :data-test="cellId(i, field)"
              />
              <p
                v-if="codeApplies(field, draft.rows[i]!.direction) && cellError(i, field)"
                :id="`${cellId(i, field)}-error`"
                class="mt-1 text-xs text-red-600 dark:text-red-400"
                data-test="field-error"
              >
                {{ t(reasonKey(cellError(i, field)!)) }}
              </p>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>
