import { createI18n } from "vue-i18n";
import { readStorage, writeStorage } from "@/lib/storage";

export type MessageTree = { [key: string]: string | MessageTree };

/**
 * Locale files are split per top-level namespace (`locales/<locale>/<namespace>.json`,
 * file content = that namespace) to keep each file small; merged here so runtime keys
 * stay `namespace.key`.
 */
const files = {
  cs: import.meta.glob<MessageTree>("../locales/cs/*.json", { eager: true, import: "default" }),
  en: import.meta.glob<MessageTree>("../locales/en/*.json", { eager: true, import: "default" }),
};

const namespaceOf = (path: string) => path.slice(path.lastIndexOf("/") + 1, -".json".length);

/** Namespace → tree per locale (also used by the parity test). */
export const localeFiles = Object.fromEntries(
  Object.entries(files).map(([locale, modules]) => [
    locale,
    Object.fromEntries(Object.entries(modules).map(([path, tree]) => [namespaceOf(path), tree])),
  ]),
) as Record<"cs" | "en", Record<string, MessageTree>>;

const { cs, en } = localeFiles;

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
