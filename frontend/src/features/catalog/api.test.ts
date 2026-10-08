import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { mockFetch, sentRequest } from "@/test-utils";
import { catalogGroupsApi, catalogItemsApi } from "./api";
import type { CatalogGroupInput, CatalogItemInput } from "./types";

describe("catalog api", () => {
  beforeEach(() => setActivePinia(createPinia()));
  afterEach(() => vi.unstubAllGlobals());

  it("lists items with an optional trimmed q and the active filter", async () => {
    let fetch = mockFetch(200, []);
    await catalogItemsApi.list();
    expect(sentRequest(fetch).url).toBe("/api/catalog/items");

    fetch = mockFetch(200, []);
    await catalogItemsApi.list(" host ", true);
    expect(sentRequest(fetch).url).toBe("/api/catalog/items?q=host&active=true");
  });

  it("item CRUD routes", async () => {
    const input = { name: "A" } as CatalogItemInput;
    let fetch = mockFetch(201, {});
    await catalogItemsApi.create(input);
    expect(sentRequest(fetch)).toEqual({ url: "/api/catalog/items", method: "POST", body: input });
    fetch = mockFetch(200, {});
    await catalogItemsApi.update("i 1", input);
    expect(sentRequest(fetch)).toEqual({ url: "/api/catalog/items/i%201", method: "PUT", body: input });
    fetch = mockFetch(204);
    await catalogItemsApi.remove("i1");
    expect(sentRequest(fetch)).toMatchObject({ url: "/api/catalog/items/i1", method: "DELETE" });
  });

  it("group routes", async () => {
    const input: CatalogGroupInput = { name: "G", collapse: true, members: [{ itemId: "i1", quantity: "2" }] };
    let fetch = mockFetch(200, []);
    await catalogGroupsApi.list("web");
    expect(sentRequest(fetch).url).toBe("/api/catalog/groups?q=web");
    fetch = mockFetch(201, {});
    await catalogGroupsApi.create(input);
    expect(sentRequest(fetch)).toEqual({ url: "/api/catalog/groups", method: "POST", body: input });
    fetch = mockFetch(200, {});
    await catalogGroupsApi.update("g1", input);
    expect(sentRequest(fetch)).toEqual({ url: "/api/catalog/groups/g1", method: "PUT", body: input });
    fetch = mockFetch(204);
    await catalogGroupsApi.remove("g1");
    expect(sentRequest(fetch)).toMatchObject({ url: "/api/catalog/groups/g1", method: "DELETE" });
  });
});
