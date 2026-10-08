import { ref } from "vue";
import type { BlobResponse } from "@/api/client";
import { errorDetailOf } from "@/lib/formErrors";
import { downloadPdf, openPdf } from "@/lib/pdf";
import { useErrorText } from "./useAction";

type Load = () => Promise<BlobResponse>;

/** Open / download a PDF fetched with the token; keeps the translated error + render detail. */
export function usePdf() {
  const errorText = useErrorText();
  const busy = ref(false);
  const error = ref<string | null>(null);
  const detail = ref<string | null>(null);

  // `action()` is called before any await so `openPdf` still opens its tab within the click.
  /** Resolves true on success. */
  async function run(action: () => Promise<void>): Promise<boolean> {
    if (busy.value) return false;
    busy.value = true;
    error.value = null;
    detail.value = null;
    try {
      await action();
      return true;
    } catch (err) {
      error.value = errorText(err);
      detail.value = errorDetailOf(err);
      return false;
    } finally {
      busy.value = false;
    }
  }

  return {
    busy,
    error,
    detail,
    open: (load: Load) => run(() => openPdf(load)),
    download: (load: Load, fallbackName: string) => run(() => downloadPdf(load, fallbackName)),
  };
}
