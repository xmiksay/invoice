<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import FormField from "@/components/form/FormField.vue";
import { useFormSubmit } from "@/composables/useFormSubmit";
import { toAccountingDraft, toAccountingSettings, validateAccounting } from "../accounting";
import PohodaCodesTable from "../components/PohodaCodesTable.vue";
import { useAccountingStore, useCompanyStore } from "../stores";

const { t } = useI18n();
const store = useAccountingStore();
const companyStore = useCompanyStore();
const draft = ref(toAccountingDraft(store.settings));
const saved = ref(false);
// Disabled until the reload lands, so fresh server data never replaces edits typed meanwhile.
const loaded = ref(false);
const { fieldErrors, error, submitting, submit, showError } = useFormSubmit();

onMounted(async () => {
  // The company IČO is only the override's placeholder: its failure must not block the tab.
  if (!companyStore.company) void companyStore.load().catch(() => undefined);
  try {
    await store.load();
    draft.value = toAccountingDraft(store.settings);
  } catch (err) {
    showError(err);
  } finally {
    loaded.value = store.settings !== null;
  }
});

async function onSubmit() {
  saved.value = false;
  saved.value = await submit(
    () => validateAccounting(draft.value),
    async () => {
      await store.save(toAccountingSettings(draft.value));
      draft.value = toAccountingDraft(store.settings);
    },
  );
}
</script>

<template>
  <form novalidate @submit.prevent="onSubmit">
    <fieldset class="space-y-6" :disabled="!loaded">
      <section class="card space-y-4" data-test="pohoda-section">
        <div class="space-y-1">
          <h2 class="text-lg font-semibold">{{ t("accounting.pohoda.title") }}</h2>
          <p class="text-sm text-gray-600 dark:text-gray-400">{{ t("accounting.pohoda.intro") }}</p>
        </div>
        <FormField
          class="max-w-xs"
          :label="t('accounting.pohoda.ico')"
          for="pohoda-ico"
          :error="fieldErrors['pohoda.ico']"
          :hint="t('accounting.pohoda.icoHint')"
        >
          <input
            id="pohoda-ico"
            v-model="draft.ico"
            inputmode="numeric"
            class="input"
            :class="{ 'input-error': fieldErrors['pohoda.ico'] }"
            :placeholder="companyStore.company?.ico ?? ''"
            data-test="pohoda-ico"
          />
        </FormField>
        <PohodaCodesTable v-model="draft.rows" :errors="fieldErrors" />
      </section>

      <p v-if="error" role="alert" class="alert-error" data-test="accounting-error">{{ error }}</p>
      <div class="flex items-center gap-3">
        <button type="submit" class="btn btn-primary" :disabled="submitting" data-test="accounting-save">
          {{ submitting ? t("common.saving") : t("common.save") }}
        </button>
        <span v-if="saved" class="text-sm text-green-700 dark:text-green-400" role="status">{{ t("common.saved") }}</span>
      </div>
    </fieldset>
  </form>
</template>
