<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import FormField from "@/components/form/FormField.vue";
import { useAction } from "@/composables/useAction";
import { useFormSubmit } from "@/composables/useFormSubmit";
import { collectErrors, nullIfEmpty, textRule } from "@/lib/formErrors";
import { todayIso } from "../form";
import { formatDate, formatMoney, remainingAmount } from "../format";
import { useDocumentStore } from "../store";
import type { Document } from "../types";

const props = defineProps<{ doc: Document }>();

const { t, locale } = useI18n();
const store = useDocumentStore();
const { fieldErrors, error, submitting, submit } = useFormSubmit();
const { error: removeError, run } = useAction();

const editable = computed(() => props.doc.status === "issued");
// A settled invoice rarely takes another payment; the form (defaulting to 0.00) stays behind a toggle.
const settled = computed(() => props.doc.paymentState === "paid" || props.doc.paymentState === "overpaid");
const addAnother = ref(false);
const remaining = computed(() => remainingAmount(props.doc.totals.payable, props.doc.paid));
const money = (v: string) => formatMoney(v, props.doc.currency, locale.value);

const date = ref(todayIso());
const amount = ref(remaining.value);
const note = ref("");
// The default follows the outstanding amount after each payment change.
watch(remaining, (value) => (amount.value = value));

async function onSubmit() {
  const value = amount.value.trim().replace(",", ".");
  await submit(
    () =>
      collectErrors({
        date: !/^\d{4}-\d{2}-\d{2}$/.test(date.value) && "required",
        amount: !/^\d+(\.\d{1,2})?$/.test(value) || Number(value) <= 0 ? "invalid" : null,
        note: textRule(note.value, { max: 500 }),
      }),
    async () => {
      await store.addPayment({ date: date.value, amount: value, note: nullIfEmpty(note.value) });
      note.value = "";
    },
  );
}

function remove(paymentId: string) {
  if (!window.confirm(t("documents.payments.confirmDelete"))) return;
  void run(() => store.removePayment(paymentId));
}
</script>

<template>
  <section class="card space-y-4" data-test="payments-panel">
    <div class="flex flex-wrap items-baseline justify-between gap-2">
      <h2 class="font-semibold">{{ t("documents.payments.title") }}</h2>
      <p class="text-sm">
        {{ t("documents.payments.paid") }}: <span class="tabular-nums" data-test="paid">{{ money(doc.paid) }}</span>
        · {{ t("documents.payments.remaining") }}: <span class="tabular-nums font-medium" data-test="remaining">{{ money(remaining) }}</span>
      </p>
    </div>

    <table v-if="store.payments.length" class="table">
      <thead>
        <tr>
          <th>{{ t("documents.fields.date") }}</th>
          <th class="text-right">{{ t("documents.fields.amount") }}</th>
          <th class="hidden sm:table-cell">{{ t("documents.fields.note") }}</th>
          <th v-if="editable" class="w-0"><span class="sr-only">{{ t("common.actions") }}</span></th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="p in store.payments" :key="p.id" data-test="payment-row">
          <td class="whitespace-nowrap">{{ formatDate(p.date, locale) }}</td>
          <td class="text-right tabular-nums">{{ money(p.amount) }}</td>
          <td class="hidden sm:table-cell">{{ p.note }}</td>
          <td v-if="editable">
            <button type="button" class="btn btn-sm btn-danger" :aria-label="t('documents.payments.delete')" @click="remove(p.id)">✕</button>
          </td>
        </tr>
      </tbody>
    </table>
    <p v-else class="text-sm text-gray-500">{{ t("documents.payments.empty") }}</p>
    <p v-if="removeError" role="alert" class="alert-error">{{ removeError }}</p>

    <button v-if="editable && settled && !addAnother" type="button" class="btn btn-sm" data-test="add-another" @click="addAnother = true">
      {{ t("documents.payments.addAnother") }}
    </button>
    <form v-if="editable && (!settled || addAnother)" class="grid gap-3 border-t border-gray-200 pt-4 sm:grid-cols-[auto_auto_1fr_auto] sm:items-end dark:border-gray-800" novalidate @submit.prevent="onSubmit">
      <FormField :label="t('documents.fields.date')" for="payment-date" :error="fieldErrors.date">
        <input id="payment-date" v-model="date" type="date" class="input" :class="{ 'input-error': fieldErrors.date }" />
      </FormField>
      <FormField :label="t('documents.fields.amount')" for="payment-amount" :error="fieldErrors.amount">
        <input id="payment-amount" v-model="amount" inputmode="decimal" class="input" :class="{ 'input-error': fieldErrors.amount }" />
      </FormField>
      <FormField :label="t('documents.fields.note')" for="payment-note" :error="fieldErrors.note">
        <input id="payment-note" v-model="note" maxlength="500" class="input" />
      </FormField>
      <button type="submit" class="btn btn-primary" :disabled="submitting" data-test="add-payment">{{ t("documents.payments.add") }}</button>
      <p v-if="error" role="alert" class="alert-error sm:col-span-4">{{ error }}</p>
    </form>
  </section>
</template>
