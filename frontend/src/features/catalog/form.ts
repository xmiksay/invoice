import type { FieldErrors } from "@/api/types";
import { collectErrors, nullIfEmpty, textRule } from "@/lib/formErrors";
import type { CatalogGroup, CatalogGroupInput, CatalogItem, CatalogItemInput } from "./types";

const DECIMAL = /^-?\d+([.,]\d+)?$/;
/** Czech users type decimal commas; the wire wants a dot. */
const decimal = (value: string) => value.trim().replace(/\s/g, "").replace(",", ".");

export interface ItemDraft {
  name: string;
  unit: string;
  unitPrice: string;
  currency: string;
  vatRate: string;
  active: boolean;
  note: string;
}

export function toItemDraft(item: CatalogItem | undefined, defaultVatRate: string): ItemDraft {
  return {
    name: item?.name ?? "",
    unit: item?.unit ?? "",
    unitPrice: item?.unitPrice ?? "",
    currency: item?.currency ?? "CZK",
    vatRate: item?.vatRate ?? defaultVatRate,
    active: item?.active ?? true,
    note: item?.note ?? "",
  };
}

export function validateItem(d: ItemDraft): FieldErrors {
  const price = decimal(d.unitPrice);
  return collectErrors({
    name: textRule(d.name, { required: true, max: 200 }),
    unit: textRule(d.unit, { max: 20 }),
    unitPrice: price === "" ? "required" : !DECIMAL.test(price) && "invalid",
    currency: !/^[A-Za-z]{3}$/.test(d.currency.trim()) && "invalid",
    vatRate: d.vatRate === "" && "required",
    note: textRule(d.note, { max: 2000 }),
  });
}

export function toItemInput(d: ItemDraft): CatalogItemInput {
  return {
    name: d.name.trim(),
    unit: nullIfEmpty(d.unit),
    unitPrice: decimal(d.unitPrice),
    currency: d.currency.trim().toUpperCase(),
    vatRate: d.vatRate,
    active: d.active,
    note: nullIfEmpty(d.note),
  };
}

/** A group member in the editor; `item` is kept for display (name, rate, price). */
export interface MemberDraft {
  item: CatalogItem;
  quantity: string;
}

export interface GroupDraft {
  name: string;
  collapse: boolean;
  members: MemberDraft[];
}

export function toGroupDraft(group?: CatalogGroup): GroupDraft {
  return {
    name: group?.name ?? "",
    collapse: group?.collapse ?? true,
    members: [...(group?.members ?? [])]
      .sort((a, b) => a.position - b.position)
      .map((m) => ({ item: m.item, quantity: m.quantity })),
  };
}

/** Distinct VAT rates among the members; the server rejects more than one (`members: mixed_vat`). */
export const memberRates = (d: GroupDraft) => [...new Set(d.members.map((m) => m.item.vatRate))];

export function validateGroup(d: GroupDraft): FieldErrors {
  const errors = collectErrors({
    name: textRule(d.name, { required: true, max: 200 }),
    members: d.members.length === 0 ? "required" : memberRates(d).length > 1 && "mixed_vat",
  });
  d.members.forEach((m, i) => {
    const q = decimal(m.quantity);
    if (!DECIMAL.test(q) || Number(q) === 0) errors[`members.${i}.quantity`] = "invalid";
  });
  return errors;
}

export function toGroupInput(d: GroupDraft): CatalogGroupInput {
  return {
    name: d.name.trim(),
    collapse: d.collapse,
    members: d.members.map((m) => ({ itemId: m.item.id, quantity: decimal(m.quantity) })),
  };
}
