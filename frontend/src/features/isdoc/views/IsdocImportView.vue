<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRoute } from "vue-router";
import { useErrorText } from "@/composables/useAction";
import { formatBytes } from "@/lib/bytes";
import IsdocBatchOptions from "../components/IsdocBatchOptions.vue";
import IsdocDropZone from "../components/IsdocDropZone.vue";
import IsdocPreviewTable from "../components/IsdocPreviewTable.vue";
import IsdocResultTable from "../components/IsdocResultTable.vue";
import { useIsdocImportStore } from "../store";
import { MAX_UPLOAD_BYTES, partitionFiles, totalBytes, uploadErrorKey } from "../upload";

const { t, locale } = useI18n();
const route = useRoute();
const store = useIsdocImportStore();
const errorText = useErrorText();

// Every visit starts a fresh import; the results of an earlier one are not kept around.
store.reset();

const busy = ref(false);
const error = ref<string | null>(null);
const ignored = ref<string[]>([]);

/** Back to the list the page was opened from (`?from=received`). */
const backTo = computed(() => ({ name: route.query.from === "received" ? "received" : "invoices" }));
const selectedCount = computed(() => store.entries?.filter(store.isSelected).length ?? 0);

async function act(action: () => Promise<void>) {
  busy.value = true;
  error.value = null;
  try {
    await action();
  } catch (err) {
    const key = uploadErrorKey(err);
    error.value = key ? t(key) : errorText(err);
  } finally {
    busy.value = false;
  }
}

function onFiles(picked: File[]) {
  const { files, ignored: rest } = partitionFiles(picked);
  ignored.value = rest;
  error.value = null;
  if (files.length === 0) {
    store.reset();
    error.value = t("isdoc.upload.noFiles");
    return;
  }
  if (totalBytes(files) > MAX_UPLOAD_BYTES) {
    store.reset();
    error.value = t("isdoc.upload.tooLarge");
    return;
  }
  void act(() => store.preview(files));
}

const confirm = () => void act(store.confirm);

function restart() {
  store.reset();
  ignored.value = [];
  error.value = null;
}
</script>

<template>
  <section class="space-y-4">
    <RouterLink :to="backTo" class="text-sm text-blue-600 hover:underline dark:text-blue-400" data-test="isdoc-back">
      ← {{ t("isdoc.import.back") }}
    </RouterLink>
    <h1 class="text-2xl font-semibold">{{ t("isdoc.import.title") }}</h1>

    <template v-if="!store.results">
      <IsdocDropZone :disabled="busy" @files="onFiles" />
      <p v-if="store.files.length" class="text-sm text-gray-600 dark:text-gray-400" data-test="isdoc-files">
        {{ t("isdoc.import.files", { names: store.files.map((f) => f.name).join(", "), size: formatBytes(totalBytes(store.files), locale) }) }}
      </p>
      <p v-if="ignored.length" class="text-sm text-amber-700 dark:text-amber-400" data-test="isdoc-ignored">
        {{ t("isdoc.import.ignored", { names: ignored.join(", ") }) }}
      </p>
    </template>

    <p v-if="error" role="alert" class="alert-error" data-test="isdoc-error-alert">{{ error }}</p>
    <p v-if="busy" class="text-sm text-gray-500">{{ store.entries ? t("isdoc.import.importing") : t("isdoc.import.reading") }}</p>

    <template v-if="store.results">
      <p class="text-sm" data-test="isdoc-summary">
        {{ t("isdoc.result.summary", { imported: store.results.filter((r) => r.status === "imported").length, total: store.results.length }) }}
      </p>
      <IsdocResultTable />
      <button type="button" class="btn" data-test="isdoc-restart" @click="restart">{{ t("isdoc.result.again") }}</button>
    </template>

    <template v-else-if="store.entries">
      <IsdocPreviewTable />
      <IsdocBatchOptions />
      <div class="flex flex-wrap items-center gap-3">
        <button type="button" class="btn btn-primary" :disabled="busy || selectedCount === 0" data-test="isdoc-confirm" @click="confirm">
          {{ t("isdoc.import.confirm", { n: selectedCount }) }}
        </button>
        <span class="text-sm text-gray-500">{{ t("isdoc.import.selected", { n: selectedCount, total: store.entries.length }) }}</span>
      </div>
    </template>
  </section>
</template>
