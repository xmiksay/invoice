import type { CatalogGroup, CatalogItem } from "./types";

/** Shared fixtures for catalog tests. */
export const catalogItem = (overrides: Partial<CatalogItem> = {}): CatalogItem => ({
  id: "i1",
  name: "Hosting",
  unit: "měs",
  unitPrice: "250",
  currency: "CZK",
  vatRate: "21",
  active: true,
  note: null,
  createdAt: "",
  updatedAt: "",
  ...overrides,
});

export const catalogGroup = (overrides: Partial<CatalogGroup> = {}): CatalogGroup => ({
  id: "g1",
  name: "Web package",
  collapse: true,
  members: [
    { itemId: "i2", quantity: "2", position: 2, item: catalogItem({ id: "i2", name: "Domain", unit: null, unitPrice: "300" }) },
    { itemId: "i1", quantity: "12", position: 1, item: catalogItem() },
  ],
  createdAt: "",
  updatedAt: "",
  ...overrides,
});
