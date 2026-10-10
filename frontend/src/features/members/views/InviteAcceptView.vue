<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRoute, useRouter } from "vue-router";
import { ApiError } from "@/api/client";
import FormField from "@/components/form/FormField.vue";
import { useFormSubmit } from "@/composables/useFormSubmit";
import AuthCard from "@/features/auth/components/AuthCard.vue";
import { passwordFieldReason, passwordRule } from "@/features/auth/validation";
import { collectErrors, textRule } from "@/lib/formErrors";
import { useSessionStore } from "@/stores/session";
import { membersApi } from "../api";
import type { InviteInfo } from "../types";

const { t } = useI18n();
const route = useRoute();
const router = useRouter();
const session = useSessionStore();
const token = typeof route.query.token === "string" ? route.query.token : "";
const info = ref<InviteInfo | null>(null);
const form = reactive({ displayName: "", password: "" });
const { fieldErrors, error, submitting, submit, showError } = useFormSubmit();
const lookupNotFound = ref(false);
/** 404 on the lookup or 422 `token` on accept: used, replaced, expired or mangled. */
const invalid = computed(() => !token || fieldErrors.value.token !== undefined || lookupNotFound.value);

onMounted(async () => {
  if (!token) return;
  try {
    info.value = await membersApi.inviteInfo(token);
  } catch (err) {
    if (err instanceof ApiError && err.status === 404) lookupNotFound.value = true;
    else showError(err);
  }
});

function validate() {
  const exists = info.value?.accountExists;
  return collectErrors({
    password: exists ? form.password === "" && "required" : passwordRule(form.password),
    displayName: !exists && textRule(form.displayName, { required: true, max: 100 }),
  });
}

async function onSubmit() {
  const exists = info.value?.accountExists;
  const ok = await submit(validate, () =>
    membersApi.accept({ token, password: form.password, ...(exists ? {} : { displayName: form.displayName.trim() }) }),
  );
  if (!ok) return;
  // The accept set this host's session cookie; pick up the user and the new role.
  await session.loadMe();
  await router.replace({ name: "home" });
}
</script>

<template>
  <AuthCard :title="t('members.accept.title')">
    <template v-if="invalid">
      <p role="alert" class="alert-error" data-test="accept-invalid">{{ t("members.accept.invalid") }}</p>
      <a :href="session.baseUrl" class="btn w-full" data-test="accept-base-link">{{ t("members.accept.toBase") }}</a>
    </template>
    <p v-else-if="!info && !error" class="text-sm text-gray-500">{{ t("common.loading") }}</p>
    <p v-else-if="!info" role="alert" class="alert-error" data-test="accept-error">{{ error }}</p>
    <form v-else class="space-y-4" novalidate @submit.prevent="onSubmit">
      <p class="text-sm" data-test="accept-intro">
        {{ t("members.accept.intro", { name: info.space.name, role: t(`spaces.roles.${info.role}`) }) }}
      </p>
      <p class="text-sm text-gray-600 dark:text-gray-400">
        {{ info.accountExists ? t("members.accept.existing") : t("members.accept.new") }}
      </p>
      <FormField :label="t('auth.email')" for="accept-email">
        <input id="accept-email" :value="info.email" type="email" autocomplete="username" readonly class="input" data-test="accept-email" />
      </FormField>
      <FormField v-if="!info.accountExists" :label="t('auth.register.displayName')" for="accept-name" :error="fieldErrors.displayName">
        <input id="accept-name" v-model="form.displayName" autocomplete="name" maxlength="100" class="input" :class="{ 'input-error': fieldErrors.displayName }" data-test="accept-name" />
      </FormField>
      <FormField
        :label="t('auth.password')"
        for="accept-password"
        :error="passwordFieldReason(fieldErrors.password)"
        :hint="info.accountExists ? undefined : t('auth.passwordHint')"
      >
        <input
          id="accept-password"
          v-model="form.password"
          type="password"
          :autocomplete="info.accountExists ? 'current-password' : 'new-password'"
          class="input"
          :class="{ 'input-error': fieldErrors.password }"
          data-test="accept-password"
        />
      </FormField>
      <p v-if="error" role="alert" class="alert-error" data-test="accept-error">{{ error }}</p>
      <button type="submit" class="btn btn-primary w-full" :disabled="submitting" data-test="accept-submit">
        {{ submitting ? t("members.accept.submitting") : info.accountExists ? t("members.accept.submitExisting") : t("members.accept.submitNew") }}
      </button>
    </form>
  </AuthCard>
</template>
