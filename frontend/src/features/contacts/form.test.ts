import { describe, expect, it } from "vitest";
import { toDraft, toInput, validateContact } from "./form";

describe("contact form", () => {
  it("requires a name and validates IČO, currency and due days", () => {
    const errors = validateContact({
      ...toDraft(),
      ico: "12345678",
      defaultCurrency: "EU",
      defaultDueDays: "-1",
    });
    expect(errors).toEqual({
      name: "required",
      ico: "invalid_ico",
      defaultCurrency: "invalid",
      defaultDueDays: "invalid",
    });
  });

  it("passes a minimal valid draft", () => {
    expect(validateContact({ ...toDraft(), name: "Acme" })).toEqual({});
  });

  it("converts blanks to null and normalizes case", () => {
    const input = toInput({
      ...toDraft(),
      name: " Acme ",
      dic: "cz25596641",
      country: "sk",
      defaultDueDays: "30",
      defaultCurrency: "eur",
      defaultLocale: "en",
    });
    expect(input).toEqual({
      name: "Acme",
      ico: null,
      dic: "CZ25596641",
      street: "",
      city: "",
      zip: "",
      country: "SK",
      email: null,
      phone: null,
      note: null,
      defaultDueDays: 30,
      defaultLocale: "en",
      defaultCurrency: "EUR",
    });
  });
});
