<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import FormField from "@/components/form/FormField.vue";
import { useFormSubmit } from "@/composables/useFormSubmit";
import { collectErrors, fieldErrorsOf } from "@/lib/formErrors";
import { qrDataUri } from "@/lib/qr";
import { useMfaStore } from "../mfaStore";
import type { MfaSetup } from "../types";
import { codeRule, passwordFieldReason } from "../validation";
import CodeField from "./CodeField.vue";

/** Enrolment: password → QR + manual key → first code. Emits the recovery codes. */
const emit = defineEmits<{ enabled: [codes: string[]]; cancel: [] }>();
const { t } = useI18n();
const store = useMfaStore();
const password = ref("");
const code = ref("");
const setup = ref<MfaSetup | null>(null);
const qr = ref<string | null>(null);
const { fieldErrors, error, submitting, submit } = useFormSubmit();

async function onPassword() {
  await submit(
    () => collectErrors({ password: password.value === "" && "required" }),
    async () => {
      const next = await store.setup(password.value);
      // A QR failure is not fatal: the manual key below works on its own.
      qr.value = await qrDataUri(next.otpauthUri).catch(() => null);
      setup.value = next;
      password.value = "";
    },
  );
}

async function onCode() {
  let codes: string[] = [];
  const ok = await submit(
    () => collectErrors({ code: codeRule(code.value) }),
    async () => {
      try {
        codes = await store.enable(code.value.trim());
      } catch (err) {
        // The pending secret lives 10 minutes: start over from the password.
        if (fieldErrorsOf(err)?.code === "expired") restart();
        throw err;
      }
    },
  );
  if (ok) emit("enabled", codes);
}

function restart() {
  setup.value = null;
  qr.value = null;
  code.value = "";
}
</script>

<template>
  <div class="space-y-4" data-test="mfa-enable">
    <form v-if="!setup" class="space-y-4" novalidate @submit.prevent="onPassword">
      <p class="text-sm text-gray-600 dark:text-gray-400">{{ t("mfa.enable.passwordIntro") }}</p>
      <FormField :label="t('mfa.password')" for="mfa-setup-password" :error="passwordFieldReason(fieldErrors.password)">
        <input id="mfa-setup-password" v-model="password" type="password" autocomplete="current-password" class="input" :class="{ 'input-error': fieldErrors.password }" data-test="mfa-setup-password" />
      </FormField>
      <p v-if="fieldErrors.code === 'expired'" role="status" class="text-sm text-amber-700 dark:text-amber-400" data-test="mfa-setup-expired">{{ t("mfa.enable.expired") }}</p>
      <p v-else-if="error" role="alert" class="alert-error" data-test="mfa-enable-error">{{ error }}</p>
      <div class="flex gap-2">
        <button type="submit" class="btn btn-primary" :disabled="submitting" data-test="mfa-setup-submit">{{ t("mfa.enable.continue") }}</button>
        <button type="button" class="btn" @click="emit('cancel')">{{ t("common.cancel") }}</button>
      </div>
    </form>
    <form v-else class="space-y-4" novalidate @submit.prevent="onCode">
      <p class="text-sm">{{ t("mfa.enable.scan") }}</p>
      <img v-if="qr" :src="qr" :alt="t('mfa.enable.qrAlt')" class="h-48 w-48 rounded-md bg-white" data-test="mfa-qr" />
      <FormField :label="t('mfa.enable.manualKey')" for="mfa-secret">
        <input id="mfa-secret" :value="setup.secret" readonly class="input font-mono" data-test="mfa-secret" @focus="($event.target as HTMLInputElement).select()" />
      </FormField>
      <CodeField id="mfa-enable-code" v-model="code" mode="totp" :error="fieldErrors.code" />
      <p v-if="error" role="alert" class="alert-error" data-test="mfa-enable-error">{{ error }}</p>
      <div class="flex gap-2">
        <button type="submit" class="btn btn-primary" :disabled="submitting" data-test="mfa-enable-submit">{{ t("mfa.enable.submit") }}</button>
        <button type="button" class="btn" @click="emit('cancel')">{{ t("common.cancel") }}</button>
      </div>
    </form>
  </div>
</template>
