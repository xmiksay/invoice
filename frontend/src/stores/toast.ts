import { defineStore } from "pinia";
import { ref } from "vue";

const DURATION_MS = 4000;

/** One transient success message at a time; a newer one replaces it. */
export const useToastStore = defineStore("toast", () => {
  const message = ref<string | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;

  function dismiss(): void {
    clearTimeout(timer);
    message.value = null;
  }

  function show(text: string): void {
    clearTimeout(timer);
    message.value = text;
    timer = setTimeout(dismiss, DURATION_MS);
  }

  return { message, show, dismiss };
});
