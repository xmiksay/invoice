<script setup lang="ts">
import { useI18n } from "vue-i18n";
import type { FieldErrors } from "@/api/types";
import { defaultSubtotalRefs, moveLine, newItemLine, newSubtotalLine, newTextLine, removeLine, type LineDraft, type RefOption } from "../lines";
import LineRow from "./LineRow.vue";

const lines = defineModel<LineDraft[]>({ required: true });
const props = defineProps<{
  vatOptions: string[];
  /** Rate for newly added items. */
  defaultVatRate: string;
  vatLocked: boolean;
  errors: Record<number, FieldErrors>;
  /** Formatted base per line index, from the live compute. */
  bases: (string | null)[];
}>();

const { t } = useI18n();

function add(kind: LineDraft["kind"]) {
  const line = kind === "item" ? newItemLine(props.defaultVatRate) : kind === "text" ? newTextLine() : newSubtotalLine(defaultSubtotalRefs(lines.value));
  lines.value = [...lines.value, line];
}

function update(index: number, line: LineDraft) {
  lines.value = lines.value.map((l, i) => (i === index ? line : l));
}

/** Subtotals may reference items and other subtotals, never themselves or text lines. */
function refOptions(index: number): RefOption[] {
  return lines.value.flatMap((l, i) =>
    i !== index && l.kind !== "text" ? [{ position: i + 1, label: l.description || t(`documents.line.kinds.${l.kind}`) }] : [],
  );
}
</script>

<template>
  <div class="space-y-3">
    <ol v-if="lines.length" class="space-y-3">
      <LineRow
        v-for="(line, i) in lines"
        :key="line.key"
        :line="line"
        :index="i"
        :count="lines.length"
        :vat-options="vatOptions"
        :vat-locked="vatLocked"
        :errors="errors[i] ?? {}"
        :base="bases[i] ?? null"
        :ref-options="refOptions(i)"
        @update="update(i, $event)"
        @move="lines = moveLine(lines, i, i + $event)"
        @remove="lines = removeLine(lines, i)"
      />
    </ol>
    <p v-else class="text-sm text-gray-500">{{ t("documents.line.empty") }}</p>

    <div class="flex flex-wrap gap-2">
      <button type="button" class="btn btn-sm" data-test="add-item" @click="add('item')">+ {{ t("documents.line.addItem") }}</button>
      <button type="button" class="btn btn-sm" data-test="add-text" @click="add('text')">+ {{ t("documents.line.addText") }}</button>
      <button type="button" class="btn btn-sm" data-test="add-subtotal" @click="add('subtotal')">+ {{ t("documents.line.addSubtotal") }}</button>
    </div>
  </div>
</template>
