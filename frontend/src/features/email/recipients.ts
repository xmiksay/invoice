import type { FieldErrors } from "@/api/types";
import type { RecipientField } from "./types";

const same = (a: string, b: string) => a.trim().toLowerCase() === b.trim().toLowerCase();

export const hasAddress = (list: string[], address: string): boolean => list.some((a) => same(a, address));

/** Separators between typed / pasted addresses; not whitespace, so `Name <addr@x>` stays one entry. */
export const SEPARATORS = /[,;\n]/;

/** Typed / pasted text → trimmed addresses, appended without duplicates. */
export function addAddresses(list: string[], text: string): string[] {
  const out = [...list];
  for (const part of text.split(SEPARATORS)) {
    const address = part.trim();
    if (address && !hasAddress(out, address)) out.push(address);
  }
  return out;
}

/** Adds or removes one address (case-insensitive), e.g. the "Bcc to me" toggle. */
export function withAddress(list: string[], address: string, on: boolean): string[] {
  if (on) return hasAddress(list, address) ? list : [...list, address];
  return list.filter((a) => !same(a, address));
}

export interface RecipientErrors {
  /** Reason for the whole list (`to: required`, `to: too_long`). */
  list: string | null;
  /** Reason per address index (`to.0: invalid`). */
  items: Record<number, string>;
}

/** Splits the server's `to` / `to.0` style field keys of one recipient list. */
export function recipientErrors(fields: FieldErrors, field: RecipientField): RecipientErrors {
  const items: Record<number, string> = {};
  for (const [key, reason] of Object.entries(fields)) {
    const match = new RegExp(`^${field}\\.(\\d+)$`).exec(key);
    if (match) items[Number(match[1])] = reason;
  }
  return { list: fields[field] ?? null, items };
}
