import { describe, expect, it } from "vitest";
import { toAccountingDraft, toAccountingSettings, validateAccounting } from "./accounting";
import type { AccountingSettings } from "./types";

const settings: AccountingSettings = {
  pohoda: {
    ico: "12345678",
    codes: [{ direction: "received", docType: "credit_note", accounting: "3Pd", classificationVat: "PD", numberSeries: null, classificationVatNonDeductible: "PN" }],
  },
  money: { codes: [{ direction: "issued", docType: "invoice", future: "x" }] },
};

describe("accounting draft", () => {
  it("lays out every direction × doc type in display order, missing rows empty", () => {
    const draft = toAccountingDraft(settings);
    expect(draft.rows.map((r) => `${r.direction}/${r.docType}`)).toEqual([
      "issued/invoice",
      "issued/credit_note",
      "issued/debit_note",
      "issued/advance_tax_doc",
      "issued/advance_credit_note",
      "issued/simplified",
      "received/invoice",
      "received/credit_note",
      "received/debit_note",
      "received/advance_tax_doc",
      "received/advance_credit_note",
      "received/simplified",
    ]);
    expect(draft.rows[7]).toEqual({
      direction: "received",
      docType: "credit_note",
      accounting: "3Pd",
      classificationVat: "PD",
      numberSeries: "",
      classificationVatNonDeductible: "PN",
    });
    expect(draft.rows[0]).toMatchObject({ accounting: "", classificationVat: "", numberSeries: "" });
    expect(draft.ico).toBe("12345678");
  });

  it("trims, sends empty as null and passes money back untouched", () => {
    const draft = toAccountingDraft(settings);
    draft.ico = "  ";
    draft.rows[0]!.numberSeries = " FV ";
    // Not editable on issued rows; a stray value is never sent.
    draft.rows[0]!.classificationVatNonDeductible = "X";
    const out = toAccountingSettings(draft);
    expect(out.pohoda.ico).toBeNull();
    expect(out.pohoda.codes).toHaveLength(12);
    expect(out.pohoda.codes[0]).toEqual({ direction: "issued", docType: "invoice", accounting: null, classificationVat: null, numberSeries: "FV", classificationVatNonDeductible: null });
    expect(out.pohoda.codes[7]!.classificationVatNonDeductible).toBe("PN");
    expect(out.money).toEqual(settings.money);
  });

  it("leaves money out when the server did not send it", () => {
    const { money: _money, ...withoutMoney } = settings;
    const out = toAccountingSettings(toAccountingDraft(withoutMoney));
    expect("money" in out).toBe(false);
  });

  it("flags codes over 19 characters under the server's keys", () => {
    const draft = toAccountingDraft(null);
    draft.rows[3]!.classificationVat = "x".repeat(20);
    draft.rows[4]!.accounting = ` ${"y".repeat(19)} `;
    expect(validateAccounting(draft)).toEqual({ "pohoda.codes.3.classificationVat": "too_long" });
  });
});
