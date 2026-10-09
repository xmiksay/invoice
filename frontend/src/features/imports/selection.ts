import { computed, ref, type Ref } from "vue";
import type { PreviewEntry } from "./types";

type Selectable = Pick<PreviewEntry, "key" | "status">;

/** Only `ok` entries can be imported; they start selected. */
export const defaultSelection = (entries: Selectable[]): string[] => entries.filter((e) => e.status === "ok").map((e) => e.key);

/** The preview selection (keys echoed back in `confirm`), for use inside an import store. */
export function useSelection(entries: Ref<Selectable[] | null>) {
  const selected = ref<string[]>([]);
  const okKeys = computed(() => defaultSelection(entries.value ?? []));
  const allSelected = computed(() => okKeys.value.length > 0 && okKeys.value.every((k) => selected.value.includes(k)));
  const isSelected = (e: Pick<Selectable, "key">) => selected.value.includes(e.key);
  const selectedCount = computed(() => (entries.value ?? []).filter(isSelected).length);

  function toggle(key: string, on: boolean): void {
    selected.value = on ? [...selected.value, key] : selected.value.filter((k) => k !== key);
  }

  function toggleAll(on: boolean): void {
    selected.value = on ? [...okKeys.value] : [];
  }

  return { selected, okKeys, allSelected, selectedCount, isSelected, toggle, toggleAll };
}
