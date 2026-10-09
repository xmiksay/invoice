<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { reasonKey } from "@/lib/formErrors";
import { SEPARATORS, addAddresses } from "../recipients";

/** Chip input for a list of addresses: Enter, comma, semicolon or leaving the field turns the text into chips. */
const model = defineModel<string[]>({ required: true });
const props = defineProps<{
  id: string;
  /** Reason per address index from the server (`to.0: invalid`). */
  invalid?: Record<number, string>;
  /** The list as a whole is rejected (`to: required`). */
  listError?: boolean;
}>();

const { t } = useI18n();
const draft = ref("");

function commit(): void {
  if (draft.value.trim()) model.value = addAddresses(model.value, draft.value);
  draft.value = "";
}

function onKeydown(e: KeyboardEvent): void {
  if (e.key === "Enter" || e.key === "," || e.key === ";") {
    e.preventDefault();
    commit();
  } else if (e.key === "Backspace" && draft.value === "" && model.value.length > 0) {
    model.value = model.value.slice(0, -1);
  }
}

/** A pasted list: every complete address becomes a chip, the unfinished tail stays in the field. */
function onInput(): void {
  const parts = draft.value.split(SEPARATORS);
  if (parts.length < 2) return;
  const tail = parts.pop() ?? "";
  model.value = addAddresses(model.value, parts.join(","));
  draft.value = tail.trimStart();
}

const remove = (index: number) => (model.value = model.value.filter((_, i) => i !== index));
const reasonAt = (index: number) => props.invalid?.[index];
</script>

<template>
  <div>
    <div
      class="input flex flex-wrap items-center gap-1 !py-1"
      :class="{ 'input-error': listError || Object.keys(invalid ?? {}).length > 0 }"
      :data-test="`recipients-${id}`"
    >
      <span
        v-for="(address, i) in model"
        :key="address"
        class="inline-flex items-center gap-1 rounded bg-gray-100 px-2 py-0.5 text-sm dark:bg-gray-800"
        :class="{ '!bg-red-100 text-red-800 dark:!bg-red-950 dark:text-red-300': reasonAt(i) }"
        data-test="chip"
      >
        {{ address }}
        <button
          type="button"
          class="text-gray-500 hover:text-gray-900 dark:hover:text-gray-100"
          :aria-label="t('email.send.removeAddress', { address })"
          data-test="chip-remove"
          @click="remove(i)"
        >
          ×
        </button>
      </span>
      <input
        :id="id"
        v-model="draft"
        type="text"
        inputmode="email"
        autocomplete="email"
        class="min-w-40 flex-1 bg-transparent py-1 text-sm outline-none"
        :aria-invalid="listError || undefined"
        @keydown="onKeydown"
        @input="onInput"
        @blur="commit"
      />
    </div>
    <template v-for="(address, i) in model" :key="`err-${address}`">
      <p v-if="reasonAt(i)" class="text-xs text-red-600 dark:text-red-400" data-test="chip-error">
        {{ t("email.send.addressError", { address, reason: t(reasonKey(reasonAt(i)!)) }) }}
      </p>
    </template>
  </div>
</template>
