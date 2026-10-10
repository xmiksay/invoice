<script setup lang="ts">
import { reactive } from "vue";
import { useI18n } from "vue-i18n";
import FormField from "@/components/form/FormField.vue";
import { useFormSubmit } from "@/composables/useFormSubmit";
import { collectErrors } from "@/lib/formErrors";
import { useMfaStore } from "../mfaStore";
import { codeRule, passwordFieldReason } from "../validation";
import CodeField from "./CodeField.vue";

/** Password + code confirmation for regenerating the recovery codes or turning TOTP off. */
const props = defineProps<{ kind: "regenerate" | "disable" }>();
const emit = defineEmits<{ done: [codes: string[] | null]; cancel: [] }>();
const { t } = useI18n();
const store = useMfaStore();
const form = reactive({ password: "", code: "" });
const { fieldErrors, error, submitting, submit } = useFormSubmit();

async function onSubmit() {
  let codes: string[] | null = null;
  const ok = await submit(
    () => collectErrors({ password: form.password === "" && "required", code: codeRule(form.code) }),
    async () => {
      const body = { password: form.password, code: form.code.trim() };
      if (props.kind === "regenerate") codes = await store.regenerate(body);
      else await store.disable(body);
    },
  );
  if (ok) emit("done", codes);
}
</script>

<template>
  <form class="space-y-4" novalidate :data-test="`mfa-${kind}`" @submit.prevent="onSubmit">
    <p class="text-sm text-gray-600 dark:text-gray-400">{{ t(`mfa.${kind}.intro`) }}</p>
    <div v-if="kind === 'disable' && store.status?.requiredBy.length" class="alert-error space-y-1" data-test="mfa-required-by">
      <p>{{ t("mfa.disable.requiredBy") }}</p>
      <ul class="list-disc pl-5">
        <li v-for="space in store.status.requiredBy" :key="space.slug">{{ space.name }}</li>
      </ul>
    </div>
    <FormField :label="t('mfa.password')" :for="`mfa-${kind}-password`" :error="passwordFieldReason(fieldErrors.password)">
      <input
        :id="`mfa-${kind}-password`"
        v-model="form.password"
        type="password"
        autocomplete="current-password"
        class="input"
        :class="{ 'input-error': fieldErrors.password }"
        :data-test="`mfa-${kind}-password`"
      />
    </FormField>
    <CodeField :id="`mfa-${kind}-code`" v-model="form.code" :error="fieldErrors.code" />
    <p v-if="error" role="alert" class="alert-error" :data-test="`mfa-${kind}-error`">{{ error }}</p>
    <div class="flex gap-2">
      <button type="submit" class="btn" :class="kind === 'disable' ? 'btn-danger' : 'btn-primary'" :disabled="submitting" :data-test="`mfa-${kind}-submit`">
        {{ t(`mfa.${kind}.submit`) }}
      </button>
      <button type="button" class="btn" @click="emit('cancel')">{{ t("common.cancel") }}</button>
    </div>
  </form>
</template>
