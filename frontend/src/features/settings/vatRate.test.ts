import { describe, expect, it } from "vitest";
import { normalizeRate, toVatRateInput, validateVatRate } from "./vatRate";

describe("vat rate", () => {
  it.each(["0", "21", "12.5", "12,5", "100", "99.99"])("accepts %s", (rate) => {
    expect(validateVatRate({ rate, label: "x" })).toEqual({});
  });

  it.each(["100.01", "-1", "1.234", "abc", "1000"])("rejects %s", (rate) => {
    expect(validateVatRate({ rate, label: "x" })).toEqual({ rate: "invalid" });
  });

  it("requires rate and label", () => {
    expect(validateVatRate({ rate: " ", label: "" })).toEqual({ rate: "required", label: "required" });
  });

  it("rejects an inactive default", () => {
    expect(validateVatRate({ rate: "21", label: "x", isDefault: true, active: false })).toEqual({ isDefault: "invalid" });
    expect(validateVatRate({ rate: "21", label: "x", isDefault: false, active: false })).toEqual({});
  });

  it("normalizes a decimal comma and strips the id for the body", () => {
    expect(normalizeRate(" 12,5 ")).toBe("12.5");
    expect(toVatRateInput({ id: "v", rate: "21", label: "Z", isDefault: true, active: true, position: 0 })).toEqual({
      rate: "21",
      label: "Z",
      isDefault: true,
      active: true,
      position: 0,
    });
  });
});
