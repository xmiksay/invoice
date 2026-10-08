import { defineStore } from "pinia";
import { ref, type Ref } from "vue";
import { catalogGroupsApi, catalogItemsApi } from "./api";
import type { CatalogGroup, CatalogGroupInput, CatalogItem, CatalogItemInput } from "./types";

interface ListApi<T, I> {
  list(q?: string): Promise<T[]>;
  create(input: I): Promise<T>;
  update(id: string, input: I): Promise<T>;
  remove(id: string): Promise<void>;
}

/**
 * Catalog list with a search term. Every mutation reloads: deleting an item
 * also drops it from groups, and the server owns the order.
 */
function defineCatalogStore<T extends { id: string }, I>(id: string, api: ListApi<T, I>) {
  return defineStore(id, () => {
    const items = ref([]) as Ref<T[]>;
    const q = ref("");
    const loaded = ref(false);
    let seq = 0;

    async function load(): Promise<void> {
      const mine = ++seq;
      const list = await api.list(q.value);
      if (mine !== seq) return;
      items.value = list;
      loaded.value = true;
    }

    async function search(term: string): Promise<void> {
      q.value = term;
      await load();
    }

    async function save(input: I, itemId?: string): Promise<void> {
      if (itemId) await api.update(itemId, input);
      else await api.create(input);
      await load();
    }

    async function remove(itemId: string): Promise<void> {
      await api.remove(itemId);
      await load();
    }

    return { items, q, loaded, load, search, save, remove };
  });
}

export const useCatalogItemsStore = defineCatalogStore<CatalogItem, CatalogItemInput>("catalog/items", catalogItemsApi);
export const useCatalogGroupsStore = defineCatalogStore<CatalogGroup, CatalogGroupInput>("catalog/groups", catalogGroupsApi);
