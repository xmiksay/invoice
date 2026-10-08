<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import type { FieldErrors } from "@/api/types";
import FormField from "@/components/form/FormField.vue";
import type { AdvanceLineDraft, LineDraft, RefOption } from "../lines";

type EditableLine = Exclude<LineDraft, AdvanceLineDraft>;

const props = defineProps<{
  line: EditableLine;
  index: number;
  count: number;
  vatOptions: string[];
  vatLocked: boolean;
  errors: FieldErrors;
  /** Formatted computed base, when known. */
  base: string | null;
  refOptions: RefOption[];
}>();
const emit = defineEmits<{ update: [line: EditableLine]; move: [delta: -1 | 1]; remove: [] }>();

const { t } = useI18n();

const position = computed(() => props.index + 1);
const id = (field: string) => `line-${props.index}-${field}`;
const cls = (field: string) => ({ "input-error": props.errors[field] });

function set(patch: Record<string, unknown>) {
  emit("update", { ...props.line, ...patch } as EditableLine);
}
const value = (e: Event) => (e.target as HTMLInputElement).value;

function toggleRef(pos: number, on: boolean) {
  if (props.line.kind !== "subtotal") return;
  const refs = on ? [...props.line.refs, pos] : props.line.refs.filter((r) => r !== pos);
  set({ refs: [...new Set(refs)].sort((a, b) => a - b) });
}

const vatChoices = computed(() => {
  const current = props.line.kind === "item" ? props.line.vatRate : "";
  return current && !props.vatOptions.includes(current) ? [...props.vatOptions, current] : props.vatOptions;
});
</script>

<template>
  <li class="space-y-3 rounded-md border border-gray-200 p-3 dark:border-gray-700" :class="{ 'bg-gray-50 dark:bg-gray-800/40': line.kind === 'subtotal' }" :data-test="`line-${index}`">
    <div class="flex items-center gap-2">
      <span class="font-mono text-sm font-semibold" data-test="line-position">{{ position }}.</span>
      <span class="badge !bg-gray-200 !text-gray-700 dark:!bg-gray-700 dark:!text-gray-200">{{ t(`documents.line.kinds.${line.kind}`) }}</span>
      <span v-if="base !== null" class="ml-auto text-sm tabular-nums" data-test="line-base">{{ base }}</span>
      <div class="flex gap-1" :class="{ 'ml-auto': base === null }">
        <button type="button" class="btn btn-sm" :disabled="index === 0" :aria-label="t('documents.line.moveUp', { n: position })" data-test="move-up" @click="emit('move', -1)">↑</button>
        <button type="button" class="btn btn-sm" :disabled="index === count - 1" :aria-label="t('documents.line.moveDown', { n: position })" data-test="move-down" @click="emit('move', 1)">↓</button>
        <button type="button" class="btn btn-sm btn-danger" :aria-label="t('documents.line.remove', { n: position })" data-test="remove-line" @click="emit('remove')">✕</button>
      </div>
    </div>

    <FormField :label="t('documents.line.description')" :for="id('description')" :error="errors.description">
      <textarea
        v-if="line.kind === 'text'"
        :id="id('description')"
        :value="line.description"
        rows="2"
        maxlength="500"
        class="input"
        :class="cls('description')"
        @input="set({ description: value($event) })"
      />
      <input v-else :id="id('description')" :value="line.description" maxlength="500" class="input" :class="cls('description')" @input="set({ description: value($event) })" />
    </FormField>

    <div v-if="line.kind === 'item'" class="grid grid-cols-2 gap-3 sm:grid-cols-5">
      <FormField :label="t('documents.line.quantity')" :for="id('quantity')" :error="errors.quantity">
        <input :id="id('quantity')" :value="line.quantity" inputmode="decimal" class="input" :class="cls('quantity')" @input="set({ quantity: value($event) })" />
      </FormField>
      <FormField :label="t('documents.line.unit')" :for="id('unit')" :error="errors.unit">
        <input :id="id('unit')" :value="line.unit ?? ''" maxlength="20" class="input" :class="cls('unit')" @input="set({ unit: value($event) })" />
      </FormField>
      <FormField :label="t('documents.line.unitPrice')" :for="id('unitPrice')" :error="errors.unitPrice">
        <input :id="id('unitPrice')" :value="line.unitPrice" inputmode="decimal" class="input" :class="cls('unitPrice')" @input="set({ unitPrice: value($event) })" />
      </FormField>
      <FormField :label="t('documents.line.discountPct')" :for="id('discountPct')" :error="errors.discountPct">
        <input :id="id('discountPct')" :value="line.discountPct" inputmode="decimal" class="input" :class="cls('discountPct')" @input="set({ discountPct: value($event) })" />
      </FormField>
      <FormField :label="t('documents.line.vatRate')" :for="id('vatRate')" :error="errors.vatRate">
        <select :id="id('vatRate')" :value="line.vatRate" class="input" :class="cls('vatRate')" :disabled="vatLocked" @change="set({ vatRate: value($event) })">
          <option v-for="r in vatChoices" :key="r" :value="r">{{ r }} %</option>
        </select>
      </FormField>
    </div>

    <div v-else-if="line.kind === 'subtotal'" class="space-y-2">
      <fieldset>
        <legend class="text-sm font-medium">{{ t("documents.line.refs") }}</legend>
        <p v-if="refOptions.length === 0" class="text-xs text-gray-500">{{ t("documents.line.noRefOptions") }}</p>
        <div class="mt-1 flex flex-wrap gap-2">
          <label
            v-for="opt in refOptions"
            :key="opt.position"
            class="flex cursor-pointer items-center gap-1 rounded-md border border-gray-300 px-2 py-1 text-sm dark:border-gray-700"
            :title="opt.label"
          >
            <input
              type="checkbox"
              class="size-4"
              :checked="line.refs.includes(opt.position)"
              :data-test="`ref-${opt.position}`"
              @change="toggleRef(opt.position, ($event.target as HTMLInputElement).checked)"
            />
            {{ opt.position }}
          </label>
        </div>
        <p v-if="errors.refs" :id="`${id('refs')}-error`" class="mt-1 text-xs text-red-600 dark:text-red-400" data-test="field-error">
          {{ t("documents.line.refsInvalid") }}
        </p>
      </fieldset>
      <label class="flex items-center gap-2 text-sm">
        <input type="checkbox" class="size-4" :checked="line.collapse" @change="set({ collapse: ($event.target as HTMLInputElement).checked })" />
        {{ t("documents.line.collapse") }}
      </label>
    </div>
  </li>
</template>
