import { defineStore } from "pinia";
import { ref } from "vue";
import { tokensApi } from "./api";
import type { ApiToken, CreatedToken, CreateTokenBody } from "./types";

export const useTokensStore = defineStore("tokens", () => {
  const items = ref<ApiToken[]>([]);
  const loaded = ref(false);

  async function load(): Promise<void> {
    items.value = await tokensApi.list();
    loaded.value = true;
  }

  /** The returned secret is shown once by the caller and never kept in the store. */
  async function create(body: CreateTokenBody): Promise<CreatedToken> {
    const created = await tokensApi.create(body);
    const { token: _secret, ...row } = created;
    items.value = [row, ...items.value];
    return created;
  }

  async function revoke(id: string): Promise<void> {
    await tokensApi.revoke(id);
    items.value = items.value.filter((t) => t.id !== id);
  }

  return { items, loaded, load, create, revoke };
});
