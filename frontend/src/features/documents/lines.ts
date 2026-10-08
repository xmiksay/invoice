import type { FieldErrors } from "@/api/types";
import type { DocumentLine, ItemLine, SubtotalLine, TextLine, VatMode } from "./types";

/**
 * Editor state of a line: the wire line plus a client-only `key` so v-for keeps
 * input state attached to the right row across reorders. Item amounts stay the
 * strings the user typed.
 */
export type LineDraft = DocumentLine & { key: number };
type Keyed<T> = T & { key: number };

/** A line a subtotal can reference, shown by its 1-based position. */
export interface RefOption {
  position: number;
  label: string;
}

let nextKey = 1;
const key = () => nextKey++;

export function newItemLine(vatRate: string): Keyed<ItemLine> {
  return { key: key(), kind: "item", description: "", quantity: "1", unit: null, unitPrice: "0", discountPct: "0", vatRate };
}

export function newTextLine(): Keyed<TextLine> {
  return { key: key(), kind: "text", description: "" };
}

export function newSubtotalLine(refs: number[] = []): Keyed<SubtotalLine> {
  return { key: key(), kind: "subtotal", description: "", refs, collapse: false };
}

/**
 * Refs for a subtotal appended at the end: the item lines after the last
 * subtotal. The server rejects empty refs, so starting empty would break the
 * live totals until the user ticks something.
 */
export function defaultSubtotalRefs(lines: LineDraft[]): number[] {
  const lastSubtotal = lines.map((l) => l.kind).lastIndexOf("subtotal");
  return lines.flatMap((l, i) => (i > lastSubtotal && l.kind === "item" ? [i + 1] : []));
}

/** Wire/response line → editor draft (drops response-only fields such as position/base). */
export function toLineDraft(line: DocumentLine): LineDraft {
  switch (line.kind) {
    case "item":
      return {
        key: key(),
        kind: "item",
        description: line.description,
        quantity: line.quantity,
        unit: line.unit,
        unitPrice: line.unitPrice,
        discountPct: line.discountPct,
        vatRate: line.vatRate,
      };
    case "text":
      return { key: key(), kind: "text", description: line.description };
    case "subtotal":
      return { key: key(), kind: "subtotal", description: line.description, refs: [...line.refs], collapse: line.collapse };
  }
}

/** Czech users type decimal commas; the wire wants a dot. */
const decimal = (value: string) => value.trim().replace(/\s/g, "").replace(",", ".");

export function toWireLine(line: LineDraft): DocumentLine {
  switch (line.kind) {
    case "item": {
      const unit = line.unit?.trim() ?? "";
      const discount = decimal(line.discountPct);
      return {
        kind: "item",
        description: line.description.trim(),
        quantity: decimal(line.quantity),
        unit: unit === "" ? null : unit,
        unitPrice: decimal(line.unitPrice),
        discountPct: discount === "" ? "0" : discount,
        vatRate: line.vatRate,
      };
    }
    case "text":
      return { kind: "text", description: line.description.trim() };
    case "subtotal":
      return {
        kind: "subtotal",
        description: line.description.trim(),
        refs: [...line.refs].sort((a, b) => a - b),
        collapse: line.collapse,
      };
  }
}

/** Applies `oldPosition → newPosition | null` to every subtotal's refs; null drops the ref. */
function remapRefs(lines: LineDraft[], map: (pos: number) => number | null): LineDraft[] {
  return lines.map((line) => {
    if (line.kind !== "subtotal") return line;
    const refs = line.refs.map(map).filter((p): p is number => p !== null);
    return { ...line, refs: [...new Set(refs)].sort((a, b) => a - b) };
  });
}

/** Moves the line at `from` to index `to`; subtotal refs follow the lines they point at. */
export function moveLine(lines: LineDraft[], from: number, to: number): LineDraft[] {
  if (from === to || from < 0 || to < 0 || from >= lines.length || to >= lines.length) return lines;
  const order = lines.map((_, i) => i);
  const [moved] = order.splice(from, 1);
  order.splice(to, 0, moved as number);
  // order[newIndex] = oldIndex → invert to oldPosition → newPosition.
  const newPos = new Map(order.map((oldIndex, newIndex) => [oldIndex + 1, newIndex + 1]));
  const reordered = order.map((oldIndex) => lines[oldIndex] as LineDraft);
  return remapRefs(reordered, (pos) => newPos.get(pos) ?? null);
}

/** Removes the line at `index`; refs to it are dropped, refs after it shift down by one. */
export function removeLine(lines: LineDraft[], index: number): LineDraft[] {
  if (index < 0 || index >= lines.length) return lines;
  const removed = index + 1;
  const rest = lines.filter((_, i) => i !== index);
  return remapRefs(rest, (pos) => (pos === removed ? null : pos > removed ? pos - 1 : pos));
}

/** Non-payers charge no VAT: every item is forced to rate "0". */
export function enforceVatMode(lines: LineDraft[], vatMode: VatMode): LineDraft[] {
  if (vatMode !== "non_payer") return lines;
  return lines.map((l) => (l.kind === "item" && l.vatRate !== "0" ? { ...l, vatRate: "0" } : l));
}

const LINE_FIELD = /^lines\.(\d+)\.(\w+)$/;

/**
 * Splits a 422 `fields` map into header fields and per-line fields.
 * `lines.N.field` uses the 0-based array index of the line.
 */
export function splitLineErrors(fields: FieldErrors): { header: FieldErrors; lines: Record<number, FieldErrors> } {
  const header: FieldErrors = {};
  const lines: Record<number, FieldErrors> = {};
  for (const [field, reason] of Object.entries(fields)) {
    const m = LINE_FIELD.exec(field);
    if (m) {
      const index = Number(m[1]);
      (lines[index] ??= {})[m[2] as string] = reason;
    } else {
      header[field] = reason;
    }
  }
  return { header, lines };
}
