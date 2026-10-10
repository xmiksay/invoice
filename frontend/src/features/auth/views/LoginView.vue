<script setup lang="ts">
import { computed, reactive } from "vue";
import { useI18n } from "vue-i18n";
import { useRoute, useRouter } from "vue-router";
import FormField from "@/components/form/FormField.vue";
import { useFormSubmit } from "@/composables/useFormSubmit";
import { collectErrors } from "@/lib/formErrors";
import { safeReturnPath } from "@/lib/returnPath";
import { useSessionStore } from "@/stores/session";
import AuthCard from "../components/AuthCard.vue";
import { emailRule } from "../validation";

const { t } = useI18n();
const route = useRoute();
const router = useRouter();
const session = useSessionStore();
const form = reactive({ email: "", password: "" });
const { fieldErrors, error, submitting, submit } = useFormSubmit();

const title = computed(() => {
  const space = session.context?.space;
  return space ? t("auth.login.spaceTitle", { name: space.name }) : t("auth.login.title");
});
const canRegister = computed(() => session.context?.kind === "base" && session.context.registration);

async function onSubmit() {
  const ok = await submit(
    () => collectErrors({ email: emailRule(form.email), password: form.password === "" && "required" }),
    () => session.login({ email: form.email.trim(), password: form.password }),
  );
  if (ok) await router.replace(safeReturnPath(route.query.return));
}
</script>

<template>
  <AuthCard :title="title">
    <form class="space-y-4" novalidate @submit.prevent="onSubmit">
      <FormField :label="t('auth.email')" for="login-email" :error="fieldErrors.email">
        <input id="login-email" v-model="form.email" type="email" autocomplete="username" autofocus class="input" :class="{ 'input-error': fieldErrors.email }" data-test="login-email" />
      </FormField>
      <FormField :label="t('auth.password')" for="login-password" :error="fieldErrors.password">
        <input id="login-password" v-model="form.password" type="password" autocomplete="current-password" class="input" :class="{ 'input-error': fieldErrors.password }" data-test="login-password" />
      </FormField>
      <p v-if="error" role="alert" class="alert-error" data-test="login-error">{{ error }}</p>
      <button type="submit" class="btn btn-primary w-full" :disabled="submitting" data-test="login-submit">
        {{ submitting ? t("auth.login.submitting") : t("auth.login.submit") }}
      </button>
    </form>
    <div class="flex flex-col gap-1 text-sm">
      <RouterLink :to="{ name: 'forgot' }" class="text-blue-600 hover:underline dark:text-blue-400">{{ t("auth.login.forgot") }}</RouterLink>
      <RouterLink v-if="canRegister" :to="{ name: 'register' }" class="text-blue-600 hover:underline dark:text-blue-400" data-test="register-link">
        {{ t("auth.login.register") }}
      </RouterLink>
    </div>
  </AuthCard>
</template>
