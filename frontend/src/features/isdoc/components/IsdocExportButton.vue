<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { useErrorText } from "@/composables/useAction";
import type { DocumentFilterQuery } from "@/features/documents/api";
import { downloadPdf } from "@/lib/pdf";
import { isdocApi } from "../api";
import { exportErrorKey } from "../upload";

/** "Export ISDOC" on the issued list: one ZIP of every non-draft document matching `query`. */
const props = defineProps<{ query: () => DocumentFilterQuery }>();

const { t } = useI18n();
const errorText = useErrorText();
const busy = ref(false);
const error = ref<string | null>(null);

async function run() {
  if (busy.value) return;
  busy.value = true;
  error.value = null;
  try {
    // The blob download helper is file-type agnostic despite its name.
    await downloadPdf(() => isdocApi.bulk(props.query()), "isdoc-export.zip");
  } catch (err) {
    const key = exportErrorKey(err);
    error.value = key ? t(key) : errorText(err);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <button type="button" class="btn" :disabled="busy" :title="t('isdoc.export.hint')" data-test="isdoc-export" @click="run">
    {{ busy ? t("isdoc.export.preparing") : t("isdoc.export.list") }}
  </button>
  <p v-if="error" role="alert" class="alert-error w-full" data-test="isdoc-export-error">{{ error }}</p>
</template>
