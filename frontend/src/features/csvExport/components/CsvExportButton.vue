<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { useErrorText } from "@/composables/useAction";
import type { DocumentFilterQuery } from "@/features/documents/api";
import { todayIso } from "@/features/documents/form";
import { downloadPdf } from "@/lib/pdf";
import { csvExportApi } from "../api";
import { exportErrorKey } from "../period";

/** "Export CSV" on a document list: the current tab + filters (no paging), drafts and cancelled left out by the server. */
const props = defineProps<{ query: () => DocumentFilterQuery }>();

const { t } = useI18n();
const errorText = useErrorText();
const busy = ref(false);
const error = ref<string | null>(null);

async function run() {
  if (busy.value) return;
  busy.value = true;
  error.value = null;
  const query = props.query();
  try {
    await downloadPdf(() => csvExportApi.list(query), `doklady-${query.direction ?? "issued"}-${todayIso()}.csv`);
  } catch (err) {
    const key = exportErrorKey(err);
    error.value = key ? t(key) : errorText(err);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <button type="button" class="btn" :disabled="busy" :title="t('csvExport.list.hint')" data-test="csv-export" @click="run">
    {{ busy ? t("csvExport.preparing") : t("csvExport.list.action") }}
  </button>
  <p v-if="error" role="alert" class="alert-error w-full" data-test="csv-export-error">{{ error }}</p>
</template>
