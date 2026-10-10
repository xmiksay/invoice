<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRoute } from "vue-router";
import FormField from "@/components/form/FormField.vue";
import { useFormSubmit } from "@/composables/useFormSubmit";
import { collectErrors } from "@/lib/formErrors";
import { useSessionStore } from "@/stores/session";
import { authApi } from "../api";
import AuthCard from "../components/AuthCard.vue";
import { passwordRule } from "../validation";

const { t } = useI18n();
const route = useRoute();
const session = useSessionStore();
const token = typeof route.query.token === "string" ? route.query.token : "";
const password = ref("");
const done = ref(false);
const { fieldErrors, error, submitting, submit } = useFormSubmit();

async function onSubmit() {
  done.value = await submit(
    () => collectErrors({ password: passwordRule(password.value) }),
    async () => {
      await authApi.confirmReset(token, password.value);
      // The reset revokes every session, this browser's included.
      session.clear();
    },
  );
}
</script>

<template>
  <AuthCard :title="t('auth.reset.title')">
    <p v-if="done" role="status" class="text-sm" data-test="reset-done">{{ t("auth.reset.done") }}</p>
    <template v-else-if="!token || fieldErrors.token">
      <p role="alert" class="alert-error" data-test="reset-invalid">{{ t("auth.reset.invalid") }}</p>
      <RouterLink :to="{ name: 'forgot' }" class="btn w-full">{{ t("auth.reset.requestNew") }}</RouterLink>
    </template>
    <form v-else class="space-y-4" novalidate @submit.prevent="onSubmit">
      <FormField :label="t('auth.reset.password')" for="reset-password" :error="fieldErrors.password" :hint="t('auth.passwordHint')">
        <input id="reset-password" v-model="password" type="password" autocomplete="new-password" class="input" :class="{ 'input-error': fieldErrors.password }" data-test="reset-password" />
      </FormField>
      <p v-if="error" role="alert" class="alert-error">{{ error }}</p>
      <button type="submit" class="btn btn-primary w-full" :disabled="submitting" data-test="reset-submit">{{ t("auth.reset.submit") }}</button>
    </form>
    <RouterLink :to="{ name: 'login' }" class="block text-sm text-blue-600 hover:underline dark:text-blue-400">{{ t("auth.toLogin") }}</RouterLink>
  </AuthCard>
</template>
