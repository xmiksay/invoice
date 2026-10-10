import { defineStore } from "pinia";
import { ref } from "vue";
import { spacesApi } from "./api";
import type { CreateSpaceBody, Space } from "./types";

/** The base host's "Moje spaces" list. */
export const useSpacesStore = defineStore("spaces", () => {
  const items = ref<Space[]>([]);
  const loaded = ref(false);

  async function load(): Promise<void> {
    items.value = await spacesApi.list();
    loaded.value = true;
  }

  async function create(body: CreateSpaceBody): Promise<Space> {
    const space = await spacesApi.create(body);
    // Same order as the server: by name.
    items.value = [...items.value, space].sort((a, b) => a.name.localeCompare(b.name));
    return space;
  }

  return { items, loaded, load, create };
});
