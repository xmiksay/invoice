import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { effectScope, nextTick, ref } from "vue";
import { createPinia, setActivePinia } from "pinia";
import { flushPromises } from "@vue/test-utils";
import { ApiError } from "@/api/client";
import { calls, mockFetchRoutes, reply } from "@/test-utils";
import { useIndicativeRate } from "./useIndicativeRate";

describe("useIndicativeRate", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  function setup(currency: string, date: string) {
    const src = ref({ currency, date });
    const scope = effectScope();
    const state = scope.run(() => useIndicativeRate(() => src.value))!;
    return { src, state, scope };
  }

  it("fetches for a foreign currency after the debounce and refetches on date change", async () => {
    const fetch = mockFetchRoutes({ "GET /api/exchange-rates/EUR": { currency: "EUR", date: "2026-10-07", rate: "24.4" } });
    const { src, state, scope } = setup("EUR", "2026-10-08");
    expect(state.loading.value).toBe(true);
    await vi.advanceTimersByTimeAsync(300);
    await flushPromises();
    expect(state.rate.value?.rate).toBe("24.4");
    expect(state.loading.value).toBe(false);

    src.value = { currency: "EUR", date: "2026-09-30" };
    await nextTick();
    expect(state.rate.value).toBeNull();
    await vi.advanceTimersByTimeAsync(300);
    await flushPromises();
    expect(calls(fetch)).toEqual(["GET /api/exchange-rates/EUR?date=2026-10-08", "GET /api/exchange-rates/EUR?date=2026-09-30"]);
    scope.stop();
  });

  it("does nothing for CZK or an unfinished currency code", async () => {
    const fetch = mockFetchRoutes({});
    const { src, state, scope } = setup("CZK", "2026-10-08");
    src.value = { currency: "EU", date: "2026-10-08" };
    await nextTick();
    await vi.advanceTimersByTimeAsync(300);
    expect(fetch).not.toHaveBeenCalled();
    expect(state.loading.value).toBe(false);
    scope.stop();
  });

  it("keeps the error (404 / cnb_unavailable) without a rate", async () => {
    mockFetchRoutes({ "GET /api/exchange-rates/XYZ": reply(404, { code: "not_found" }) });
    const { state, scope } = setup("XYZ", "2026-10-08");
    await vi.advanceTimersByTimeAsync(300);
    await flushPromises();
    expect(state.rate.value).toBeNull();
    expect(state.error.value).toBeInstanceOf(ApiError);
    expect((state.error.value as ApiError).status).toBe(404);
    scope.stop();
  });
});
