const LOCALE_TAGS: Record<string, string> = { cs: "cs-CZ", en: "en-GB" };

const tag = (locale: string) => LOCALE_TAGS[locale] ?? locale;

/** Formats a decimal string as money; display only, so the float conversion is harmless. */
export function formatMoney(amount: string | null | undefined, currency: string, locale: string): string {
  if (amount == null || amount === "") return "";
  const value = Number(amount);
  if (Number.isNaN(value)) return amount;
  try {
    return new Intl.NumberFormat(tag(locale), { style: "currency", currency }).format(value);
  } catch {
    // Unknown currency code — still show the number.
    return `${new Intl.NumberFormat(tag(locale), { minimumFractionDigits: 2 }).format(value)} ${currency}`;
  }
}

export function formatNumber(value: string, locale: string, maxFractionDigits = 4): string {
  const n = Number(value);
  if (Number.isNaN(n)) return value;
  return new Intl.NumberFormat(tag(locale), { maximumFractionDigits: maxFractionDigits }).format(n);
}

export function formatDate(date: string | null | undefined, locale: string): string {
  if (!date) return "";
  const d = new Date(date.length === 10 ? `${date}T00:00:00` : date);
  if (Number.isNaN(d.getTime())) return date;
  return new Intl.DateTimeFormat(tag(locale), date.length === 10 ? { dateStyle: "medium" } : { dateStyle: "medium", timeStyle: "short" }).format(d);
}

/** Integer cents of a 2-dp decimal string, so payment arithmetic stays exact. */
const cents = (amount: string) => Math.round(Number(amount) * 100);

/** `payable − paid`, never below zero, as a 2-dp string. */
export function remainingAmount(payable: string, paid: string): string {
  const rest = Math.max(0, cents(payable) - cents(paid));
  return (rest / 100).toFixed(2);
}

/** Negates a decimal string exactly (no float round-trip); "0.00" stays unsigned. */
export function negate(amount: string): string {
  const v = amount.trim();
  if (v.startsWith("-")) return v.slice(1);
  return /^[0.]*$/.test(v.replace(/^\+/, "")) ? v : `-${v.replace(/^\+/, "")}`;
}

/** Applies a document's sign (credit notes are stored positive, shown negative). */
export const signed = (amount: string, sign: 1 | -1): string => (sign === -1 ? negate(amount) : amount);

/** True for a decimal string below zero. */
export const isNegative = (amount: string | null | undefined): boolean => !!amount && Number(amount) < 0;
