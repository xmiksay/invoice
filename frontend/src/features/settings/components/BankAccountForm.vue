<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import FormField from "@/components/form/FormField.vue";
import { useFormSubmit } from "@/composables/useFormSubmit";
import { collectErrors, nullIfEmpty } from "@/lib/formErrors";
import { useBankAccountsStore } from "../stores";
import type { BankAccount, BankAccountInput } from "../types";

const props = defineProps<{ account?: BankAccount }>();
const emit = defineEmits<{ done: [] }>();

const { t } = useI18n();
const store = useBankAccountsStore();
const { fieldErrors, error, submitting, submit } = useFormSubmit();

const draft = ref({
  label: props.account?.label ?? "",
  currency: props.account?.currency ?? "CZK",
  accountNumber: props.account?.accountNumber ?? "",
  iban: props.account?.iban ?? "",
  bic: props.account?.bic ?? "",
  isDefault: props.account?.isDefault ?? false,
});

const ACCOUNT_NUMBER = /^(\d{1,6}-)?\d{2,10}\/\d{4}$/;

function validate() {
  const d = draft.value;
  const number = d.accountNumber.trim();
  const iban = d.iban.trim();
  return collectErrors({
    currency: /^[A-Za-z]{3}$/.test(d.currency.trim()) ? null : "invalid",
    accountNumber: number === "" && iban === "" ? "required" : number !== "" && !ACCOUNT_NUMBER.test(number) && "invalid",
  });
}

function toInput(): BankAccountInput {
  const d = draft.value;
  return {
    label: nullIfEmpty(d.label),
    currency: d.currency.trim().toUpperCase(),
    accountNumber: nullIfEmpty(d.accountNumber),
    iban: nullIfEmpty(d.iban.replace(/\s+/g, "").toUpperCase()),
    bic: nullIfEmpty(d.bic)?.toUpperCase() ?? null,
    isDefault: d.isDefault,
  };
}

async function onSubmit() {
  if (await submit(validate, () => store.save(toInput(), props.account?.id))) emit("done");
}
</script>

<template>
  <form class="card space-y-4" novalidate @submit.prevent="onSubmit">
    <h2 class="font-semibold">{{ account ? t("settings.bankAccounts.edit") : t("settings.bankAccounts.add") }}</h2>
    <div class="grid gap-4 sm:grid-cols-2">
      <FormField :label="t('settings.bankAccounts.label')" for="bank-label" :error="fieldErrors.label">
        <input id="bank-label" v-model="draft.label" class="input" />
      </FormField>
      <FormField :label="t('settings.bankAccounts.currency')" for="bank-currency" :error="fieldErrors.currency">
        <input id="bank-currency" v-model="draft.currency" maxlength="3" class="input uppercase" :class="{ 'input-error': fieldErrors.currency }" />
      </FormField>
      <FormField
        :label="t('settings.bankAccounts.accountNumber')"
        for="bank-number"
        :error="fieldErrors.accountNumber"
        :hint="t('settings.bankAccounts.accountNumberHint')"
      >
        <input id="bank-number" v-model="draft.accountNumber" class="input" :class="{ 'input-error': fieldErrors.accountNumber }" />
      </FormField>
      <FormField :label="t('settings.bankAccounts.iban')" for="bank-iban" :error="fieldErrors.iban">
        <input id="bank-iban" v-model="draft.iban" class="input uppercase" :class="{ 'input-error': fieldErrors.iban }" />
      </FormField>
      <FormField :label="t('settings.bankAccounts.bic')" for="bank-bic" :error="fieldErrors.bic">
        <input id="bank-bic" v-model="draft.bic" class="input uppercase" :class="{ 'input-error': fieldErrors.bic }" />
      </FormField>
      <label class="flex items-center gap-2 self-end pb-2 text-sm">
        <input v-model="draft.isDefault" type="checkbox" class="size-4 rounded" />
        {{ t("settings.bankAccounts.isDefault") }}
      </label>
    </div>
    <p v-if="error" role="alert" class="alert-error">{{ error }}</p>
    <div class="flex gap-2">
      <button type="submit" class="btn btn-primary" :disabled="submitting">
        {{ submitting ? t("common.saving") : t("common.save") }}
      </button>
      <button type="button" class="btn" @click="emit('done')">{{ t("common.cancel") }}</button>
    </div>
  </form>
</template>
