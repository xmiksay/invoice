<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import FormField from "@/components/form/FormField.vue";
import { useFormSubmit } from "@/composables/useFormSubmit";
import { collectErrors, nullIfEmpty, textRule } from "@/lib/formErrors";
import { useDocumentStore } from "../store";

/** Editable in every status — the only free-text field an issued document still accepts. */
const props = defineProps<{ note: string | null }>();

const { t } = useI18n();
const store = useDocumentStore();
const { fieldErrors, error, submitting, submit } = useFormSubmit();
const text = ref(props.note ?? "");
const saved = ref(false);
watch(
  () => props.note,
  (value) => (text.value = value ?? ""),
);

async function onSubmit() {
  saved.value = false;
  const ok = await submit(
    () => collectErrors({ internalNote: textRule(text.value, { max: 2000 }) }),
    () => store.setInternalNote(nullIfEmpty(text.value)),
  );
  saved.value = ok;
}
</script>

<template>
  <form class="card space-y-2" novalidate @submit.prevent="onSubmit">
    <FormField :label="t('documents.fields.internalNote')" for="internal-note" :error="fieldErrors.internalNote" :hint="t('documents.editor.internalNoteHint')">
      <textarea id="internal-note" v-model="text" rows="3" maxlength="2000" class="input" @input="saved = false" />
    </FormField>
    <div class="flex items-center gap-3">
      <button type="submit" class="btn btn-sm" :disabled="submitting || text === (note ?? '')" data-test="save-note">{{ t("common.save") }}</button>
      <span v-if="saved" class="text-xs text-green-700 dark:text-green-400">{{ t("common.saved") }}</span>
    </div>
    <p v-if="error" role="alert" class="alert-error">{{ error }}</p>
  </form>
</template>
