import { describe, expect, it } from "vitest";
import { isNegative, negate, signed } from "./format";

describe("sign helpers", () => {
  it("negates decimal strings exactly; zero stays unsigned", () => {
    expect(negate("1210.00")).toBe("-1210.00");
    expect(negate("-0.10")).toBe("0.10");
    expect(negate("0.00")).toBe("0.00");
    expect(negate("123456789012345.67")).toBe("-123456789012345.67");
  });

  it("applies a document sign", () => {
    expect(signed("100.00", -1)).toBe("-100.00");
    expect(signed("100.00", 1)).toBe("100.00");
  });

  it("detects negative amounts", () => {
    expect(isNegative("-0.01")).toBe(true);
    expect(isNegative("0.00")).toBe(false);
    expect(isNegative(null)).toBe(false);
  });
});
