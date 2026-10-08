import { onScopeDispose, ref, shallowRef, watch } from "vue";
import type { FieldErrors } from "@/api/types";
import { fieldErrorsOf } from "@/lib/formErrors";
import { documentsApi } from "./api";
import type { ComputeRequest, ComputeResult } from "./types";

export const COMPUTE_DEBOUNCE_MS = 300;

/**
 * Live totals: re-posts the compute body (debounced) whenever it changes.
 * Only the latest request may update the state — older ones are aborted and,
 * should they resolve anyway, ignored. On failure the last good result stays.
 */
export function useCompute(source: () => ComputeRequest, delay = COMPUTE_DEBOUNCE_MS) {
  const result = shallowRef<ComputeResult | null>(null);
  const fieldErrors = ref<FieldErrors>({});
  const error = shallowRef<unknown>(null);
  const pending = ref(false);

  let timer: ReturnType<typeof setTimeout> | undefined;
  let controller: AbortController | undefined;
  let seq = 0;
  // Bumped when a change is scheduled, so `pending` stays true while a newer
  // edit still waits on the debounce even if an older request finishes.
  let scheduled = 0;

  async function run(body: ComputeRequest, ticket: number): Promise<void> {
    controller?.abort();
    controller = new AbortController();
    const mine = ++seq;
    try {
      const res = await documentsApi.compute(body, controller.signal);
      if (mine !== seq) return;
      result.value = res;
      fieldErrors.value = {};
      error.value = null;
    } catch (err) {
      if (mine !== seq) return;
      const fields = fieldErrorsOf(err);
      fieldErrors.value = fields ?? {};
      error.value = fields ? null : err;
    } finally {
      if (ticket === scheduled) pending.value = false;
    }
  }

  // Serialized so unrelated reactive changes (e.g. a line's client key) don't refire.
  watch(
    () => JSON.stringify(source()),
    (json) => {
      clearTimeout(timer);
      pending.value = true;
      const ticket = ++scheduled;
      timer = setTimeout(() => void run(JSON.parse(json) as ComputeRequest, ticket), delay);
    },
    { immediate: true },
  );

  onScopeDispose(() => {
    clearTimeout(timer);
    controller?.abort();
    seq++;
  });

  return { result, fieldErrors, error, pending };
}
