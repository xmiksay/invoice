<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { useErrorText } from "@/composables/useAction";
import { fieldErrorsOf } from "@/lib/formErrors";
import { describeFieldErrors } from "../fieldMessages";
import { editLocation, listLocation } from "../routes";
import { useDocumentStore } from "../store";
import type { Document } from "../types";

const props = defineProps<{ doc: Document }>();

const { t } = useI18n();
const router = useRouter();
const store = useDocumentStore();
const errorText = useErrorText();

const busy = ref(false);
const error = ref<string | null>(null);
const reasons = ref<string[]>([]);

const issued = computed(() => props.doc.status === "issued");
// A DDPP is created and cancelled by its proforma's payments; only its delivery is tracked here.
const isDdpp = computed(() => props.doc.docType === "advance_tax_doc");
const canCreditNote = computed(() => issued.value && props.doc.docType === "invoice");
const canSettle = computed(() => issued.value && props.doc.docType === "proforma" && !props.doc.settled);

async function act(action: () => Promise<void>): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  error.value = null;
  reasons.value = [];
  try {
    await action();
  } catch (err) {
    const fields = fieldErrorsOf(err);
    error.value = fields ? t("documents.detail.cannotProceed") : errorText(err);
    reasons.value = fields ? describeFieldErrors(fields, t) : [];
  } finally {
    busy.value = false;
  }
}

function issue() {
  if (!window.confirm(t("documents.detail.confirmIssue"))) return;
  void act(store.issue);
}

function remove() {
  if (!window.confirm(t("documents.detail.confirmDelete"))) return;
  const docType = props.doc.docType;
  void act(async () => {
    await store.remove();
    await router.push(listLocation(docType));
  });
}

function cancel() {
  const reason = window.prompt(t("documents.detail.cancelPrompt", { number: props.doc.number ?? "" }));
  if (reason === null) return;
  void act(() => store.cancel(reason.trim() || null));
}

/** The new draft opens in the editor; it is required again there before issue. */
function creditNote() {
  const reason = window.prompt(t("documents.detail.creditNotePrompt", { number: props.doc.number ?? "" }));
  if (reason === null) return;
  void act(async () => {
    const draft = await store.creditNote(reason.trim() || null);
    await router.push(editLocation(draft.id));
  });
}

function settle() {
  void act(async () => {
    const draft = await store.settle();
    await router.push(editLocation(draft.id));
  });
}
</script>

<template>
  <div class="space-y-2">
    <div class="flex flex-wrap gap-2">
      <template v-if="doc.status === 'draft'">
        <RouterLink :to="editLocation(doc.id)" class="btn" data-test="edit">{{ t("common.edit") }}</RouterLink>
        <button type="button" class="btn btn-primary" :disabled="busy" data-test="issue" @click="issue">{{ t("documents.detail.issue") }}</button>
        <button type="button" class="btn btn-danger" :disabled="busy" data-test="delete" @click="remove">{{ t("common.delete") }}</button>
      </template>
      <template v-else-if="issued">
        <button v-if="canSettle" type="button" class="btn btn-primary" :disabled="busy" data-test="settle" @click="settle">
          {{ t("documents.detail.settle") }}
        </button>
        <button type="button" class="btn" :disabled="busy" data-test="mark-sent" @click="act(store.markSent)">
          {{ doc.sentAt ? t("documents.detail.markSentAgain") : t("documents.detail.markSent") }}
        </button>
        <button v-if="canCreditNote" type="button" class="btn" :disabled="busy" data-test="credit-note" @click="creditNote">
          {{ t("documents.detail.creditNote") }}
        </button>
        <button v-if="!isDdpp" type="button" class="btn btn-danger" :disabled="busy" data-test="cancel" @click="cancel">{{ t("documents.detail.cancel") }}</button>
      </template>
    </div>
    <div v-if="error" role="alert" class="alert-error" data-test="action-error">
      <p>{{ error }}</p>
      <ul v-if="reasons.length" class="mt-1 list-disc pl-5">
        <li v-for="r in reasons" :key="r">{{ r }}</li>
      </ul>
    </div>
  </div>
</template>
