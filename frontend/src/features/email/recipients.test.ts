import { describe, expect, it } from "vitest";
import { addAddresses, hasAddress, recipientErrors, withAddress } from "./recipients";

describe("recipients", () => {
  it("splits typed text on comma, semicolon and newline without duplicates", () => {
    expect(addAddresses(["a@x.cz"], " b@x.cz; c@x.cz,A@x.cz\n d@x.cz ")).toEqual(["a@x.cz", "b@x.cz", "c@x.cz", "d@x.cz"]);
    expect(addAddresses([], " ,; \n")).toEqual([]);
  });

  it("keeps a display name with spaces as one address", () => {
    expect(addAddresses([], "Odběratel a.s. <a@x.cz>; Jan Novák <jan@x.cz>")).toEqual(["Odběratel a.s. <a@x.cz>", "Jan Novák <jan@x.cz>"]);
  });

  it("toggles one address case-insensitively", () => {
    expect(withAddress(["a@x.cz"], "Me@x.cz", true)).toEqual(["a@x.cz", "Me@x.cz"]);
    expect(withAddress(["a@x.cz", "me@x.cz"], "ME@x.cz", true)).toEqual(["a@x.cz", "me@x.cz"]);
    expect(withAddress(["a@x.cz", "me@x.cz"], "ME@x.cz", false)).toEqual(["a@x.cz"]);
    expect(hasAddress(["me@x.cz"], " ME@X.CZ")).toBe(true);
  });

  it("splits list and per-index field errors of one list", () => {
    const fields = { to: "too_long", "to.0": "invalid", "to.2": "invalid", "cc.1": "invalid", subject: "required" };
    expect(recipientErrors(fields, "to")).toEqual({ list: "too_long", items: { 0: "invalid", 2: "invalid" } });
    expect(recipientErrors(fields, "cc")).toEqual({ list: null, items: { 1: "invalid" } });
    expect(recipientErrors(fields, "bcc")).toEqual({ list: null, items: {} });
  });
});
