import { computed } from "vue";
import type { Direction } from "@/features/documents/types";
import { useCategoriesStore, useCustomFieldsStore } from "@/features/settings/stores";
import { applicableFields } from "./metadata";

/** Categories + the custom field definitions applicable to `direction`, loaded once per session. */
export function useMetadataSettings(direction: () => Direction) {
  const categories = useCategoriesStore();
  const customFields = useCustomFieldsStore();

  async function load(): Promise<void> {
    await Promise.all([categories.loaded ? undefined : categories.load(), customFields.loaded ? undefined : customFields.load()]);
  }

  return {
    categories: computed(() => categories.items),
    fieldDefs: computed(() => applicableFields(customFields.items, direction())),
    load,
  };
}
