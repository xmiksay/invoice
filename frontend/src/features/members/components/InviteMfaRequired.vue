<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { useSessionStore } from "@/stores/session";

/**
 * 403 `mfa_required` on accept: the space requires TOTP the account does not have. `created` = the
 * account was just created (no membership, no session). Either way the invitation stays valid.
 */
const props = defineProps<{ outcome: "existing" | "created"; space: string }>();
const { t } = useI18n();
const session = useSessionStore();
const accountUrl = computed(() => `${session.baseUrl.replace(/\/$/, "")}/account`);
const steps = computed(() => (props.outcome === "created" ? ["login", "enable", "reopen"] : ["enable", "reopen"]));
</script>

<template>
  <div class="space-y-3" :data-test="`accept-mfa-${outcome}`">
    <p role="alert" class="alert-error">{{ t(`members.accept.mfa.${outcome}`, { name: space }) }}</p>
    <ol class="list-decimal space-y-1 pl-5 text-sm">
      <li v-for="step in steps" :key="step">{{ t(`members.accept.mfa.steps.${step}`) }}</li>
    </ol>
    <a :href="accountUrl" class="btn btn-primary w-full" data-test="accept-mfa-link">{{ t("members.accept.mfa.link") }}</a>
  </div>
</template>
