import { onScopeDispose, ref, shallowRef, watch } from "vue";
import { exchangeRatesApi } from "./api";
import type { ExchangeRate } from "./types";

export const RATE_DEBOUNCE_MS = 300;

/**
 * Indicative ČNB rate for a foreign currency at a date, refetched (debounced)
 * when either changes. Display/preview only — issue fixes the real rate.
 */
export function useIndicativeRate(source: () => { currency: string; date: string }, delay = RATE_DEBOUNCE_MS) {
  const rate = shallowRef<ExchangeRate | null>(null);
  const error = shallowRef<unknown>(null);
  const loading = ref(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let seq = 0;

  async function load(currency: string, date: string, mine: number): Promise<void> {
    try {
      const res = await exchangeRatesApi.get(currency, date);
      if (mine === seq) rate.value = res;
    } catch (err) {
      if (mine === seq) error.value = err;
    } finally {
      if (mine === seq) loading.value = false;
    }
  }

  watch(
    () => {
      const { currency, date } = source();
      return /^[A-Z]{3}$/.test(currency) && currency !== "CZK" && /^\d{4}-\d{2}-\d{2}$/.test(date) ? `${currency}|${date}` : "";
    },
    (key) => {
      clearTimeout(timer);
      const mine = ++seq;
      rate.value = null;
      error.value = null;
      loading.value = key !== "";
      if (!key) return;
      const [currency = "", date = ""] = key.split("|");
      timer = setTimeout(() => void load(currency, date, mine), delay);
    },
    { immediate: true },
  );

  onScopeDispose(() => {
    clearTimeout(timer);
    seq++;
  });

  return { rate, error, loading };
}
