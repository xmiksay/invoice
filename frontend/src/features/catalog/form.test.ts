import { describe, expect, it } from "vitest";
import { toGroupDraft, toGroupInput, toItemDraft, toItemInput, validateGroup, validateItem } from "./form";
import { catalogGroup, catalogItem } from "./testData";

describe("catalog item form", () => {
  it("new item defaults: CZK, active, given VAT rate", () => {
    expect(toItemDraft(undefined, "21")).toMatchObject({ currency: "CZK", active: true, vatRate: "21", unitPrice: "" });
  });

  it("round-trips an item; trims, decimal comma, empty → null, currency upper-cased", () => {
    const d = { ...toItemDraft(catalogItem(), "21"), name: " Hosting ", unit: " ", unitPrice: "1 250,5", currency: "eur", note: "" };
    expect(toItemInput(d)).toEqual({ name: "Hosting", unit: null, unitPrice: "1250.5", currency: "EUR", vatRate: "21", active: true, note: null });
  });

  it("validates name, price and currency", () => {
    expect(validateItem({ ...toItemDraft(undefined, "21"), currency: "E" })).toEqual({ name: "required", unitPrice: "required", currency: "invalid" });
    expect(validateItem({ ...toItemDraft(catalogItem(), "21"), unitPrice: "abc" })).toEqual({ unitPrice: "invalid" });
  });
});

describe("catalog group form", () => {
  it("orders members by position and sends them in that order", () => {
    const d = toGroupDraft(catalogGroup());
    expect(d.members.map((m) => m.item.name)).toEqual(["Hosting", "Domain"]);
    expect(toGroupInput({ ...d, name: " Web " })).toEqual({
      name: "Web",
      collapse: true,
      members: [
        { itemId: "i1", quantity: "12" },
        { itemId: "i2", quantity: "2" },
      ],
    });
  });

  it("new group collapses by default and needs a name and a member", () => {
    const d = toGroupDraft();
    expect(d.collapse).toBe(true);
    expect(validateGroup(d)).toEqual({ name: "required", members: "required" });
  });

  it("flags mixed VAT rates and bad quantities by index", () => {
    const d = toGroupDraft(catalogGroup());
    d.members[1] = { item: catalogItem({ id: "i3", vatRate: "12" }), quantity: "0" };
    expect(validateGroup(d)).toEqual({ members: "mixed_vat", "members.1.quantity": "invalid" });
  });
});
