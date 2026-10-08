<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useFormSubmit } from "@/composables/useFormSubmit";
import ContactPicker from "@/features/documents/components/ContactPicker.vue";
import RelatedDocumentPicker from "@/features/documents/components/RelatedDocumentPicker.vue";
import { defaultVatRate } from "@/features/documents/form";
import { useDocumentStore } from "@/features/documents/store";
import type { Document, RelatedDocument } from "@/features/documents/types";
import { useIndicativeRate } from "@/features/documents/useIndicativeRate";
import MetadataFields from "@/features/metadata/components/MetadataFields.vue";
import type { Category } from "@/features/settings/types";
import { applyDefaults, defaultVatDeductible, toReceivedInput, validateReceived, type ReceivedContext, type ReceivedDraft } from "../form";
import { enforceRecapVatMode } from "../recap";
import ReceivedHeaderFields from "./ReceivedHeaderFields.vue";
import VatRecapEditor from "./VatRecapEditor.vue";

const props = defineProps<{
  initial: ReceivedDraft;
  ctx: ReceivedContext;
  categories: Category[];
  docId?: string;
  /** The linked document of an edited one (label for the picker). */
  parent?: RelatedDocument | null;
}>();
const emit = defineEmits<{ saved: [doc: Document] }>();

const { t } = useI18n();
const store = useDocumentStore();
const draft = ref<ReceivedDraft>(applyDefaults(props.initial));
const { fieldErrors, error, submitting, submit } = useFormSubmit();
const indicative = useIndicativeRate(() => ({ currency: draft.value.currency, date: draft.value.receivedDate }));

// receivedDate / payable follow their sources until edited.
watch(draft, (d) => (draft.value = applyDefaults(d)), { deep: true });
watch(
  () => draft.value.vatMode,
  (mode) => {
    draft.value = { ...draft.value, vatRecap: enforceRecapVatMode(draft.value.vatRecap, mode), vatDeductible: defaultVatDeductible(mode) };
  },
);

const rateOptions = computed(() => props.ctx.vatRates.filter((r) => r.active).map((r) => r.rate));
const defaultRate = computed(() => defaultVatRate(props.ctx.vatRates, "standard"));

async function onSubmit() {
  await submit(
    () => validateReceived(draft.value, props.ctx.fieldDefs),
    async () => {
      emit("saved", await store.save(toReceivedInput(draft.value, props.ctx.fieldDefs), props.docId));
    },
  );
}
</script>

<template>
  <form class="space-y-6" novalidate data-test="received-form" @submit.prevent="onSubmit">
    <section class="card">
      <ContactPicker
        :contact-id="draft.contactId"
        :error="fieldErrors.contactId"
        :label="t('documents.fields.supplier')"
        @pick="draft.contactId = $event.id"
        @clear="draft.contactId = null"
      />
    </section>

    <section class="card">
      <ReceivedHeaderFields
        v-model="draft"
        :errors="fieldErrors"
        :indicative-rate="indicative.rate.value"
        :indicative-error="indicative.error.value"
        :indicative-loading="indicative.loading.value"
      />
    </section>

    <section class="card">
      <VatRecapEditor v-model="draft" :errors="fieldErrors" :rate-options="rateOptions" :default-rate="defaultRate" />
    </section>

    <section class="card space-y-4">
      <RelatedDocumentPicker
        v-model="draft.relatedDocumentId"
        direction="received"
        :doc-type="draft.docType"
        :contact-id="draft.contactId"
        :current="parent"
        :exclude-id="docId"
        :error="fieldErrors.relatedDocumentId"
      />
      <MetadataFields v-model="draft.meta" direction="received" :categories="categories" :field-defs="ctx.fieldDefs" :errors="fieldErrors" id-prefix="rec" />
    </section>

    <p v-if="error" role="alert" class="alert-error" data-test="form-error">{{ error }}</p>

    <div class="flex flex-wrap items-center gap-2">
      <button type="submit" class="btn btn-primary" :disabled="submitting" data-test="save-received">
        {{ submitting ? t("common.saving") : t("common.save") }}
      </button>
      <slot name="actions" />
    </div>
  </form>
</template>
