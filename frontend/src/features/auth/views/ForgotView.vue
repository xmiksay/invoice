<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import FormField from "@/components/form/FormField.vue";
import { useFormSubmit } from "@/composables/useFormSubmit";
import { collectErrors } from "@/lib/formErrors";
import { authApi } from "../api";
import AuthCard from "../components/AuthCard.vue";
import { emailRule } from "../validation";

const { t, locale } = useI18n();
const email = ref("");
const sent = ref(false);
const { fieldErrors, error, submitting, submit } = useFormSubmit();

async function onSubmit() {
  sent.value = await submit(
    () => collectErrors({ email: emailRule(email.value) }),
    () => authApi.requestReset(email.value.trim(), locale.value),
  );
}
</script>

<template>
  <AuthCard :title="t('auth.forgot.title')">
    <p v-if="sent" role="status" class="text-sm" data-test="forgot-sent">{{ t("auth.forgot.sent") }}</p>
    <form v-else class="space-y-4" novalidate @submit.prevent="onSubmit">
      <p class="text-sm text-gray-600 dark:text-gray-400">{{ t("auth.forgot.intro") }}</p>
      <FormField :label="t('auth.email')" for="forgot-email" :error="fieldErrors.email">
        <input id="forgot-email" v-model="email" type="email" autocomplete="email" class="input" :class="{ 'input-error': fieldErrors.email }" data-test="forgot-email" />
      </FormField>
      <p v-if="error" role="alert" class="alert-error">{{ error }}</p>
      <button type="submit" class="btn btn-primary w-full" :disabled="submitting" data-test="forgot-submit">{{ t("auth.forgot.submit") }}</button>
    </form>
    <RouterLink :to="{ name: 'login' }" class="block text-sm text-blue-600 hover:underline dark:text-blue-400">{{ t("auth.toLogin") }}</RouterLink>
  </AuthCard>
</template>
