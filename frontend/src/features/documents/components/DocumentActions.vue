<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { useErrorText } from "@/composables/useAction";
import { fieldErrorsOf } from "@/lib/formErrors";
import { describeFieldErrors } from "../fieldMessages";
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
  void act(async () => {
    await store.remove();
    await router.push({ name: "invoices" });
  });
}

function cancel() {
  const reason = window.prompt(t("documents.detail.cancelPrompt", { number: props.doc.number ?? "" }));
  if (reason === null) return;
  void act(() => store.cancel(reason.trim() || null));
}
</script>

<template>
  <div class="space-y-2">
    <div class="flex flex-wrap gap-2">
      <template v-if="doc.status === 'draft'">
        <RouterLink :to="{ name: 'invoice-edit', params: { id: doc.id } }" class="btn" data-test="edit">{{ t("common.edit") }}</RouterLink>
        <button type="button" class="btn btn-primary" :disabled="busy" data-test="issue" @click="issue">{{ t("documents.detail.issue") }}</button>
        <button type="button" class="btn btn-danger" :disabled="busy" data-test="delete" @click="remove">{{ t("common.delete") }}</button>
      </template>
      <template v-else-if="doc.status === 'issued'">
        <button type="button" class="btn" :disabled="busy" data-test="mark-sent" @click="act(store.markSent)">
          {{ doc.sentAt ? t("documents.detail.markSentAgain") : t("documents.detail.markSent") }}
        </button>
        <button type="button" class="btn btn-danger" :disabled="busy" data-test="cancel" @click="cancel">{{ t("documents.detail.cancel") }}</button>
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
