<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { usePdf } from "@/composables/usePdf";
import ImportConfirmBar from "@/features/imports/components/ImportConfirmBar.vue";
import ImportDropZone from "@/features/imports/components/ImportDropZone.vue";
import ImportResultTable from "@/features/imports/components/ImportResultTable.vue";
import { MAX_UPLOAD_BYTES } from "@/features/imports/upload";
import { useImportPage } from "@/features/imports/useImportPage";
import { formatBytes } from "@/lib/bytes";
import { csvImportApi } from "../api";
import { ACCEPT_ATTR, pickFile, rowErrorKey, SAMPLE_NAME, uploadError } from "../codes";
import CsvPreviewTable from "../components/CsvPreviewTable.vue";
import { useCsvImportStore } from "../store";

const { t, locale } = useI18n();
const store = useCsvImportStore();
const { busy, error, backTo, act } = useImportPage((err) => {
  const found = uploadError(err);
  return found ? t(found.key, found.params) : null;
});
// The download plumbing is file-type agnostic: blob + Content-Disposition filename.
const sample = usePdf();

// Every visit starts a fresh import; the results of an earlier one are not kept around.
store.reset();

const resultError = (code: string) => t(rowErrorKey(code), { code, number: "" });
const rowLabel = (key: string) => t("csvImport.preview.rowNo", { n: key.replace(/^row:/, "") });

function onFiles(picked: File[]) {
  const file = pickFile(picked);
  error.value = null;
  if (!file) {
    store.reset();
    error.value = t("csvImport.upload.noFile");
    return;
  }
  if (file.size > MAX_UPLOAD_BYTES) {
    store.reset();
    error.value = t("csvImport.upload.tooLarge");
    return;
  }
  void act(() => store.preview(file));
}

const confirm = () => void act(store.confirm);
const downloadSample = () => void sample.download(csvImportApi.sample, SAMPLE_NAME);

function restart() {
  store.reset();
  error.value = null;
}
</script>

<template>
  <section class="space-y-4">
    <RouterLink :to="backTo" class="text-sm text-blue-600 hover:underline dark:text-blue-400" data-test="import-back">
      ← {{ t("imports.page.back") }}
    </RouterLink>
    <h1 class="text-2xl font-semibold">{{ t("csvImport.import.title") }}</h1>

    <template v-if="!store.results">
      <ImportDropZone
        :accept="ACCEPT_ATTR"
        :disabled="busy"
        :hint="t('csvImport.import.dropHint')"
        :pick="t('csvImport.import.pick')"
        :limits="t('csvImport.import.limits')"
        @files="onFiles"
      >
        <template #actions>
          <button type="button" class="btn" :disabled="sample.busy.value" data-test="csv-sample" @click="downloadSample">
            {{ t("csvImport.import.sample") }}
          </button>
        </template>
      </ImportDropZone>
      <p v-if="sample.error.value" role="alert" class="alert-error" data-test="csv-sample-error">{{ sample.error.value }}</p>
      <p v-if="store.file" class="text-sm text-gray-600 dark:text-gray-400" data-test="csv-file">
        {{ t("csvImport.import.file", { name: store.file.name, size: formatBytes(store.file.size, locale) }) }}
      </p>
    </template>

    <p v-if="error" role="alert" class="alert-error" data-test="import-error-alert">{{ error }}</p>
    <p v-if="busy" class="text-sm text-gray-500">{{ store.entries ? t("imports.page.importing") : t("imports.page.reading") }}</p>

    <template v-if="store.results">
      <ImportResultTable :results="store.results" :entries="store.entries ?? []" :error-text="resultError" :key-label="rowLabel" />
      <button type="button" class="btn" data-test="import-restart" @click="restart">{{ t("csvImport.import.again") }}</button>
    </template>

    <template v-else-if="store.entries">
      <CsvPreviewTable />
      <ImportConfirmBar :count="store.selectedCount" :total="store.entries.length" :busy="busy" @confirm="confirm" />
    </template>
  </section>
</template>
