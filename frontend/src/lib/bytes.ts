const UNITS = ["B", "kB", "MB", "GB"] as const;

/** Human-readable size in 1024 steps: `512 B`, `1,5 kB` (cs) / `1.5 kB` (en). */
export function formatBytes(bytes: number, locale: string): string {
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const digits = unit === 0 ? 0 : 1;
  const number = new Intl.NumberFormat(locale, { maximumFractionDigits: digits }).format(value);
  return `${number} ${UNITS[unit]}`;
}
