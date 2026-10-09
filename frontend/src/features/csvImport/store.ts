import { defineStore } from "pinia";
import { ref } from "vue";
import { useSelection } from "@/features/imports/selection";
import type { ConfirmResult } from "@/features/imports/types";
import { csvImportApi } from "./api";
import type { CsvPreviewEntry } from "./types";

/**
 * One CSV / XLSX import: the picked file, its preview, the selection and the result.
 * The server keeps nothing between preview and confirm, so the same `File` is uploaded twice.
 */
export const useCsvImportStore = defineStore("csvImport", () => {
  const file = ref<File | null>(null);
  const entries = ref<CsvPreviewEntry[] | null>(null);
  const selection = useSelection(entries);
  const results = ref<ConfirmResult[] | null>(null);

  function reset(): void {
    file.value = null;
    entries.value = null;
    selection.selected.value = [];
    results.value = null;
  }

  async function preview(next: File): Promise<void> {
    reset();
    file.value = next;
    const res = await csvImportApi.preview(next);
    entries.value = res.entries;
    selection.toggleAll(true);
  }

  async function confirm(): Promise<void> {
    if (!file.value) return;
    const res = await csvImportApi.confirm(file.value, { selected: selection.selected.value });
    results.value = res.results;
  }

  return { file, entries, ...selection, results, reset, preview, confirm };
});
