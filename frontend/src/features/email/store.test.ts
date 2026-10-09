import { afterEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { logEntry } from "./testData";
import { useEmailHistoryStore } from "./store";

describe("useEmailHistoryStore", () => {
  afterEach(() => vi.unstubAllGlobals());

  it("ignores a late answer for the previously opened document", async () => {
    setActivePinia(createPinia());
    const pending: Record<string, (body: unknown) => void> = {};
    vi.stubGlobal(
      "fetch",
      vi.fn(
        (url: string) =>
          new Promise<Response>((resolve) => {
            pending[url] = (body) => resolve(new Response(JSON.stringify(body), { status: 200, headers: { "Content-Type": "application/json" } }));
          }),
      ),
    );
    const store = useEmailHistoryStore();
    const first = store.load("d1");
    const second = store.load("d2");
    pending["/api/documents/d2/emails"]!([logEntry({ id: "for-d2" })]);
    await second;
    pending["/api/documents/d1/emails"]!([logEntry({ id: "for-d1" })]);
    await first;

    expect(store.documentId).toBe("d2");
    expect(store.entries.map((e) => e.id)).toEqual(["for-d2"]);
  });
});
