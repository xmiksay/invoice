<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useAction } from "@/composables/useAction";
import { useFormSubmit } from "@/composables/useFormSubmit";
import { useDocumentStore } from "@/features/documents/store";
import type { Document } from "@/features/documents/types";
import { toMetadataDraft, toMetadataInput, validateMetadata, type MetadataDraft } from "../metadata";
import { useMetadataSettings } from "../useMetadataSettings";
import MetadataFields from "./MetadataFields.vue";
import { useSessionStore } from "@/stores/session";

/** Detail panel: category, custom fields and internal note stay editable in every status (`PUT …/metadata`). */
const props = defineProps<{ doc: Document }>();

const { t } = useI18n();
const session = useSessionStore();
const store = useDocumentStore();
const settings = useMetadataSettings(() => props.doc.direction);
const { error: loadError, run } = useAction();
const { fieldErrors, error, submitting, submit } = useFormSubmit();

const draft = ref<MetadataDraft | null>(null);
const saved = ref(false);
/** Serialized input the draft started from: the dirty check compares against it, not the live document. */
const baseline = ref("");
const serialize = (d: MetadataDraft) => JSON.stringify(toMetadataInput(d, settings.fieldDefs.value));
function reset() {
  draft.value = toMetadataDraft(props.doc, settings.fieldDefs.value);
  baseline.value = serialize(draft.value);
}
const dirty = computed(() => !!draft.value && serialize(draft.value) !== baseline.value);

void run(async () => {
  await settings.load();
  reset();
});
// Other actions (a payment, issue…) answer with the whole document: take its stored values unless the
// user has unsaved edits here, which are kept. Another document always starts from its own values.
watch(
  () => props.doc,
  (next, prev) => {
    if (draft.value && (next.id !== prev?.id || !dirty.value)) reset();
  },
);

async function onSubmit() {
  const d = draft.value;
  if (!d) return;
  saved.value = false;
  const defs = settings.fieldDefs.value;
  saved.value = await submit(
    () => validateMetadata(d, defs),
    async () => {
      await store.setMetadata(toMetadataInput(d, defs));
      reset();
    },
  );
}
</script>

<template>
  <form class="card space-y-3" novalidate data-test="metadata-card" @submit.prevent="onSubmit">
    <h2 class="font-semibold">{{ t("metadata.title") }}</h2>
    <p v-if="loadError" role="alert" class="alert-error">{{ loadError }}</p>
    <template v-if="draft">
      <MetadataFields
        v-model="draft"
        :direction="doc.direction"
        :categories="settings.categories.value"
        :field-defs="settings.fieldDefs.value"
        :errors="fieldErrors"
        id-prefix="meta"
      />
      <div v-if="session.can('write')" class="flex items-center gap-3">
        <button type="submit" class="btn btn-sm" :disabled="submitting || !dirty" data-test="save-metadata">{{ t("common.save") }}</button>
        <span v-if="saved && !dirty" class="text-xs text-green-700 dark:text-green-400">{{ t("common.saved") }}</span>
      </div>
    </template>
    <p v-if="error" role="alert" class="alert-error">{{ error }}</p>
  </form>
</template>
