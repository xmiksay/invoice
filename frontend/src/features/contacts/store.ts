import { defineStore } from "pinia";
import { ref } from "vue";
import { contactsApi } from "./api";
import type { Contact, ContactInput } from "./types";

export const PAGE_SIZE = 50;

/** Contact list state; kept in the store so search + page survive a trip to the edit form. */
export const useContactsStore = defineStore("contacts", () => {
  const items = ref<Contact[]>([]);
  const total = ref(0);
  const q = ref("");
  const offset = ref(0);
  const loading = ref(false);
  // Drops responses that arrive after a newer search was started.
  let seq = 0;

  async function load(): Promise<void> {
    const mine = ++seq;
    loading.value = true;
    try {
      const page = await contactsApi.list({ q: q.value, limit: PAGE_SIZE, offset: offset.value });
      if (mine !== seq) return;
      items.value = page.items;
      total.value = page.total;
    } finally {
      if (mine === seq) loading.value = false;
    }
  }

  async function search(text: string): Promise<void> {
    q.value = text;
    offset.value = 0;
    await load();
  }

  async function goTo(newOffset: number): Promise<void> {
    offset.value = Math.max(0, newOffset);
    await load();
  }

  const get = (id: string) => contactsApi.get(id);

  async function save(input: ContactInput, id?: string): Promise<Contact> {
    return id ? contactsApi.update(id, input) : contactsApi.create(input);
  }

  async function remove(id: string): Promise<void> {
    await contactsApi.remove(id);
    items.value = items.value.filter((c) => c.id !== id);
    total.value = Math.max(0, total.value - 1);
  }

  return { items, total, q, offset, loading, load, search, goTo, get, save, remove };
});
