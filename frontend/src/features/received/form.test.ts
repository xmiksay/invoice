import { describe, expect, it } from "vitest";
import { document, vatRates } from "@/features/documents/testData";
import type { CustomField } from "@/features/settings/types";
import { applyDefaults, newReceivedDraft, toReceivedDraft, toReceivedInput, validateReceived, type ReceivedDraft } from "./form";
import { updateRow } from "./recap";

const ctx = { vatRates, fieldDefs: [] as CustomField[], today: "2026-10-08" };

function filled(overrides: Partial<ReceivedDraft> = {}): ReceivedDraft {
  const d = newReceivedDraft(ctx);
  return applyDefaults({
    ...d,
    contactId: "c1",
    supplierNumber: "FV-77",
    dueDate: "2026-10-22",
    vatRecap: [updateRow(d.vatRecap[0]!, { base: "1000" }, "standard")],
    ...overrides,
  });
}

describe("received form defaults", () => {
  it("starts with one recap row at the default rate, no tax point for a proforma, no due date for a DDPP", () => {
    expect(newReceivedDraft(ctx).vatRecap.map((r) => r.rate)).toEqual(["21"]);
    expect(newReceivedDraft(ctx, "proforma").taxPointDate).toBe("");
    const ddpp = toReceivedInput(filled({ docType: "advance_tax_doc" }), []);
    expect(ddpp.dueDate).toBeNull();
  });

  it("receivedDate follows taxPoint ?? issue until edited", () => {
    let d = applyDefaults({ ...newReceivedDraft(ctx), taxPointDate: "2026-09-30" });
    expect(d.receivedDate).toBe("2026-09-30");
    d = applyDefaults({ ...d, taxPointDate: "" , issueDate: "2026-09-29" });
    expect(d.receivedDate).toBe("2026-09-29");
    d = applyDefaults({ ...d, receivedDate: "2026-10-02", receivedDateAuto: false, issueDate: "2026-09-01" });
    expect(d.receivedDate).toBe("2026-10-02");
  });

  it("payable follows the total until edited and stays when the recap is incomplete", () => {
    let d = filled({ rounding: "-0.5" });
    expect(d.payable).toBe("1209.50");
    d = applyDefaults({ ...d, vatRecap: [{ ...d.vatRecap[0]!, base: "abc" }] });
    expect(d.payable).toBe("1209.50");
    d = applyDefaults({ ...d, payable: "500", payableAuto: false, rounding: "0" });
    expect(d.payable).toBe("500");
    expect(applyDefaults(d)).toBe(d);
  });

  it("an edited document keeps auto flags only where its values still match", () => {
    const doc = document({
      direction: "received",
      issueDate: "2026-10-01",
      taxPointDate: "2026-10-01",
      receivedDate: "2026-10-03",
      vatRecap: [{ rate: "21", base: "1000.00", vat: "210.00" }],
      rounding: "0.00",
      payable: "1210.00",
      supplierNumber: "FV-1",
      exchangeRate: "24.5",
      exchangeRateSource: "cnb",
    });
    const d = toReceivedDraft(doc, []);
    expect(d).toMatchObject({ receivedDateAuto: false, payableAuto: true, exchangeRate: "", supplierNumber: "FV-1" });
    expect(d.vatRecap[0]).toMatchObject({ vat: "210.00", vatAuto: true });
  });
});

describe("received validation + wire", () => {
  it("requires supplier, supplier number and due date (except DDPP) and checks the payable", () => {
    const d = { ...newReceivedDraft(ctx), payable: "-1", payableAuto: false };
    expect(validateReceived(d, [])).toMatchObject({
      contactId: "required",
      supplierNumber: "required",
      dueDate: "required",
      payable: "invalid",
      "vatRecap.0.base": "required",
    });
    expect(validateReceived(filled(), [])).toEqual({});
    expect(validateReceived(filled({ docType: "advance_tax_doc", dueDate: "" }), [])).toEqual({});
  });

  it("validates custom fields with the metadata", () => {
    const defs: CustomField[] = [
      { id: "f1", key: "po", label: "PO", type: "number", options: [], appliesTo: "received", required: true, active: true, position: 0 },
    ];
    const d = filled();
    expect(validateReceived({ ...d, meta: { ...d.meta, customFields: { po: "" } } }, defs)).toEqual({ "customFields.po": "required" });
  });

  it("serializes amounts with dots and the metadata inline", () => {
    const d = filled({ rounding: "-0,5", exchangeRate: "24,1", currency: "eur", supplierAccount: " 123/0100 " });
    expect(toReceivedInput({ ...d, meta: { categoryId: "k1", customFields: {}, internalNote: " x " } }, [])).toEqual({
      direction: "received",
      docType: "invoice",
      contactId: "c1",
      supplierNumber: "FV-77",
      issueDate: "2026-10-08",
      taxPointDate: "2026-10-08",
      receivedDate: "2026-10-08",
      dueDate: "2026-10-22",
      currency: "EUR",
      exchangeRate: "24.1",
      vatMode: "standard",
      vatRecap: [{ rate: "21", base: "1000", vat: "210.00" }],
      rounding: "-0.5",
      payable: "1209.50",
      vatDeductible: true,
      variableSymbol: null,
      constantSymbol: null,
      supplierAccount: "123/0100",
      relatedDocumentId: null,
      categoryId: "k1",
      customFields: {},
      internalNote: "x",
    });
  });
});
