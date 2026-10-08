import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { mockFetch, sentRequest } from "@/test-utils";
import { aresApi } from "@/api/ares";
import { contactsApi } from "./api";
import { toInput, toDraft } from "./form";

describe("contacts api", () => {
  beforeEach(() => setActivePinia(createPinia()));
  afterEach(() => vi.unstubAllGlobals());

  it("lists with trimmed q, limit and offset", async () => {
    const fetch = mockFetch(200, { items: [], total: 0 });
    await contactsApi.list({ q: " acme s.r.o. ", limit: 50, offset: 100 });
    expect(sentRequest(fetch).url).toBe("/api/contacts?q=acme+s.r.o.&limit=50&offset=100");
  });

  it("omits an empty q", async () => {
    const fetch = mockFetch(200, { items: [], total: 0 });
    await contactsApi.list({ q: "  ", limit: 50, offset: 0 });
    expect(sentRequest(fetch).url).toBe("/api/contacts?limit=50&offset=0");
  });

  it("get / create / update / delete", async () => {
    const input = toInput({ ...toDraft(), name: "Acme" });
    let fetch = mockFetch(200, {});
    await contactsApi.get("c1");
    expect(sentRequest(fetch)).toMatchObject({ url: "/api/contacts/c1", method: "GET" });

    fetch = mockFetch(200, {});
    await contactsApi.create(input);
    expect(sentRequest(fetch)).toEqual({ url: "/api/contacts", method: "POST", body: input });

    fetch = mockFetch(200, {});
    await contactsApi.update("c1", input);
    expect(sentRequest(fetch)).toEqual({ url: "/api/contacts/c1", method: "PUT", body: input });

    fetch = mockFetch(204);
    await contactsApi.remove("c1");
    expect(sentRequest(fetch)).toMatchObject({ url: "/api/contacts/c1", method: "DELETE" });
  });

  it("ARES lookup", async () => {
    const fetch = mockFetch(200, {});
    await aresApi.lookup("25596641");
    expect(sentRequest(fetch)).toMatchObject({ url: "/api/ares/25596641", method: "GET" });
  });
});
