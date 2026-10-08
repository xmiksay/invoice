<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import FormField from "@/components/form/FormField.vue";
import { useFormSubmit } from "@/composables/useFormSubmit";
import { useVatRatesStore } from "../stores";
import type { VatRate } from "../types";
import { normalizeRate, validateVatRate } from "../vatRate";

const props = defineProps<{ vatRate?: VatRate; nextPosition: number }>();
const emit = defineEmits<{ done: [] }>();

const { t } = useI18n();
const store = useVatRatesStore();
const { fieldErrors, error, submitting, submit } = useFormSubmit();

const draft = ref({
  rate: props.vatRate?.rate ?? "",
  label: props.vatRate?.label ?? "",
  position: props.vatRate?.position ?? props.nextPosition,
  isDefault: props.vatRate?.isDefault ?? false,
  active: props.vatRate?.active ?? true,
});

async function onSubmit() {
  const d = draft.value;
  const ok = await submit(
    () => validateVatRate(d),
    () =>
      store.save(
        { ...d, rate: normalizeRate(d.rate), label: d.label.trim(), position: Number(d.position) || 0 },
        props.vatRate?.id,
      ),
  );
  if (ok) emit("done");
}
</script>

<template>
  <form class="card space-y-4" novalidate @submit.prevent="onSubmit">
    <h2 class="font-semibold">{{ vatRate ? t("settings.vatRates.edit") : t("settings.vatRates.add") }}</h2>
    <div class="grid gap-4 sm:grid-cols-3">
      <FormField :label="t('settings.vatRates.rate')" for="vat-rate" :error="fieldErrors.rate">
        <input id="vat-rate" v-model="draft.rate" inputmode="decimal" class="input" :class="{ 'input-error': fieldErrors.rate }" />
      </FormField>
      <FormField :label="t('settings.vatRates.label')" for="vat-label" :error="fieldErrors.label">
        <input id="vat-label" v-model="draft.label" maxlength="100" class="input" :class="{ 'input-error': fieldErrors.label }" />
      </FormField>
      <FormField :label="t('settings.vatRates.position')" for="vat-position" :error="fieldErrors.position">
        <input id="vat-position" v-model.number="draft.position" type="number" min="0" class="input" />
      </FormField>
    </div>
    <div class="flex flex-wrap gap-4 text-sm">
      <label class="flex items-center gap-2">
        <input v-model="draft.isDefault" type="checkbox" class="size-4 rounded" />
        {{ t("settings.vatRates.isDefault") }}
      </label>
      <label class="flex items-center gap-2">
        <input v-model="draft.active" type="checkbox" class="size-4 rounded" />
        {{ t("settings.vatRates.active") }}
      </label>
    </div>
    <p v-if="fieldErrors.isDefault" class="text-xs text-red-600 dark:text-red-400" data-test="field-error">
      {{ t("settings.vatRates.defaultMustBeActive") }}
    </p>
    <p v-if="error" role="alert" class="alert-error">{{ error }}</p>
    <div class="flex gap-2">
      <button type="submit" class="btn btn-primary" :disabled="submitting">
        {{ submitting ? t("common.saving") : t("common.save") }}
      </button>
      <button type="button" class="btn" @click="emit('done')">{{ t("common.cancel") }}</button>
    </div>
  </form>
</template>
