<script setup lang="ts">
import { onMounted, reactive, ref, useTemplateRef } from "vue";
import { useI18n } from "vue-i18n";
import type { FieldErrors } from "@/api/types";
import FormField from "@/components/form/FormField.vue";
import { useErrorText } from "@/composables/useAction";
import { fieldErrorsOf } from "@/lib/formErrors";
import { downloadPdf } from "@/lib/pdf";
import { csvExportApi } from "../api";
import { exportErrorKey, periodErrors, previousMonth } from "../period";
import type { AccountantDirection, AccountantQuery } from "../types";

const emit = defineEmits<{ close: [] }>();

const { t } = useI18n();
const errorText = useErrorText();

const DIRECTIONS: AccountantDirection[] = ["both", "issued", "received"];
const form = reactive<AccountantQuery>({ ...previousMonth(), direction: "both" });
const busy = ref(false);
const error = ref<string | null>(null);
const fieldErrors = ref<FieldErrors>({});

function fail(err: unknown): void {
  const key = exportErrorKey(err);
  if (key) {
    error.value = t(key);
    return;
  }
  const fields = fieldErrorsOf(err);
  fieldErrors.value = fields ?? {};
  error.value = fields ? t("errors.validation") : errorText(err);
}

async function download(): Promise<void> {
  if (busy.value) return;
  error.value = null;
  fieldErrors.value = periodErrors(form.from, form.to);
  if (Object.keys(fieldErrors.value).length > 0) {
    error.value = t("errors.validation");
    return;
  }
  busy.value = true;
  const query = { ...form };
  try {
    await downloadPdf(() => csvExportApi.accountant(query), `ucetni-${query.from}-${query.to}.csv`);
    emit("close");
  } catch (err) {
    fail(err);
  } finally {
    busy.value = false;
  }
}

const panel = useTemplateRef<HTMLElement>("panel");
onMounted(() => panel.value?.focus());
</script>

<template>
  <div class="fixed inset-0 z-40 flex items-start justify-center overflow-y-auto bg-black/40 p-4 sm:p-8" @click.self="emit('close')">
    <div
      ref="panel"
      role="dialog"
      aria-modal="true"
      aria-labelledby="accountant-export-title"
      tabindex="-1"
      class="card w-full max-w-md space-y-4 outline-none"
      data-test="accountant-export-dialog"
      @keydown.esc="emit('close')"
    >
      <h2 id="accountant-export-title" class="text-lg font-semibold">{{ t("csvExport.accountant.title") }}</h2>
      <p class="text-sm text-gray-600 dark:text-gray-400">{{ t("csvExport.accountant.intro") }}</p>

      <form class="space-y-3" novalidate @submit.prevent="download">
        <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
          <FormField :label="t('csvExport.accountant.from')" for="accountant-from" :error="fieldErrors.from">
            <input id="accountant-from" v-model="form.from" type="date" class="input" :class="{ 'input-error': fieldErrors.from }" data-test="accountant-from" />
          </FormField>
          <FormField :label="t('csvExport.accountant.to')" for="accountant-to" :error="fieldErrors.to">
            <input id="accountant-to" v-model="form.to" type="date" class="input" :class="{ 'input-error': fieldErrors.to }" data-test="accountant-to" />
          </FormField>
        </div>
        <p class="text-xs text-gray-500 dark:text-gray-400">{{ t("csvExport.accountant.periodHint") }}</p>
        <FormField :label="t('csvExport.accountant.direction')" for="accountant-direction" :error="fieldErrors.direction">
          <select id="accountant-direction" v-model="form.direction" class="input" :class="{ 'input-error': fieldErrors.direction }" data-test="accountant-direction">
            <option v-for="d in DIRECTIONS" :key="d" :value="d">{{ t(`csvExport.accountant.directions.${d}`) }}</option>
          </select>
        </FormField>
      </form>

      <p v-if="error" role="alert" class="alert-error" data-test="accountant-export-error">{{ error }}</p>

      <div class="flex justify-end gap-2">
        <button type="button" class="btn" data-test="accountant-cancel" @click="emit('close')">{{ t("common.cancel") }}</button>
        <button type="button" class="btn btn-primary" :disabled="busy" data-test="accountant-download" @click="download">
          {{ busy ? t("csvExport.preparing") : t("csvExport.accountant.download") }}
        </button>
      </div>
    </div>
  </div>
</template>
