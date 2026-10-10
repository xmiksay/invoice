<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useFormSubmit } from "@/composables/useFormSubmit";
import CodeField from "@/features/auth/components/CodeField.vue";
import { codeRule } from "@/features/auth/validation";
import { membersApi } from "@/features/members/api";
import type { Member } from "@/features/members/types";
import { collectErrors } from "@/lib/formErrors";
import { useSessionStore } from "@/stores/session";
import { spacesApi } from "../api";
import type { Space } from "../types";

/**
 * Owner's "require TOTP" policy. Either direction is confirmed with the owner's own code (step-up);
 * turning it on also lists the members who lack TOTP.
 */
const props = defineProps<{ space: Space }>();
const emit = defineEmits<{ updated: [space: Space] }>();
const { t } = useI18n();
const session = useSessionStore();
const { fieldErrors, error, submitting, submit, showError } = useFormSubmit();
/** The requested state while the confirmation is open, else null. */
const pending = ref<boolean | null>(null);
/** Who will be refused at the next login (only when turning it on). */
const lockedOut = ref<Member[]>([]);
const code = ref("");
const loading = ref(false);

// Without the owner's own TOTP there is no code to confirm with (server: 422 / step-up).
const blocked = computed(() => !session.mfaEnabled);

async function onToggle(event: Event) {
  const box = event.target as HTMLInputElement;
  // The box mirrors the saved state; it only moves once the server agreed.
  box.checked = props.space.requireMfa;
  const next = !props.space.requireMfa;
  error.value = null;
  fieldErrors.value = {};
  code.value = "";
  lockedOut.value = [];
  if (next) {
    loading.value = true;
    try {
      lockedOut.value = (await membersApi.list()).filter((m) => !m.mfaEnabled);
    } catch (err) {
      showError(err);
      return;
    } finally {
      loading.value = false;
    }
  }
  pending.value = next;
}

async function confirm() {
  if (pending.value === null) return;
  const requireMfa = pending.value;
  await submit(
    () => collectErrors({ code: codeRule(code.value) }),
    async () => {
      emit("updated", await spacesApi.update({ requireMfa, code: code.value.trim() }));
      pending.value = null;
    },
  );
}
</script>

<template>
  <section class="card space-y-3" data-test="require-mfa">
    <h2 class="text-lg font-semibold">{{ t("spaces.mfa.title") }}</h2>
    <label class="flex items-center gap-2 text-sm font-medium">
      <input type="checkbox" :checked="space.requireMfa" :disabled="blocked || submitting || loading || pending !== null" data-test="require-mfa-toggle" @change="onToggle" />
      {{ t("spaces.mfa.toggle") }}
    </label>
    <p class="text-sm text-gray-600 dark:text-gray-400">{{ t("spaces.mfa.intro") }}</p>
    <p v-if="blocked" class="text-sm text-amber-700 dark:text-amber-400" data-test="require-mfa-blocked">
      {{ t("spaces.mfa.ownerFirst") }}
      <RouterLink :to="{ name: 'account' }" class="font-medium underline">{{ t("spaces.mfa.toAccount") }}</RouterLink>
    </p>
    <form v-if="pending !== null" class="space-y-3 rounded-md border border-amber-300 p-3 dark:border-amber-800" novalidate data-test="require-mfa-confirm" @submit.prevent="confirm">
      <template v-if="pending">
        <template v-if="lockedOut.length">
          <p class="text-sm">{{ t("spaces.mfa.lockedOut") }}</p>
          <ul class="list-disc pl-5 text-sm">
            <li v-for="m in lockedOut" :key="m.userId" data-test="require-mfa-member">{{ m.displayName }} ({{ m.email }})</li>
          </ul>
        </template>
        <p v-else class="text-sm" data-test="require-mfa-all-set">{{ t("spaces.mfa.allSet") }}</p>
      </template>
      <p v-else class="text-sm" data-test="require-mfa-disable-intro">{{ t("spaces.mfa.disableIntro") }}</p>
      <CodeField id="require-mfa-code" v-model="code" :error="fieldErrors.code" />
      <div class="flex gap-2">
        <button type="submit" class="btn btn-primary" :disabled="submitting" data-test="require-mfa-submit">
          {{ pending ? t("spaces.mfa.enable") : t("spaces.mfa.disable") }}
        </button>
        <button type="button" class="btn" data-test="require-mfa-cancel" @click="pending = null">{{ t("common.cancel") }}</button>
      </div>
    </form>
    <p v-if="fieldErrors.requireMfa === 'mfa_not_enabled'" role="alert" class="alert-error" data-test="require-mfa-error">{{ t("spaces.mfa.ownerFirst") }}</p>
    <p v-else-if="error" role="alert" class="alert-error" data-test="require-mfa-error">{{ error }}</p>
  </section>
</template>
