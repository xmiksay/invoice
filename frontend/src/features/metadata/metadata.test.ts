import { describe, expect, it } from "vitest";
import type { Category, CustomField } from "@/features/settings/types";
import {
  applicableFields,
  categoryOptions,
  customFieldErrors,
  serializeCustomFields,
  toCustomFieldDraft,
  toMetadataInput,
  validateCustomFields,
} from "./metadata";

const field = (overrides: Partial<CustomField>): CustomField => ({
  id: overrides.key ?? "f",
  key: "f",
  label: "F",
  type: "text",
  options: [],
  appliesTo: "both",
  required: false,
  active: true,
  position: 0,
  ...overrides,
});

const defs = [
  field({ key: "note", type: "text", position: 3 }),
  field({ key: "amount", type: "number", required: true, position: 1 }),
  field({ key: "due", type: "date", appliesTo: "received", position: 2 }),
  field({ key: "flag", type: "bool", appliesTo: "issued" }),
  field({ key: "kind", type: "select", options: ["A", "B"] }),
  field({ key: "old", active: false }),
];

describe("custom fields", () => {
  it("applies active definitions of the direction in position order", () => {
    expect(applicableFields(defs, "received").map((d) => d.key)).toEqual(["kind", "amount", "due", "note"]);
    expect(applicableFields(defs, "issued").map((d) => d.key)).toEqual(["flag", "kind", "amount", "note"]);
  });

  it("drafts a value per field and keeps stored keys of other definitions", () => {
    const active = applicableFields(defs, "issued");
    expect(toCustomFieldDraft({ amount: "12.5", old: "keep" }, active)).toEqual({ amount: "12.5", old: "keep", flag: false, kind: "", note: "" });
  });

  it("validates type, required, length and options", () => {
    const active = applicableFields(defs, "received");
    expect(validateCustomFields({ amount: "", due: "08.10.2026", kind: "C", note: "x".repeat(501) }, active)).toEqual({
      "customFields.amount": "required",
      "customFields.due": "invalid",
      "customFields.kind": "invalid",
      "customFields.note": "too_long",
    });
    expect(validateCustomFields({ amount: "-12,5", due: "2026-10-08", kind: "A", note: "" }, active)).toEqual({});
  });

  it("serializes empty to null, decimal comma to dot, and passes unknown keys through unchanged", () => {
    const active = applicableFields(defs, "issued");
    expect(serializeCustomFields({ amount: " 12,5 ", note: "  ", flag: true, kind: "B", old: "keep" }, active)).toEqual({
      amount: "12.5",
      note: null,
      flag: true,
      kind: "B",
      old: "keep",
    });
  });

  it("routes customFields.<key> errors by key", () => {
    expect(customFieldErrors({ "customFields.po": "unknown", internalNote: "too_long" })).toEqual({ po: "unknown" });
  });
});

describe("categories + metadata input", () => {
  const cat = (id: string, kind: Category["kind"], active = true, position = 0): Category => ({ id, name: id, kind, active, position });

  it("offers active categories of the kind plus the current inactive one", () => {
    const all = [cat("b", "expense", true, 2), cat("a", "expense", true, 1), cat("x", "expense", false), cat("i", "income")];
    expect(categoryOptions(all, "expense", null).map((c) => c.id)).toEqual(["a", "b"]);
    expect(categoryOptions(all, "expense", "x").map((c) => c.id)).toEqual(["x", "a", "b"]);
  });

  it("maps empty strings to null", () => {
    expect(toMetadataInput({ categoryId: "", customFields: {}, internalNote: " " }, [])).toEqual({ categoryId: null, customFields: {}, internalNote: null });
  });
});
