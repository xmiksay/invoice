import type { RouteLocationRaw } from "vue-router";
import type { DocType } from "@/features/settings/types";
import { LIST_DOC_TYPES, type ListDocType } from "./store";

/** The list tab is the `?type=` query (absent = invoices), so links can open a tab. */
export function listLocation(docType: DocType): RouteLocationRaw {
  return { name: "invoices", query: docType === "invoice" ? {} : { type: docType } };
}

export function listDocTypeOf(query: unknown): ListDocType {
  return (LIST_DOC_TYPES as readonly unknown[]).includes(query) ? (query as ListDocType) : "invoice";
}

export const detailLocation = (id: string): RouteLocationRaw => ({ name: "invoice-detail", params: { id } });
export const editLocation = (id: string): RouteLocationRaw => ({ name: "invoice-edit", params: { id } });
