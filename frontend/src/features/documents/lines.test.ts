import { describe, expect, it } from "vitest";
import {
  defaultSubtotalRefs,
  enforceVatMode,
  moveLine,
  newItemLine,
  refTargets,
  newSubtotalLine,
  newTextLine,
  removeLine,
  splitLineErrors,
  toLineDraft,
  toWireLine,
  type LineDraft,
} from "./lines";

const item = (description: string, vatRate = "21"): LineDraft => ({ ...newItemLine(vatRate), description });
const subtotal = (description: string, refs: number[]): LineDraft => ({ ...newSubtotalLine(), description, refs });
const refsOf = (lines: LineDraft[]) => lines.map((l) => (l.kind === "subtotal" ? l.refs : null));
const names = (lines: LineDraft[]) => lines.map((l) => l.description);

describe("line defaults", () => {
  it("new item: quantity 1, price 0, no discount, given rate; unique keys", () => {
    const a = newItemLine("21");
    const b = newItemLine("12");
    expect(a).toMatchObject({ kind: "item", quantity: "1", unitPrice: "0", discountPct: "0", unit: null, vatRate: "21" });
    expect(b.vatRate).toBe("12");
    expect(a.key).not.toBe(b.key);
    expect(newTextLine()).toMatchObject({ kind: "text", description: "" });
    expect(newSubtotalLine()).toMatchObject({ kind: "subtotal", refs: [], collapse: false });
  });
});

describe("defaultSubtotalRefs", () => {
  it("picks the items after the last subtotal", () => {
    expect(defaultSubtotalRefs([])).toEqual([]);
    expect(defaultSubtotalRefs([item("A"), newTextLine(), item("B")])).toEqual([1, 3]);
    expect(defaultSubtotalRefs([item("A"), subtotal("S", [1]), item("B"), item("C")])).toEqual([3, 4]);
  });
});

describe("toWireLine", () => {
  it("normalizes an item: trims, decimal comma → dot, empty unit → null, empty discount → 0", () => {
    const line: LineDraft = {
      key: 1,
      kind: "item",
      description: "  Konzultace ",
      quantity: " 1,5 ",
      unit: " ",
      unitPrice: "1 200,50",
      discountPct: "",
      vatRate: "21",
    };
    expect(toWireLine(line)).toEqual({
      kind: "item",
      description: "Konzultace",
      quantity: "1.5",
      unit: null,
      unitPrice: "1200.50",
      discountPct: "0",
      vatRate: "21",
    });
  });

  it("drops the client key and sorts subtotal refs", () => {
    expect(toWireLine({ key: 9, kind: "subtotal", description: "S", refs: [3, 1], collapse: true })).toEqual({
      kind: "subtotal",
      description: "S",
      refs: [1, 3],
      collapse: true,
    });
    expect(toWireLine({ key: 9, kind: "text", description: " note " })).toEqual({ kind: "text", description: "note" });
  });

  it("response line → draft drops position/base", () => {
    const draft = toLineDraft({ kind: "subtotal", description: "S", refs: [1], collapse: false, position: 2, base: "10.00" } as never);
    expect(draft).not.toHaveProperty("position");
    expect(draft).not.toHaveProperty("base");
    expect(draft).toMatchObject({ kind: "subtotal", refs: [1] });
  });
});

describe("moveLine", () => {
  // 1 A, 2 B, 3 C, 4 Σ(1,2)
  const base = () => [item("A"), item("B"), item("C"), subtotal("S", [1, 2])];

  it("remaps refs when a referenced line moves down", () => {
    const moved = moveLine(base(), 0, 2); // B, C, A, S
    expect(names(moved)).toEqual(["B", "C", "A", "S"]);
    expect(refsOf(moved)[3]).toEqual([1, 3]);
  });

  it("remaps refs when the subtotal itself moves up", () => {
    const moved = moveLine(base(), 3, 0); // S, A, B, C
    expect(names(moved)).toEqual(["S", "A", "B", "C"]);
    expect(refsOf(moved)[0]).toEqual([2, 3]);
  });

  it("swapping neighbours keeps the members", () => {
    const moved = moveLine(base(), 1, 2); // A, C, B, S
    expect(refsOf(moved)[3]).toEqual([1, 3]);
  });

  it("remaps nested subtotal refs", () => {
    const lines = [item("A"), subtotal("S1", [1]), item("B"), subtotal("S2", [2, 3])];
    const moved = moveLine(lines, 0, 3); // S1, B, S2, A
    expect(names(moved)).toEqual(["S1", "B", "S2", "A"]);
    expect(refsOf(moved)).toEqual([[4], null, [1, 2], null]);
  });

  it("ignores out-of-range moves", () => {
    const lines = base();
    expect(moveLine(lines, 0, -1)).toBe(lines);
    expect(moveLine(lines, 3, 4)).toBe(lines);
  });
});

describe("removeLine", () => {
  it("drops refs to the removed line and shifts later positions down", () => {
    const lines = [item("A"), item("B"), item("C"), subtotal("S", [1, 2, 3])];
    const rest = removeLine(lines, 1);
    expect(names(rest)).toEqual(["A", "C", "S"]);
    expect(refsOf(rest)[2]).toEqual([1, 2]);
  });

  it("leaves refs before the removed line untouched", () => {
    const lines = [item("A"), subtotal("S", [1]), item("B")];
    expect(refsOf(removeLine(lines, 2))).toEqual([null, [1]]);
  });
});

describe("enforceVatMode", () => {
  it("forces item rates to 0 for non-payers only", () => {
    const lines = [item("A", "21"), newTextLine()];
    expect(enforceVatMode(lines, "standard")).toBe(lines);
    const forced = enforceVatMode(lines, "non_payer");
    expect(forced[0]).toMatchObject({ vatRate: "0" });
    expect(forced[1]).toBe(lines[1]);
  });
});

describe("splitLineErrors", () => {
  it("routes lines.N.field to the 0-based line index", () => {
    expect(
      splitLineErrors({ contactId: "required", "lines.0.quantity": "invalid", "lines.2.refs": "invalid", "lines.0.vatRate": "invalid", lines: "required" }),
    ).toEqual({
      header: { contactId: "required", lines: "required" },
      lines: { 0: { quantity: "invalid", vatRate: "invalid" }, 2: { refs: "invalid" } },
    });
  });
});

describe("advance lines", () => {
  const advance = (): LineDraft =>
    toLineDraft({ kind: "advance", position: 1, advanceDocumentId: "ddpp1", description: "Odpočet", base: "-100.00", recap: [{ vatRate: "21", base: "-100.00", vat: "-21.00" }] });

  it("are never subtotal targets", () => {
    const lines = [item("A"), advance(), newTextLine(), subtotal("S", [1]), item("B")];
    expect(refTargets(lines, 3)).toEqual([1, 5]);
    expect(refTargets(lines, 0)).toEqual([4, 5]);
  });

  it("removing one shifts the subtotal refs after it", () => {
    const lines = [item("A"), advance(), item("B"), subtotal("S", [1, 3])];
    expect(refsOf(removeLine(lines, 1))).toEqual([null, null, [1, 2]]);
  });

  it("wire form is only the reference", () => {
    expect(toWireLine(advance())).toEqual({ kind: "advance", advanceDocumentId: "ddpp1" });
  });
});
