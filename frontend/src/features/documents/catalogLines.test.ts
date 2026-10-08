import { describe, expect, it } from "vitest";
import { catalogGroup, catalogItem } from "@/features/catalog/testData";
import { insertCatalogGroup, insertCatalogItem } from "./catalogLines";
import { toWireLine } from "./lines";

const czk = { currency: "CZK", vatLocked: false };

describe("catalog insertion", () => {
  it("item in the document currency keeps its price, unit and rate", () => {
    const { lines, unpriced } = insertCatalogItem(catalogItem(), czk);
    expect(lines.map(toWireLine)).toEqual([
      { kind: "item", description: "Hosting", quantity: "1", unit: "měs", unitPrice: "250", discountPct: "0", vatRate: "21" },
    ]);
    expect(unpriced).toEqual([]);
  });

  it("item in another currency leaves the price empty and reports it", () => {
    const { lines, unpriced } = insertCatalogItem(catalogItem({ currency: "EUR" }), czk);
    expect(lines[0]).toMatchObject({ kind: "item", unitPrice: "" });
    expect(unpriced).toEqual(["Hosting"]);
  });

  it("non-payer documents get rate 0", () => {
    expect(insertCatalogItem(catalogItem(), { currency: "CZK", vatLocked: true }).lines[0]).toMatchObject({ vatRate: "0" });
  });

  it("group → member lines in order + a subtotal over them with the group's collapse", () => {
    const { lines } = insertCatalogGroup(catalogGroup(), 0, czk);
    expect(lines.map(toWireLine)).toEqual([
      { kind: "item", description: "Hosting", quantity: "12", unit: "měs", unitPrice: "250", discountPct: "0", vatRate: "21" },
      { kind: "item", description: "Domain", quantity: "2", unit: null, unitPrice: "300", discountPct: "0", vatRate: "21" },
      { kind: "subtotal", description: "Web package", refs: [1, 2], collapse: true },
    ]);
  });

  it("subtotal refs account for the lines already in the document", () => {
    const { lines, unpriced } = insertCatalogGroup(catalogGroup({ collapse: false }), 3, { currency: "EUR", vatLocked: false });
    expect(lines[2]).toMatchObject({ kind: "subtotal", refs: [4, 5], collapse: false });
    expect(lines.slice(0, 2).map((l) => (l.kind === "item" ? l.unitPrice : null))).toEqual(["", ""]);
    expect(unpriced).toEqual(["Hosting", "Domain"]);
  });
});
