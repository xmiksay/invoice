<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { useFormSubmit } from "@/composables/useFormSubmit";
import { collectErrors, isApiError } from "@/lib/formErrors";
import { useSessionStore } from "@/stores/session";
import { codeRule } from "../validation";
import CodeField from "./CodeField.vue";

/** Second login step for a user with TOTP: the pending login lives in the `invoice_mfa` cookie. */
const emit = defineEmits<{ done: []; expired: [] }>();
const { t } = useI18n();
const session = useSessionStore();
const code = ref("");
const recovery = ref(false);
const { fieldErrors, error, submitting, submit } = useFormSubmit();

function toggleRecovery() {
  recovery.value = !recovery.value;
  code.value = "";
  fieldErrors.value = {};
}

async function onSubmit() {
  let expired = false;
  const ok = await submit(
    () => collectErrors({ code: codeRule(code.value) }),
    async () => {
      try {
        await session.loginMfa(code.value.trim());
      } catch (err) {
        // No / expired pending login (5 min, or 5 wrong codes): only the password step can restart it.
        expired = isApiError(err, 401, "invalid_credentials");
        throw err;
      }
    },
  );
  if (ok) emit("done");
  else if (expired) emit("expired");
}
</script>

<template>
  <form class="space-y-4" novalidate data-test="login-code-step" @submit.prevent="onSubmit">
    <p class="text-sm text-gray-600 dark:text-gray-400">{{ recovery ? t("auth.login.mfa.introRecovery") : t("auth.login.mfa.intro") }}</p>
    <CodeField id="login-code" v-model="code" :mode="recovery ? 'recovery' : 'totp'" :error="fieldErrors.code" />
    <p v-if="error" role="alert" class="alert-error" data-test="login-code-error">{{ error }}</p>
    <button type="submit" class="btn btn-primary w-full" :disabled="submitting" data-test="login-code-submit">
      {{ submitting ? t("auth.login.submitting") : t("auth.login.mfa.submit") }}
    </button>
    <button type="button" class="text-sm text-blue-600 hover:underline dark:text-blue-400" data-test="login-recovery-toggle" @click="toggleRecovery">
      {{ recovery ? t("auth.login.mfa.useTotp") : t("auth.login.mfa.useRecovery") }}
    </button>
  </form>
</template>
