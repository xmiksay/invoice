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

/** URL, method and parsed JSON body of the n-th fetch call. */
export function sentRequest(fn: ReturnType<typeof mockFetch>, call = 0) {
  const [url, init] = fn.mock.calls[call] ?? [];
  return {
    url,
    method: init?.method ?? "GET",
    body: typeof init?.body === "string" ? (JSON.parse(init.body) as unknown) : undefined,
  };
}

/** Stubs fetch with a queue of JSON bodies, one per call; `undefined` answers 204. */
export function mockFetchSequence(...bodies: unknown[]) {
  const fn = vi.fn<typeof fetch>(async () => {
    const body = bodies.shift();
    return body === undefined
      ? new Response(null, { status: 204 })
      : new Response(JSON.stringify(body), { status: 200, headers: { "Content-Type": "application/json" } });
  });
  vi.stubGlobal("fetch", fn);
  return fn;
}
