import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { isdocApi } from "./api";
import type { ConfirmOptions, ConfirmResult, PreviewEntry } from "./types";
import { defaultSelection } from "./upload";

export type BatchOptions = Omit<ConfirmOptions, "selected">;

const defaultOptions = (): BatchOptions => ({ markPaid: true, categoryId: null, vatDeductible: true });

/**
 * One ISDOC import: the picked files, their preview, the selection + batch options and the result.
 * The server keeps nothing between preview and confirm, so the same `File`s are uploaded twice.
 */
export const useIsdocImportStore = defineStore("isdoc/import", () => {
  const files = ref<File[]>([]);
  const entries = ref<PreviewEntry[] | null>(null);
  const selected = ref<string[]>([]);
  const options = ref<BatchOptions>(defaultOptions());
  const results = ref<ConfirmResult[] | null>(null);

  const isSelected = (e: PreviewEntry) => selected.value.includes(e.key);
  /** The category / VAT deductible options apply (and are shown) only when a received entry is selected. */
  const receivedSelected = computed(() => !!entries.value?.some((e) => e.direction === "received" && isSelected(e)));

  function toggle(key: string, on: boolean): void {
    selected.value = on ? [...selected.value, key] : selected.value.filter((k) => k !== key);
  }

  function reset(): void {
    files.value = [];
    entries.value = null;
    selected.value = [];
    options.value = defaultOptions();
    results.value = null;
  }

  async function preview(next: File[]): Promise<void> {
    reset();
    files.value = next;
    const res = await isdocApi.preview(next);
    entries.value = res.entries;
    selected.value = defaultSelection(res.entries);
  }

  async function confirm(): Promise<void> {
    const { markPaid, categoryId, vatDeductible } = options.value;
    const res = await isdocApi.confirm(files.value, {
      selected: selected.value,
      markPaid,
      categoryId: receivedSelected.value ? categoryId : null,
      vatDeductible: receivedSelected.value ? vatDeductible : true,
    });
    results.value = res.results;
  }

  return { files, entries, selected, options, results, receivedSelected, isSelected, toggle, reset, preview, confirm };
});
