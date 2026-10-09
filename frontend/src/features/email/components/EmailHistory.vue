<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useAction } from "@/composables/useAction";
import { useEmailHistoryStore } from "../store";

const props = defineProps<{ documentId: string }>();

const { t, locale } = useI18n();
const history = useEmailHistoryStore();
const { error, run } = useAction();
const expanded = ref<string | null>(null);

watch(
  () => props.documentId,
  (id) => void run(() => history.load(id)),
  { immediate: true },
);

const toggle = (id: string) => (expanded.value = expanded.value === id ? null : id);
const formatTime = (iso: string) => new Intl.DateTimeFormat(locale.value, { dateStyle: "medium", timeStyle: "short" }).format(new Date(iso));
const list = (addresses: string[]) => addresses.join(", ");
</script>

<template>
  <section class="card space-y-3" data-test="email-history">
    <h2 class="text-lg font-semibold">{{ t("email.history.title") }}</h2>
    <p v-if="error" role="alert" class="alert-error">{{ error }}</p>
    <p v-else-if="history.entries.length === 0" class="text-sm text-gray-500" data-test="email-history-empty">{{ t("email.history.empty") }}</p>
    <div v-else class="overflow-x-auto">
      <table class="table">
        <thead>
          <tr>
            <th>{{ t("email.history.date") }}</th>
            <th>{{ t("email.history.recipients") }}</th>
            <th>{{ t("email.history.subject") }}</th>
            <th>{{ t("email.history.result") }}</th>
          </tr>
        </thead>
        <tbody>
          <template v-for="e in history.entries" :key="e.id">
            <tr data-test="email-entry">
              <td class="whitespace-nowrap">
                <button
                  type="button"
                  class="text-blue-600 hover:underline dark:text-blue-400"
                  :aria-expanded="expanded === e.id"
                  :aria-controls="`email-${e.id}`"
                  data-test="email-toggle"
                  @click="toggle(e.id)"
                >
                  {{ formatTime(e.createdAt) }}
                </button>
              </td>
              <td class="break-all">{{ list(e.to) }}</td>
              <td>{{ e.subject }}</td>
              <td>
                <span v-if="e.ok" class="badge !bg-green-100 !text-green-800 dark:!bg-green-900/50 dark:!text-green-300" data-test="email-ok">{{ t("email.history.ok") }}</span>
                <span v-else class="badge !bg-red-100 !text-red-800 dark:!bg-red-900/50 dark:!text-red-300" data-test="email-failed">{{ t("email.history.failed") }}</span>
              </td>
            </tr>
            <tr v-if="expanded === e.id" :id="`email-${e.id}`" data-test="email-detail">
              <td colspan="4" class="space-y-2 bg-gray-50 dark:bg-gray-950">
                <dl class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-sm">
                  <template v-if="e.cc.length">
                    <dt class="text-gray-600 dark:text-gray-400">{{ t("email.send.cc") }}</dt>
                    <dd class="break-all">{{ list(e.cc) }}</dd>
                  </template>
                  <template v-if="e.bcc.length">
                    <dt class="text-gray-600 dark:text-gray-400">{{ t("email.send.bcc") }}</dt>
                    <dd class="break-all">{{ list(e.bcc) }}</dd>
                  </template>
                  <dt class="text-gray-600 dark:text-gray-400">{{ t("email.send.attachments") }}</dt>
                  <dd class="font-mono" data-test="email-attachments">{{ e.attachments.length ? list(e.attachments) : t("email.history.noAttachments") }}</dd>
                  <template v-if="e.error">
                    <dt class="text-gray-600 dark:text-gray-400">{{ t("email.history.error") }}</dt>
                    <dd class="text-red-700 dark:text-red-400" data-test="email-entry-error">{{ e.error }}</dd>
                  </template>
                  <template v-if="e.messageId">
                    <dt class="text-gray-600 dark:text-gray-400">{{ t("email.history.messageId") }}</dt>
                    <dd class="font-mono break-all">{{ e.messageId }}</dd>
                  </template>
                </dl>
                <pre class="max-h-80 overflow-auto rounded border border-gray-200 bg-white p-2 text-xs whitespace-pre-wrap dark:border-gray-800 dark:bg-gray-900" data-test="email-entry-body">{{ e.body }}</pre>
              </td>
            </tr>
          </template>
        </tbody>
      </table>
    </div>
  </section>
</template>
