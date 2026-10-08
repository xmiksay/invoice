<script setup lang="ts">
import { useI18n } from "vue-i18n";
import FormField from "@/components/form/FormField.vue";
import { reasonKey } from "@/lib/formErrors";
import type { CustomFieldValues } from "@/features/documents/types";
import type { CustomField } from "@/features/settings/types";

/** Inputs for the applicable custom fields; `errors` are keyed by field key. */
const values = defineModel<CustomFieldValues>({ required: true });
defineProps<{ defs: CustomField[]; errors: Record<string, string>; idPrefix: string }>();

const { t } = useI18n();

const text = (key: string) => {
  const v = values.value[key];
  return typeof v === "string" ? v : "";
};
function set(key: string, value: string | boolean) {
  values.value = { ...values.value, [key]: value };
}
const label = (def: CustomField) => (def.required ? `${def.label} *` : def.label);
</script>

<template>
  <template v-for="def in defs" :key="def.key">
    <div v-if="def.type === 'bool'" class="flex flex-col justify-end space-y-1 pb-2" :data-test="`custom-field-${def.key}`">
      <label class="flex items-center gap-2 text-sm">
        <input :id="`${idPrefix}-cf-${def.key}`" type="checkbox" class="size-4 rounded" :checked="values[def.key] === true" @change="set(def.key, ($event.target as HTMLInputElement).checked)" />
        {{ def.label }}
      </label>
      <p v-if="errors[def.key]" class="text-xs text-red-600 dark:text-red-400" data-test="field-error">{{ t(reasonKey(errors[def.key] ?? "")) }}</p>
    </div>
    <FormField v-else :label="label(def)" :for="`${idPrefix}-cf-${def.key}`" :error="errors[def.key]" :data-test="`custom-field-${def.key}`">
      <select
        v-if="def.type === 'select'"
        :id="`${idPrefix}-cf-${def.key}`"
        :value="text(def.key)"
        class="input"
        :class="{ 'input-error': errors[def.key] }"
        @change="set(def.key, ($event.target as HTMLSelectElement).value)"
      >
        <option value="">{{ t("common.notSet") }}</option>
        <option v-for="o in def.options" :key="o" :value="o">{{ o }}</option>
        <option v-if="text(def.key) && !def.options.includes(text(def.key))" :value="text(def.key)">{{ text(def.key) }}</option>
      </select>
      <input
        v-else
        :id="`${idPrefix}-cf-${def.key}`"
        :value="text(def.key)"
        :type="def.type === 'date' ? 'date' : 'text'"
        :inputmode="def.type === 'number' ? 'decimal' : undefined"
        :maxlength="def.type === 'text' ? 500 : undefined"
        class="input"
        :class="{ 'input-error': errors[def.key] }"
        autocomplete="off"
        @input="set(def.key, ($event.target as HTMLInputElement).value)"
      />
    </FormField>
  </template>
</template>
