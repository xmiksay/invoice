import { createI18n } from "vue-i18n";
import cs from "@/locales/cs.json";
import en from "@/locales/en.json";
import { readStorage, writeStorage } from "@/lib/storage";

export const LOCALES = ["cs", "en"] as const;
export type Locale = (typeof LOCALES)[number];

export const LOCALE_STORAGE_KEY = "invoice.locale";
const DEFAULT_LOCALE: Locale = "cs";

function isLocale(value: string | null): value is Locale {
  return (LOCALES as readonly string[]).includes(value ?? "");
}

const stored = readStorage(LOCALE_STORAGE_KEY);
const initial: Locale = isLocale(stored) ? stored : DEFAULT_LOCALE;

export const i18n = createI18n({
  legacy: false,
  locale: initial,
  fallbackLocale: DEFAULT_LOCALE,
  messages: { cs, en },
});

export function setLocale(locale: Locale): void {
  i18n.global.locale.value = locale;
  writeStorage(LOCALE_STORAGE_KEY, locale);
  document.documentElement.lang = locale;
}
