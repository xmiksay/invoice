import { vi } from "vitest";

/** Stubs global fetch with a single canned response; returns the mock for assertions. */
export function mockFetch(status: number, body?: unknown) {
  const fn = vi.fn<typeof fetch>(async () =>
    new Response(body === undefined ? null : JSON.stringify(body), {
      status,
      headers: { "Content-Type": "application/json" },
    }),
  );
  vi.stubGlobal("fetch", fn);
  return fn;
}

/** Headers sent on the n-th fetch call. */
export function sentHeaders(fn: ReturnType<typeof mockFetch>, call = 0): Headers {
  return new Headers(fn.mock.calls[call]?.[1]?.headers);
}
