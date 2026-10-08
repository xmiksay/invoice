<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import type { FieldErrors } from "@/api/types";
import type { CatalogGroup, CatalogItem } from "@/features/catalog/types";
import { insertCatalogGroup, insertCatalogItem, type CatalogInsert } from "../catalogLines";
import type { VatMode } from "../types";
import { defaultSubtotalRefs, moveLine, newItemLine, newSubtotalLine, newTextLine, refTargets, removeLine, type LineDraft, type RefOption } from "../lines";
import AdvanceLineRow from "./AdvanceLineRow.vue";
import CatalogPicker from "./CatalogPicker.vue";
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
  /** Document currency; catalog prices in another currency are not converted. */
  currency: string;
  vatMode: VatMode;
  /** The draft's `relatedDocumentId` (the proforma a final invoice settles). */
  relatedDocumentId?: string | null;
}>();

const { t } = useI18n();
/** Catalog items just inserted without a price (other currency). */
const unpriced = ref<string[]>([]);

function add(kind: "item" | "text" | "subtotal") {
  const line = kind === "item" ? newItemLine(props.defaultVatRate) : kind === "text" ? newTextLine() : newSubtotalLine(defaultSubtotalRefs(lines.value));
  lines.value = [...lines.value, line];
}

function append(insert: CatalogInsert) {
  lines.value = [...lines.value, ...insert.lines];
  unpriced.value = insert.unpriced;
}

const target = () => ({ currency: props.currency, vatLocked: props.vatLocked });
const onCatalogItem = (item: CatalogItem) => append(insertCatalogItem(item, target()));
const onCatalogGroup = (group: CatalogGroup) => append(insertCatalogGroup(group, lines.value.length, target()));

function update(index: number, line: LineDraft) {
  lines.value = lines.value.map((l, i) => (i === index ? line : l));
}

function refOptions(index: number): RefOption[] {
  return refTargets(lines.value, index).map((position) => {
    const l = lines.value[position - 1] as LineDraft;
    return { position, label: l.description || t(`documents.line.kinds.${l.kind}`) };
  });
}
</script>

<template>
  <div class="space-y-3">
    <ol v-if="lines.length" class="space-y-3">
      <template v-for="(line, i) in lines" :key="line.key">
        <AdvanceLineRow
          v-if="line.kind === 'advance'"
          :line="line"
          :index="i"
          :currency="currency"
          :errors="errors[i] ?? {}"
          :base="bases[i] ?? null"
          :vat-mode="vatMode"
          :related-document-id="relatedDocumentId ?? null"
          @remove="lines = removeLine(lines, i)"
        />
        <LineRow
          v-else
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
      </template>
    </ol>
    <p v-else class="text-sm text-gray-500">{{ t("documents.line.empty") }}</p>

    <p v-if="unpriced.length" class="rounded-md bg-amber-50 px-3 py-2 text-sm text-amber-800 dark:bg-amber-950/50 dark:text-amber-300" data-test="catalog-unpriced">
      {{ t("documents.catalog.unpriced", { names: unpriced.join(", "), currency }) }}
    </p>

    <div class="flex flex-wrap gap-2">
      <button type="button" class="btn btn-sm" data-test="add-item" @click="add('item')">+ {{ t("documents.line.addItem") }}</button>
      <button type="button" class="btn btn-sm" data-test="add-text" @click="add('text')">+ {{ t("documents.line.addText") }}</button>
      <button type="button" class="btn btn-sm" data-test="add-subtotal" @click="add('subtotal')">+ {{ t("documents.line.addSubtotal") }}</button>
      <CatalogPicker :currency="currency" @item="onCatalogItem" @group="onCatalogGroup" />
    </div>
  </div>
</template>
