<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import type { FieldErrors } from "@/api/types";
import FormField from "@/components/form/FormField.vue";
import IndicativeRateNote from "@/features/documents/components/IndicativeRateNote.vue";
import { VAT_MODES, type ExchangeRate } from "@/features/documents/types";
import { hasDueDate, hasTaxPoint, type ReceivedDraft } from "../form";

const model = defineModel<ReceivedDraft>({ required: true });
const props = defineProps<{
  errors: FieldErrors;
  /** Indicative ČNB rate for the received date; the server fixes the real one on save. */
  indicativeRate: ExchangeRate | null;
  indicativeError: unknown;
  indicativeLoading: boolean;
}>();

const { t } = useI18n();
const isCzk = computed(() => model.value.currency === "CZK");
const cls = (field: string) => ({ "input-error": props.errors[field] });

function onCurrency(event: Event) {
  const code = (event.target as HTMLInputElement).value.trim().toUpperCase();
  // A manual rate for one currency is meaningless for another.
  model.value = { ...model.value, currency: code, exchangeRate: code === model.value.currency ? model.value.exchangeRate : "" };
}

function onReceivedDate(event: Event) {
  model.value = { ...model.value, receivedDate: (event.target as HTMLInputElement).value, receivedDateAuto: false };
}
</script>

<template>
  <div class="grid gap-4 sm:grid-cols-3">
    <FormField :label="t('received.fields.supplierNumber')" for="rec-supplierNumber" :error="errors.supplierNumber" :hint="t('received.fields.supplierNumberHint')">
      <input id="rec-supplierNumber" v-model="model.supplierNumber" maxlength="40" class="input" :class="cls('supplierNumber')" autocomplete="off" />
    </FormField>
    <FormField :label="t('documents.fields.variableSymbol')" for="rec-variableSymbol" :error="errors.variableSymbol">
      <input id="rec-variableSymbol" v-model="model.variableSymbol" inputmode="numeric" maxlength="10" class="input" :class="cls('variableSymbol')" />
    </FormField>
    <FormField :label="t('documents.fields.constantSymbol')" for="rec-constantSymbol" :error="errors.constantSymbol">
      <input id="rec-constantSymbol" v-model="model.constantSymbol" inputmode="numeric" maxlength="4" class="input" :class="cls('constantSymbol')" />
    </FormField>

    <FormField :label="t('documents.fields.issueDate')" for="rec-issueDate" :error="errors.issueDate">
      <input id="rec-issueDate" v-model="model.issueDate" type="date" class="input" :class="cls('issueDate')" />
    </FormField>
    <FormField v-if="hasTaxPoint(model.docType)" :label="t('documents.fields.taxPointDate')" for="rec-taxPointDate" :error="errors.taxPointDate">
      <input id="rec-taxPointDate" v-model="model.taxPointDate" type="date" class="input" :class="cls('taxPointDate')" />
    </FormField>
    <FormField
      :label="t('received.fields.receivedDate')"
      for="rec-receivedDate"
      :error="errors.receivedDate"
      :hint="model.receivedDateAuto ? t('received.fields.receivedDateAuto') : undefined"
    >
      <input id="rec-receivedDate" :value="model.receivedDate" type="date" class="input" :class="cls('receivedDate')" @input="onReceivedDate" />
    </FormField>
    <FormField v-if="hasDueDate(model.docType)" :label="t('documents.fields.dueDate')" for="rec-dueDate" :error="errors.dueDate">
      <input id="rec-dueDate" v-model="model.dueDate" type="date" class="input" :class="cls('dueDate')" />
    </FormField>

    <FormField :label="t('documents.fields.currency')" for="rec-currency" :error="errors.currency">
      <input id="rec-currency" :value="model.currency" maxlength="3" class="input uppercase" :class="cls('currency')" autocomplete="off" @change="onCurrency" />
    </FormField>
    <div v-if="!isCzk" class="space-y-1 sm:col-span-2">
      <FormField :label="t('documents.editor.manualRate')" for="rec-exchangeRate" :error="errors.exchangeRate" :hint="t('received.fields.manualRateHint')">
        <input id="rec-exchangeRate" v-model="model.exchangeRate" inputmode="decimal" class="input" :class="cls('exchangeRate')" autocomplete="off" />
      </FormField>
      <IndicativeRateNote
        :rate="indicativeRate"
        :error="indicativeError"
        :loading="indicativeLoading"
        :currency="model.currency"
        message-key="received.fields.indicativeRate"
      />
    </div>

    <FormField :label="t('documents.fields.vatMode')" for="rec-vatMode" :error="errors.vatMode">
      <select id="rec-vatMode" v-model="model.vatMode" class="input">
        <option v-for="m in VAT_MODES" :key="m" :value="m">{{ m === "non_payer" ? t("received.supplierNonPayer") : t(`documents.vatMode.${m}`) }}</option>
      </select>
    </FormField>
    <div class="flex flex-col justify-end pb-2">
      <label class="flex items-center gap-2 text-sm">
        <input id="rec-vatDeductible" v-model="model.vatDeductible" type="checkbox" class="size-4 rounded" />
        {{ t("received.fields.vatDeductible") }}
      </label>
    </div>
    <FormField :label="t('received.fields.supplierAccount')" for="rec-supplierAccount" :error="errors.supplierAccount">
      <input id="rec-supplierAccount" v-model="model.supplierAccount" maxlength="60" class="input" :class="cls('supplierAccount')" autocomplete="off" />
    </FormField>
  </div>
</template>
