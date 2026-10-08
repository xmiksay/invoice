import { describe, expect, it } from "vitest";
import { isValidIco } from "./ico";

describe("isValidIco", () => {
  it.each(["25596641", "26168685", "00006947", "27082440"])("accepts %s", (ico) => {
    expect(isValidIco(ico)).toBe(true);
  });

  it.each([
    ["10000003", "remainder 8 → 3"],
    ["00000019", "remainder 2 → 9"],
    ["00000001", "remainder 0 → 1"],
    ["00000060", "remainder 1 → 0"],
  ])("handles check digit edge case %s (%s)", (ico) => {
    expect(isValidIco(ico)).toBe(true);
  });

  it.each(["25596642", "2559664", "255966410", "2559664a", "", " 25596641"])("rejects %j", (ico) => {
    expect(isValidIco(ico)).toBe(false);
  });
});
