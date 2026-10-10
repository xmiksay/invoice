<script setup lang="ts">
import { reactive } from "vue";
import { useI18n } from "vue-i18n";
import FormField from "@/components/form/FormField.vue";
import { useAction } from "@/composables/useAction";
import { useFormSubmit } from "@/composables/useFormSubmit";
import LeaveSpaceSection from "@/features/members/components/LeaveSpaceSection.vue";
import { collectErrors } from "@/lib/formErrors";
import { useSessionStore } from "@/stores/session";
import { useToastStore } from "@/stores/toast";
import { authApi } from "../api";
import { passwordFieldReason, passwordRule } from "../validation";

const { t } = useI18n();
const session = useSessionStore();
const toast = useToastStore();
const form = reactive({ currentPassword: "", newPassword: "" });
const { fieldErrors, error, submitting, submit } = useFormSubmit();
const revoke = useAction();

async function changePassword() {
  const ok = await submit(
    () => collectErrors({ currentPassword: form.currentPassword === "" && "required", newPassword: passwordRule(form.newPassword) }),
    () => authApi.changePassword({ ...form }),
  );
  if (!ok) return;
  form.currentPassword = "";
  form.newPassword = "";
  toast.show(t("account.password.changed"));
}

async function revokeOthers() {
  if (!window.confirm(t("account.sessions.confirm"))) return;
  if (await revoke.run(authApi.revokeOthers)) toast.show(t("account.sessions.revoked"));
}
</script>

<template>
  <section class="max-w-xl space-y-4">
    <h1 class="text-2xl font-semibold">{{ t("account.title") }}</h1>

    <dl v-if="session.me" class="card grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm">
      <dt class="text-gray-500">{{ t("account.name") }}</dt>
      <dd data-test="account-name">{{ session.me.user.displayName }}</dd>
      <dt class="text-gray-500">{{ t("account.email") }}</dt>
      <dd data-test="account-email">{{ session.me.user.email }}</dd>
    </dl>

    <form class="card space-y-4" novalidate @submit.prevent="changePassword">
      <h2 class="text-lg font-semibold">{{ t("account.password.title") }}</h2>
      <!-- Hidden username lets password managers pair the new password with the account. -->
      <input type="email" autocomplete="username" :value="session.me?.user.email" class="hidden" readonly />
      <FormField :label="t('account.password.current')" for="account-current" :error="passwordFieldReason(fieldErrors.currentPassword)">
        <input id="account-current" v-model="form.currentPassword" type="password" autocomplete="current-password" class="input" :class="{ 'input-error': fieldErrors.currentPassword }" data-test="current-password" />
      </FormField>
      <FormField :label="t('account.password.new')" for="account-new" :error="fieldErrors.newPassword" :hint="t('auth.passwordHint')">
        <input id="account-new" v-model="form.newPassword" type="password" autocomplete="new-password" class="input" :class="{ 'input-error': fieldErrors.newPassword }" data-test="new-password" />
      </FormField>
      <p v-if="error" role="alert" class="alert-error" data-test="password-error">{{ error }}</p>
      <button type="submit" class="btn btn-primary" :disabled="submitting" data-test="change-password">{{ t("account.password.submit") }}</button>
    </form>

    <section class="card space-y-3">
      <h2 class="text-lg font-semibold">{{ t("account.sessions.title") }}</h2>
      <p class="text-sm text-gray-600 dark:text-gray-400">{{ t("account.sessions.intro") }}</p>
      <p v-if="revoke.error.value" role="alert" class="alert-error">{{ revoke.error.value }}</p>
      <button type="button" class="btn" data-test="revoke-others" @click="revokeOthers">{{ t("account.sessions.revoke") }}</button>
    </section>

    <LeaveSpaceSection v-if="session.role" />
  </section>
</template>
