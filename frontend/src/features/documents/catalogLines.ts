import type { CatalogGroup, CatalogItem } from "@/features/catalog/types";
import { newItemLine, newSubtotalLine, type LineDraft } from "./lines";

export interface CatalogInsert {
  lines: LineDraft[];
  /** Names of items inserted without a price because their currency differs from the document's. */
  unpriced: string[];
}

interface Target {
  /** Document currency. */
  currency: string;
  /** Non-payers charge no VAT: every rate becomes "0". */
  vatLocked: boolean;
}

/** A catalog item as an item line; the price is left empty (no conversion) when the currencies differ. */
function itemLine(item: CatalogItem, quantity: string, target: Target): LineDraft {
  return {
    ...newItemLine(target.vatLocked ? "0" : item.vatRate),
    description: item.name,
    quantity,
    unit: item.unit,
    unitPrice: item.currency === target.currency ? item.unitPrice : "",
  };
}

const unpriced = (items: CatalogItem[], currency: string) =>
  [...new Set(items.filter((i) => i.currency !== currency).map((i) => i.name))];

export function insertCatalogItem(item: CatalogItem, target: Target): CatalogInsert {
  return { lines: [itemLine(item, "1", target)], unpriced: unpriced([item], target.currency) };
}

/**
 * A group as its member lines followed by a subtotal over them (with the
 * group's collapse), to be appended after `existing` lines — refs are the
 * members' 1-based positions in the combined list.
 */
export function insertCatalogGroup(group: CatalogGroup, existing: number, target: Target): CatalogInsert {
  const members = [...group.members].sort((a, b) => a.position - b.position);
  const lines = members.map((m) => itemLine(m.item, m.quantity, target));
  const subtotal = {
    ...newSubtotalLine(lines.map((_, i) => existing + i + 1)),
    description: group.name,
    collapse: group.collapse,
  };
  return { lines: [...lines, subtotal], unpriced: unpriced(members.map((m) => m.item), target.currency) };
}
