import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { errorMessageKey } from "@/lib/formErrors";

/** Translates any thrown error into a user-facing message. */
export function useErrorText() {
  const { t } = useI18n();
  return (err: unknown): string => {
    const { key, params } = errorMessageKey(err);
    return params ? t(key, params) : t(key);
  };
}

/** Runs an async action and keeps the last failure as a translated message. */
export function useAction() {
  const errorText = useErrorText();
  const error = ref<string | null>(null);

  async function run(action: () => Promise<void>): Promise<boolean> {
    error.value = null;
    try {
      await action();
      return true;
    } catch (err) {
      error.value = errorText(err);
      return false;
    }
  }

  return { error, run, errorText };
}
