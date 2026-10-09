<script setup lang="ts">
import { reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { FieldErrors } from "@/api/types";
import FormField from "@/components/form/FormField.vue";
import { useErrorText } from "@/composables/useAction";
import { collectErrors, errorDetailOf, fieldErrorsOf, textRule } from "@/lib/formErrors";
import { useToastStore } from "@/stores/toast";
import { emailApi } from "../api";
import { localizeTemplateDetail } from "../templateDetail";
import { useEmailSettingsStore } from "../store";
import type { EmailTemplate, TemplateText } from "../types";

/** Subject + body of one locale: preview on the sample invoice, save, restore the embedded default. */
const props = defineProps<{ template: EmailTemplate }>();

const { t } = useI18n();
const errorText = useErrorText();
const toast = useToastStore();
const store = useEmailSettingsStore();

const form = reactive<TemplateText>({ subject: "", body: "" });
const preview = ref<TemplateText | null>(null);
const busy = ref(false);
const error = ref<string | null>(null);
const detail = ref<string | null>(null);
const fieldErrors = ref<FieldErrors>({});

// The server's answer (save / restore) is the new baseline.
watch(
  () => props.template,
  (tpl) => Object.assign(form, { subject: tpl.subject, body: tpl.body }),
  { immediate: true },
);

const validate = () =>
  collectErrors({
    subject: textRule(form.subject, { required: true, max: 500 }),
    body: textRule(form.body, { max: 20_000 }),
  });

/** Runs one action; a `template_invalid` 422 lands at its field together with the line message. */
async function act(action: () => Promise<void>, check = true): Promise<void> {
  if (busy.value) return;
  error.value = null;
  detail.value = null;
  fieldErrors.value = check ? validate() : {};
  if (Object.keys(fieldErrors.value).length > 0) {
    error.value = t("errors.validation");
    return;
  }
  busy.value = true;
  try {
    await action();
  } catch (err) {
    const fields = fieldErrorsOf(err);
    fieldErrors.value = fields ?? {};
    error.value = errorText(err);
    detail.value = errorDetailOf(err);
  } finally {
    busy.value = false;
  }
}

const locale = () => props.template.locale;
const input = (): TemplateText => ({ subject: form.subject, body: form.body });

const runPreview = () =>
  act(async () => {
    preview.value = null;
    preview.value = await emailApi.previewTemplate(locale(), input());
  });

const save = () =>
  act(async () => {
    await store.saveTemplate(locale(), input());
    toast.show(t("email.templates.saved"));
  });

function restore(): void {
  if (!window.confirm(t("email.templates.confirmRestore"))) return;
  void act(async () => {
    await store.restoreTemplate(locale());
    preview.value = null;
    toast.show(t("email.templates.restored"));
  }, false);
}

const id = (field: string) => `email-template-${props.template.locale}-${field}`;
const lineDetail = (field: "subject" | "body") =>
  fieldErrors.value[field] === "template_invalid" && detail.value ? localizeTemplateDetail(detail.value, t) : null;
</script>

<template>
  <form class="space-y-3" novalidate :data-test="`template-editor-${template.locale}`" @submit.prevent="save">
    <p class="text-sm">
      <span v-if="template.custom" class="badge" data-test="template-custom">{{ t("email.templates.custom") }}</span>
      <span v-else class="badge !bg-gray-100 !text-gray-700 dark:!bg-gray-800 dark:!text-gray-300" data-test="template-default">
        {{ t("email.templates.default") }}
      </span>
    </p>
    <FormField :label="t('email.send.subject')" :for="id('subject')" :error="fieldErrors.subject">
      <input :id="id('subject')" v-model="form.subject" class="input font-mono" :class="{ 'input-error': fieldErrors.subject }" data-test="template-subject" />
    </FormField>
    <template v-if="lineDetail('subject')">
      <p class="font-mono text-xs text-red-600 dark:text-red-400" data-test="template-detail">{{ lineDetail("subject")!.text }}</p>
      <p v-if="lineDetail('subject')!.hint" class="text-xs text-gray-600 dark:text-gray-400" data-test="template-hint">{{ lineDetail("subject")!.hint }}</p>
    </template>
    <FormField :label="t('email.send.body')" :for="id('body')" :error="fieldErrors.body">
      <textarea :id="id('body')" v-model="form.body" rows="14" class="input font-mono" :class="{ 'input-error': fieldErrors.body }" data-test="template-body" />
    </FormField>
    <template v-if="lineDetail('body')">
      <p class="font-mono text-xs text-red-600 dark:text-red-400" data-test="template-detail">{{ lineDetail("body")!.text }}</p>
      <p v-if="lineDetail('body')!.hint" class="text-xs text-gray-600 dark:text-gray-400" data-test="template-hint">{{ lineDetail("body")!.hint }}</p>
    </template>

    <div class="flex flex-wrap gap-2">
      <button type="button" class="btn" :disabled="busy" data-test="template-preview" @click="runPreview">{{ t("email.templates.preview") }}</button>
      <button type="submit" class="btn btn-primary" :disabled="busy" data-test="template-save">{{ t("common.save") }}</button>
      <button v-if="template.custom" type="button" class="btn btn-danger" :disabled="busy" data-test="template-restore" @click="restore">
        {{ t("email.templates.restore") }}
      </button>
    </div>
    <p v-if="error" role="alert" class="alert-error" data-test="template-error">{{ error }}</p>

    <section v-if="preview" class="space-y-2 rounded-md border border-gray-200 p-3 dark:border-gray-800" data-test="template-rendered">
      <h3 class="text-sm font-medium text-gray-600 dark:text-gray-400">{{ t("email.templates.previewTitle") }}</h3>
      <p class="font-medium" data-test="rendered-subject">{{ preview.subject }}</p>
      <pre class="text-sm whitespace-pre-wrap" data-test="rendered-body">{{ preview.body }}</pre>
    </section>
  </form>
</template>
