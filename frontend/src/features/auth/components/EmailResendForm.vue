<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import FormField from "@/components/form/FormField.vue";
import { useFormSubmit } from "@/composables/useFormSubmit";
import { collectErrors } from "@/lib/formErrors";
import { authApi } from "../api";
import { emailRule } from "../validation";

const props = defineProps<{ email?: string }>();
const { t, locale } = useI18n();
const email = ref(props.email ?? "");
const sent = ref(false);
const { fieldErrors, error, submitting, submit } = useFormSubmit();

async function onSubmit() {
  sent.value = await submit(
    () => collectErrors({ email: emailRule(email.value) }),
    () => authApi.resendVerification(email.value.trim(), locale.value),
  );
}
</script>

<template>
  <!-- 202 always (no account enumeration): the same message whatever the server knows. -->
  <p v-if="sent" role="status" class="text-sm text-green-700 dark:text-green-400" data-test="resent">{{ t("auth.verify.resent") }}</p>
  <form v-else class="space-y-3" novalidate @submit.prevent="onSubmit">
    <FormField :label="t('auth.email')" for="resend-email" :error="fieldErrors.email">
      <input id="resend-email" v-model="email" type="email" autocomplete="email" class="input" :class="{ 'input-error': fieldErrors.email }" data-test="resend-email" />
    </FormField>
    <p v-if="error" role="alert" class="alert-error">{{ error }}</p>
    <button type="submit" class="btn w-full" :disabled="submitting" data-test="resend">{{ t("auth.verify.resend") }}</button>
  </form>
</template>
