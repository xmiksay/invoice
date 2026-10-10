<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRoute } from "vue-router";
import { useErrorText } from "@/composables/useAction";
import { fieldErrorsOf } from "@/lib/formErrors";
import { useSessionStore } from "@/stores/session";
import { authApi } from "../api";
import AuthCard from "../components/AuthCard.vue";
import EmailResendForm from "../components/EmailResendForm.vue";

const { t } = useI18n();
const route = useRoute();
const session = useSessionStore();
const errorText = useErrorText();
const token = typeof route.query.token === "string" ? route.query.token : "";

type State = "needed" | "verifying" | "verified" | "invalid" | "failed";
const state = ref<State>(token ? "verifying" : "needed");
const error = ref<string | null>(null);

onMounted(async () => {
  if (!token) return;
  try {
    await authApi.verify(token);
    state.value = "verified";
    if (session.me) await session.loadMe();
  } catch (err) {
    state.value = fieldErrorsOf(err)?.token ? "invalid" : "failed";
    error.value = errorText(err);
  }
});
</script>

<template>
  <AuthCard :title="t('auth.verify.title')">
    <p v-if="state === 'verifying'" class="text-sm text-gray-500">{{ t("auth.verify.verifying") }}</p>
    <template v-else-if="state === 'verified'">
      <p role="status" class="text-sm text-green-700 dark:text-green-400" data-test="verified">{{ t("auth.verify.success") }}</p>
      <RouterLink :to="session.me ? { name: 'home' } : { name: 'login' }" class="btn btn-primary w-full" data-test="verify-continue">
        {{ t("auth.verify.continue") }}
      </RouterLink>
    </template>
    <template v-else>
      <p v-if="state === 'invalid'" role="alert" class="alert-error" data-test="verify-invalid">{{ t("auth.verify.invalid") }}</p>
      <p v-else-if="state === 'failed'" role="alert" class="alert-error">{{ error }}</p>
      <p v-else class="text-sm" data-test="verify-needed">{{ t("auth.verify.needed") }}</p>
      <EmailResendForm :email="session.me?.user.email" />
    </template>
  </AuthCard>
</template>
