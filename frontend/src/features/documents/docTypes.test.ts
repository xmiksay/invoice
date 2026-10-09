import { describe, expect, it } from "vitest";
import { canCorrectDdpp, canCreditOrDebit, docSign, isCorrection, isNativeNewType, relatedTargetTypes, simplifiedOverLimit } from "./docTypes";
import { DOCUMENT_TYPES } from "./types";

const totals = (total: string, totalCzk: string | null = null) => ({ total, totalCzk });

describe("docTypes", () => {
  it("signs: credit notes and DDPP corrections are negative, everything else positive", () => {
    const negative = DOCUMENT_TYPES.filter((t) => docSign(t) === -1);
    expect(negative).toEqual(["credit_note", "advance_credit_note"]);
  });

  it("corrections and native new types", () => {
    expect(DOCUMENT_TYPES.filter(isCorrection)).toEqual(["credit_note", "debit_note", "advance_credit_note"]);
    expect(DOCUMENT_TYPES.filter(isNativeNewType)).toEqual(["invoice", "simplified", "proforma"]);
  });

  it("credit and debit notes only on an issued own invoice / simplified document", () => {
    const doc = (docType: string, status = "issued", direction = "issued") => ({ docType, status, direction });
    expect(DOCUMENT_TYPES.filter((t) => canCreditOrDebit(doc(t)))).toEqual(["invoice", "simplified"]);
    expect(canCreditOrDebit(doc("invoice", "draft"))).toBe(false);
    expect(canCreditOrDebit(doc("invoice", "cancelled"))).toBe(false);
    expect(canCreditOrDebit(doc("invoice", "issued", "received"))).toBe(false);
  });

  it("a correction only on an issued DDPP", () => {
    const doc = (docType: string, status = "issued") => ({ docType, status, direction: "issued" });
    expect(DOCUMENT_TYPES.filter((t) => canCorrectDdpp(doc(t)))).toEqual(["advance_tax_doc"]);
    expect(canCorrectDdpp(doc("advance_tax_doc", "cancelled"))).toBe(false);
  });

  it("simplified limit: total for CZK, totalCzk otherwise, strictly above 10 000", () => {
    expect(simplifiedOverLimit("simplified", "CZK", totals("10000.00"))).toBe(false);
    expect(simplifiedOverLimit("simplified", "CZK", totals("10000.01"))).toBe(true);
    expect(simplifiedOverLimit("simplified", "EUR", totals("500.00", "12250.00"))).toBe(true);
    expect(simplifiedOverLimit("simplified", "EUR", totals("50000.00", "9000.00"))).toBe(false);
    // No CZK amount known yet (no rate): no warning.
    expect(simplifiedOverLimit("simplified", "EUR", totals("50000.00", null))).toBe(false);
    expect(simplifiedOverLimit("invoice", "CZK", totals("99999.00"))).toBe(false);
    expect(simplifiedOverLimit("simplified", "CZK", null)).toBe(false);
  });

  it("related link targets per doc type", () => {
    expect(relatedTargetTypes("invoice")).toEqual(["proforma"]);
    expect(relatedTargetTypes("advance_tax_doc")).toEqual(["proforma"]);
    expect(relatedTargetTypes("credit_note")).toEqual(["invoice", "simplified"]);
    expect(relatedTargetTypes("debit_note")).toEqual(["invoice", "simplified"]);
    expect(relatedTargetTypes("advance_credit_note")).toEqual(["advance_tax_doc"]);
    expect(relatedTargetTypes("simplified")).toEqual([]);
    expect(relatedTargetTypes("proforma")).toEqual([]);
  });
});
