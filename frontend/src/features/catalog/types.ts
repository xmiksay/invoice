/** Wire types for `/api/catalog` (Phase 1c). Decimals are strings. */

export interface CatalogItem {
  id: string;
  name: string;
  unit: string | null;
  /** Excl. VAT, up to 4 dp. */
  unitPrice: string;
  currency: string;
  vatRate: string;
  active: boolean;
  note: string | null;
  createdAt: string;
  updatedAt: string;
}
export type CatalogItemInput = Omit<CatalogItem, "id" | "createdAt" | "updatedAt">;

export interface CatalogGroupMember {
  itemId: string;
  quantity: string;
  position: number;
  item: CatalogItem;
}

export interface CatalogGroup {
  id: string;
  /** Becomes the subtotal description when inserted into a document. */
  name: string;
  collapse: boolean;
  members: CatalogGroupMember[];
  createdAt: string;
  updatedAt: string;
}

/** Members in order (= position), at least one, unique `itemId`, one shared VAT rate. */
export interface CatalogGroupInput {
  name: string;
  collapse: boolean;
  members: { itemId: string; quantity: string }[];
}
