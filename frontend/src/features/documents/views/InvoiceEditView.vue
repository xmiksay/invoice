<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useRoute, useRouter } from "vue-router";
import { useAction } from "@/composables/useAction";
import { useBankAccountsStore, useCompanyStore, useVatRatesStore } from "@/features/settings/stores";
import InvoiceEditorForm from "../components/InvoiceEditorForm.vue";
import { newDocumentDraft, toDraft, todayIso, type DocumentDraft, type DraftContext } from "../form";
import { detailLocation, listLocation } from "../routes";
import { useDocumentStore } from "../store";
import type { Document, EditableDocType } from "../types";

const { t } = useI18n();
const route = useRoute();
const router = useRouter();
const company = useCompanyStore();
const vatRates = useVatRatesStore();
const bankAccounts = useBankAccountsStore();
const store = useDocumentStore();
const { error, run } = useAction();

const id = computed(() => (typeof route.params.id === "string" ? route.params.id : undefined));
/** `/invoices/new?docType=proforma` starts a proforma; everything else an invoice. */
const newDocType = computed(() => (route.query.docType === "proforma" ? "proforma" : "invoice"));
const initial = ref<DocumentDraft | null>(null);
const ctx = ref<DraftContext | null>(null);

watch(
  () => [id.value, newDocType.value] as const,
  ([docId]) =>
    void run(async () => {
      initial.value = null;
      await Promise.all([
        company.company ? undefined : company.load(),
        vatRates.loaded ? undefined : vatRates.load(),
        bankAccounts.loaded ? undefined : bankAccounts.load(),
      ]);
      if (!company.company) return;
      ctx.value = { company: company.company, vatRates: vatRates.items, bankAccounts: bankAccounts.items, today: todayIso() };
      if (!docId) {
        initial.value = newDocumentDraft(ctx.value, newDocType.value);
        return;
      }
      await store.load(docId);
      if (store.doc && store.doc.status !== "draft") {
        // Issued documents are locked; the detail view has the actions still allowed.
        await router.replace(detailLocation(docId));
        return;
      }
      if (store.doc) initial.value = toDraft(store.doc);
    }),
  { immediate: true },
);

async function onSaved(doc: Document) {
  await router.push(detailLocation(doc.id));
}

const docType = computed<EditableDocType>(() => initial.value?.docType ?? newDocType.value);
const back = computed(() => (id.value ? detailLocation(id.value) : listLocation(docType.value)));
</script>

<template>
  <section class="space-y-4">
    <RouterLink :to="back" class="text-sm text-blue-600 hover:underline dark:text-blue-400">
      ← {{ id ? t("documents.detail.backToDetail") : t("documents.detail.backToList") }}
    </RouterLink>
    <h1 class="text-2xl font-semibold">{{ id ? t(`documents.editor.editTitle.${docType}`) : t(`documents.editor.newTitle.${docType}`) }}</h1>

    <p v-if="error" role="alert" class="alert-error">{{ error }}</p>
    <p v-else-if="!initial" class="text-sm text-gray-500">{{ t("common.loading") }}</p>

    <InvoiceEditorForm v-if="initial && ctx" :key="id ?? `new-${newDocType}`" :initial="initial" :ctx="ctx" :doc-id="id" @saved="onSaved">
      <template #actions>
        <RouterLink :to="back" class="btn">
          {{ t("common.cancel") }}
        </RouterLink>
      </template>
    </InvoiceEditorForm>
  </section>
</template>
