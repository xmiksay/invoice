<script setup lang="ts">
import { computed, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useRoute, useRouter } from "vue-router";
import { useAction } from "@/composables/useAction";
import OriginalPdfPanel from "@/features/documents/components/OriginalPdfPanel.vue";
import PartiesCard from "@/features/documents/components/PartiesCard.vue";
import PaymentsPanel from "@/features/documents/components/PaymentsPanel.vue";
import RelatedDocumentsPanel from "@/features/documents/components/RelatedDocumentsPanel.vue";
import StatusBadges from "@/features/documents/components/StatusBadges.vue";
import TotalsPanel from "@/features/documents/components/TotalsPanel.vue";
import { detailLocation, editLocation, listLocation } from "@/features/documents/routes";
import { useDocumentStore } from "@/features/documents/store";
import MetadataCard from "@/features/metadata/components/MetadataCard.vue";
import { useCompanyStore } from "@/features/settings/stores";
import ReceivedInfoCard from "../components/ReceivedInfoCard.vue";

const { t } = useI18n();
const route = useRoute();
const router = useRouter();
const store = useDocumentStore();
const company = useCompanyStore();
// Only fills the customer card; the detail works without it.
if (!company.company) company.load().catch(() => {});
const { error, run } = useAction();
const { error: actionError, run: runAction } = useAction();

const id = computed(() => String(route.params.id));
watch(
  id,
  (docId) =>
    void run(async () => {
      await store.load(docId);
      // An issued document opened under /received/… belongs to the issued views.
      if (store.doc?.id === docId && store.doc.direction !== "received") await router.replace(detailLocation(docId));
    }),
  { immediate: true },
);

const doc = computed(() => (store.doc?.id === id.value && store.doc.direction === "received" ? store.doc : null));
// A DDPP documents a payment already made; it has no payments of its own.
const hasPayments = computed(() => doc.value?.docType !== "advance_tax_doc");

function remove() {
  const current = doc.value;
  if (!current || !window.confirm(t("received.confirmDelete"))) return;
  void runAction(async () => {
    await store.remove();
    await router.push(listLocation(current.docType, "received"));
  });
}
</script>

<template>
  <section class="space-y-4">
    <RouterLink :to="listLocation(doc?.docType ?? 'invoice', 'received')" class="text-sm text-blue-600 hover:underline dark:text-blue-400">
      ← {{ t("documents.detail.backToList") }}
    </RouterLink>

    <p v-if="error" role="alert" class="alert-error">{{ error }}</p>
    <p v-if="!doc && !error" class="text-sm text-gray-500">{{ t("common.loading") }}</p>

    <template v-if="doc">
      <div class="flex flex-wrap items-center gap-3">
        <h1 class="text-2xl font-semibold">
          {{ t(`received.docTypes.${doc.docType}`) }}
          <span class="font-mono">{{ doc.number }}</span>
        </h1>
        <StatusBadges :status="doc.status" :payment-state="doc.paymentState" :overdue="doc.overdue" :sign="doc.sign" direction="received" />
      </div>

      <div class="flex flex-wrap gap-2">
        <RouterLink :to="editLocation(doc.id, 'received')" class="btn" data-test="edit">{{ t("common.edit") }}</RouterLink>
        <button type="button" class="btn btn-danger" data-test="delete" @click="remove">{{ t("common.delete") }}</button>
      </div>
      <p v-if="actionError" role="alert" class="alert-error" data-test="action-error">{{ actionError }}</p>

      <OriginalPdfPanel :doc="doc" />
      <!-- We are the customer of a received document; without a stored snapshot the current company is shown. -->
      <PartiesCard :supplier="doc.supplier" :customer="doc.customer ?? company.company" />
      <RelatedDocumentsPanel :parent="doc.parent" :children="doc.relatedDocuments" :currency="doc.currency" direction="received" />
      <ReceivedInfoCard :doc="doc" />
      <TotalsPanel :totals="doc.totals" :currency="doc.currency" :exchange-rate="doc.exchangeRate" :sign="doc.sign" />
      <PaymentsPanel v-if="hasPayments" :doc="doc" />
      <MetadataCard :doc="doc" />
    </template>
  </section>
</template>
