<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { SentInvite } from "../types";

const props = defineProps<{ sent: SentInvite }>();
const { t } = useI18n();
const copied = ref(false);
watch(() => props.sent.url, () => (copied.value = false));

async function copy() {
  try {
    await navigator.clipboard.writeText(props.sent.url);
    copied.value = true;
  } catch {
    // Clipboard blocked (permissions, insecure context): the link stays selectable in the field.
  }
}
</script>

<template>
  <div class="space-y-2 rounded-md border border-gray-200 p-3 dark:border-gray-800" data-test="invite-link">
    <label for="invite-url" class="block text-sm font-medium">{{ t("members.link.label", { email: sent.email }) }}</label>
    <div class="flex gap-2">
      <input id="invite-url" :value="sent.url" readonly class="input font-mono text-xs" data-test="invite-url" @focus="($event.target as HTMLInputElement).select()" />
      <button type="button" class="btn" data-test="invite-copy" @click="copy">{{ t("members.link.copy") }}</button>
    </div>
    <p v-if="copied" role="status" class="text-sm text-green-700 dark:text-green-400">{{ t("members.link.copied") }}</p>
    <p v-if="sent.emailSent" class="text-sm text-green-700 dark:text-green-400" data-test="invite-email-status">{{ t("members.link.emailSent") }}</p>
    <p v-else class="rounded-md bg-amber-50 px-3 py-2 text-sm text-amber-800 dark:bg-amber-950/50 dark:text-amber-300" data-test="invite-email-status">
      {{ t("members.link.emailNotSent") }}
    </p>
    <p class="text-xs text-gray-500 dark:text-gray-400">{{ t("members.link.hint") }}</p>
  </div>
</template>
