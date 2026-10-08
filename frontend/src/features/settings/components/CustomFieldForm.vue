<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import FormField from "@/components/form/FormField.vue";
import { useFormSubmit } from "@/composables/useFormSubmit";
import { toCustomFieldDraft, toCustomFieldInput, validateCustomField } from "../customField";
import { useCustomFieldsStore } from "../stores";
import { CUSTOM_FIELD_SCOPES, CUSTOM_FIELD_TYPES, type CustomField } from "../types";

const props = defineProps<{ field?: CustomField; nextPosition: number }>();
const emit = defineEmits<{ done: [] }>();

const { t } = useI18n();
const store = useCustomFieldsStore();
const { fieldErrors, error, submitting, submit } = useFormSubmit();
const draft = ref(toCustomFieldDraft(props.field, props.nextPosition));
/** Key and type are immutable once created (stored values depend on them). */
const existing = !!props.field;

async function onSubmit() {
  const ok = await submit(
    () => validateCustomField(draft.value),
    () => store.save(toCustomFieldInput(draft.value), props.field?.id),
  );
  if (ok) emit("done");
}
</script>

<template>
  <form class="card space-y-4" novalidate data-test="custom-field-form" @submit.prevent="onSubmit">
    <h2 class="font-semibold">{{ field ? t("settings.customFields.edit") : t("settings.customFields.add") }}</h2>
    <div class="grid gap-4 sm:grid-cols-3">
      <FormField :label="t('settings.customFields.key')" for="cf-key" :error="fieldErrors.key" :hint="existing ? t('settings.customFields.immutable') : t('settings.customFields.keyHint')">
        <input id="cf-key" v-model="draft.key" maxlength="40" class="input font-mono" :class="{ 'input-error': fieldErrors.key }" :disabled="existing" autocomplete="off" />
      </FormField>
      <FormField :label="t('settings.customFields.label')" for="cf-label" :error="fieldErrors.label">
        <input id="cf-label" v-model="draft.label" maxlength="100" class="input" :class="{ 'input-error': fieldErrors.label }" />
      </FormField>
      <FormField :label="t('settings.customFields.type')" for="cf-type" :error="fieldErrors.type" :hint="existing ? t('settings.customFields.immutable') : undefined">
        <select id="cf-type" v-model="draft.type" class="input" :disabled="existing">
          <option v-for="ty in CUSTOM_FIELD_TYPES" :key="ty" :value="ty">{{ t(`settings.customFields.types.${ty}`) }}</option>
        </select>
      </FormField>
      <FormField v-if="draft.type === 'select'" class="sm:col-span-3" :label="t('settings.customFields.options')" for="cf-options" :error="fieldErrors.options" :hint="t('settings.customFields.optionsHint')">
        <textarea id="cf-options" v-model="draft.options" rows="4" class="input" :class="{ 'input-error': fieldErrors.options }" />
      </FormField>
      <FormField :label="t('settings.customFields.appliesTo')" for="cf-appliesTo" :error="fieldErrors.appliesTo">
        <select id="cf-appliesTo" v-model="draft.appliesTo" class="input">
          <option v-for="s in CUSTOM_FIELD_SCOPES" :key="s" :value="s">{{ t(`settings.customFields.scopes.${s}`) }}</option>
        </select>
      </FormField>
      <FormField :label="t('settings.vatRates.position')" for="cf-position" :error="fieldErrors.position">
        <input id="cf-position" v-model.number="draft.position" type="number" min="0" class="input" />
      </FormField>
    </div>
    <div class="flex flex-wrap gap-4 text-sm">
      <label class="flex items-center gap-2">
        <input id="cf-required" v-model="draft.required" type="checkbox" class="size-4 rounded" />
        {{ t("settings.customFields.required") }}
      </label>
      <label class="flex items-center gap-2">
        <input id="cf-active" v-model="draft.active" type="checkbox" class="size-4 rounded" />
        {{ t("settings.vatRates.active") }}
      </label>
    </div>
    <p v-if="error" role="alert" class="alert-error">{{ error }}</p>
    <div class="flex gap-2">
      <button type="submit" class="btn btn-primary" :disabled="submitting">{{ submitting ? t("common.saving") : t("common.save") }}</button>
      <button type="button" class="btn" @click="emit('done')">{{ t("common.cancel") }}</button>
    </div>
  </form>
</template>
