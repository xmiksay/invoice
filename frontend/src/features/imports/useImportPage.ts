import { computed, ref } from "vue";
import { useRoute } from "vue-router";
import { useErrorText } from "@/composables/useAction";

/**
 * Busy / error state of an import page and the link back to the list it was opened from (`?from=received`).
 * `message` translates the import-specific failures (upload limits, file errors); anything else gets the generic text.
 */
export function useImportPage(message: (err: unknown) => string | null) {
  const route = useRoute();
  const errorText = useErrorText();
  const busy = ref(false);
  const error = ref<string | null>(null);

  const backTo = computed(() => ({ name: route.query.from === "received" ? "received" : "invoices" }));

  async function act(action: () => Promise<void>): Promise<void> {
    busy.value = true;
    error.value = null;
    try {
      await action();
    } catch (err) {
      error.value = message(err) ?? errorText(err);
    } finally {
      busy.value = false;
    }
  }

  return { busy, error, backTo, act };
}
