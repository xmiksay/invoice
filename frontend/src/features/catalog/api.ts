import { request } from "@/api/client";
import type { CatalogGroup, CatalogGroupInput, CatalogItem, CatalogItemInput } from "./types";

const BASE = "/api/catalog";
const at = (path: string, id: string) => `${BASE}/${path}/${encodeURIComponent(id)}`;

function query(params: Record<string, string | undefined>): string {
  const search = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) if (value) search.set(key, value);
  const s = search.toString();
  return s ? `?${s}` : "";
}

export const catalogItemsApi = {
  /** `activeOnly` hides retired items (the document picker); the catalog page lists all. */
  list: (q = "", activeOnly = false) =>
    request<CatalogItem[]>(`${BASE}/items${query({ q: q.trim() || undefined, active: activeOnly ? "true" : undefined })}`),
  create: (input: CatalogItemInput) => request<CatalogItem>(`${BASE}/items`, { method: "POST", body: input }),
  update: (id: string, input: CatalogItemInput) => request<CatalogItem>(at("items", id), { method: "PUT", body: input }),
  remove: (id: string) => request<void>(at("items", id), { method: "DELETE" }),
};

export const catalogGroupsApi = {
  list: (q = "") => request<CatalogGroup[]>(`${BASE}/groups${query({ q: q.trim() || undefined })}`),
  create: (input: CatalogGroupInput) => request<CatalogGroup>(`${BASE}/groups`, { method: "POST", body: input }),
  update: (id: string, input: CatalogGroupInput) => request<CatalogGroup>(at("groups", id), { method: "PUT", body: input }),
  remove: (id: string) => request<void>(at("groups", id), { method: "DELETE" }),
};
