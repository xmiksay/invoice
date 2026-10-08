<script setup lang="ts">
import { onMounted } from "vue";
import { useI18n } from "vue-i18n";
import type { DocLocale } from "@/api/types";
import ErrorDetail from "@/components/ErrorDetail.vue";
import { useAction } from "@/composables/useAction";
import { usePdf } from "@/composables/usePdf";
import { formatBytes } from "@/lib/bytes";
import { pdfApi } from "../api";
import { useDesignStore } from "../stores";

const { t, locale } = useI18n();
const store = useDesignStore();
const { error: loadError, run } = useAction();
const { busy, error: previewError, detail, open } = usePdf();

onMounted(() => void run(store.load));

const preview = (l: DocLocale) => open(() => pdfApi.preview(l));

const previews: { locale: DocLocale; label: string }[] = [
  { locale: "cs", label: "pdf.design.previewCs" },
  { locale: "en", label: "pdf.design.previewEn" },
];
</script>

<template>
  <div class="space-y-4">
    <p v-if="loadError" role="alert" class="alert-error">{{ loadError }}</p>

    <section class="card space-y-3">
      <p class="text-sm text-gray-700 dark:text-gray-300" data-test="design-help">{{ t("pdf.design.help") }}</p>
      <dl v-if="store.design" class="flex flex-wrap gap-x-2 text-sm">
        <dt class="text-gray-600 dark:text-gray-400">{{ t("pdf.design.dir") }}:</dt>
        <dd data-test="design-dir" :class="store.design.designDir ? 'font-mono break-all' : 'italic'">
          {{ store.design.designDir ?? t("pdf.design.builtIn") }}
        </dd>
      </dl>
      <div class="flex flex-wrap items-center gap-2">
        <button
          v-for="p in previews"
          :key="p.locale"
          type="button"
          class="btn"
          :disabled="busy"
          :data-test="`design-preview-${p.locale}`"
          @click="preview(p.locale)"
        >
          {{ t(p.label) }}
        </button>
        <span class="text-sm text-gray-500">{{ busy ? t("pdf.loading") : t("pdf.design.previewHint") }}</span>
      </div>
      <div v-if="previewError" role="alert" class="alert-error" data-test="design-preview-error">
        <p>{{ previewError }}</p>
        <ErrorDetail :detail="detail" open />
      </div>
    </section>

    <div v-if="store.design" class="card overflow-x-auto !p-0">
      <table class="table">
        <caption class="px-3 pt-3 text-left text-sm font-medium">{{ t("pdf.design.files") }}</caption>
        <thead>
          <tr>
            <th>{{ t("pdf.design.path") }}</th>
            <th>{{ t("pdf.design.source") }}</th>
            <th class="text-right">{{ t("pdf.design.size") }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="f in store.design.files" :key="f.path" data-test="design-file">
            <td class="font-mono">{{ f.path }}</td>
            <td>
              <span
                class="badge"
                :class="{ '!bg-gray-100 !text-gray-700 dark:!bg-gray-800 dark:!text-gray-300': f.source === 'default' }"
                :data-test="`source-${f.source}`"
              >
                {{ t(`pdf.design.sources.${f.source}`) }}
              </span>
            </td>
            <td class="text-right whitespace-nowrap">{{ formatBytes(f.size, locale) }}</td>
          </tr>
          <tr v-if="store.design.files.length === 0">
            <td colspan="3" class="py-8 text-center text-gray-500">{{ t("pdf.design.empty") }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
