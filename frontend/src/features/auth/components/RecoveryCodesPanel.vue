<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { saveBlob } from "@/lib/pdf";
import { useSessionStore } from "@/stores/session";

/** The recovery codes, shown once; closing needs the "I have saved them" acknowledgement. */
const props = defineProps<{ codes: string[] }>();
const emit = defineEmits<{ close: [] }>();
const { t } = useI18n();
const session = useSessionStore();
const saved = ref(false);
const copied = ref(false);

const text = () => `${t("mfa.codes.fileHeader", { email: session.me?.user.email ?? "" })}\n\n${props.codes.join("\n")}\n`;

async function copy() {
  try {
    await navigator.clipboard.writeText(props.codes.join("\n"));
    copied.value = true;
  } catch {
    // Clipboard blocked (permissions, insecure context): the codes stay selectable, or download them.
  }
}

function download() {
  saveBlob(new Blob([text()], { type: "text/plain;charset=utf-8" }), "invoice-recovery-codes.txt");
}
</script>

<template>
  <div class="space-y-3" data-test="recovery-codes">
    <h3 class="font-semibold">{{ t("mfa.codes.title") }}</h3>
    <p class="rounded-md bg-amber-50 px-3 py-2 text-sm text-amber-800 dark:bg-amber-950/50 dark:text-amber-300">{{ t("mfa.codes.warning") }}</p>
    <ul class="grid grid-cols-2 gap-x-6 gap-y-1 rounded-md bg-gray-50 p-3 font-mono text-sm select-all dark:bg-gray-950">
      <li v-for="code in codes" :key="code" data-test="recovery-code">{{ code }}</li>
    </ul>
    <div class="flex flex-wrap gap-2">
      <button type="button" class="btn" data-test="recovery-copy" @click="copy">{{ t("mfa.codes.copy") }}</button>
      <button type="button" class="btn" data-test="recovery-download" @click="download">{{ t("mfa.codes.download") }}</button>
    </div>
    <p v-if="copied" role="status" class="text-sm text-green-700 dark:text-green-400">{{ t("mfa.codes.copied") }}</p>
    <label class="flex items-center gap-2 text-sm">
      <input v-model="saved" type="checkbox" data-test="recovery-saved" />
      {{ t("mfa.codes.saved") }}
    </label>
    <button type="button" class="btn btn-primary" :disabled="!saved" data-test="recovery-close" @click="emit('close')">{{ t("mfa.codes.close") }}</button>
  </div>
</template>
