import { describe, expect, it } from "vitest";
import { toCompany, toCompanyDraft, validateCompany } from "./companyForm";

describe("company form", () => {
  it("defaults an empty seeded row to CZ / 14 days / cs", () => {
    const d = toCompanyDraft(null);
    expect([d.country, d.defaultDueDays, d.defaultLocale]).toEqual(["CZ", "14", "cs"]);
  });

  it("validates name, IČO and due days range", () => {
    const errors = validateCompany({ ...toCompanyDraft(null), ico: "25596642", defaultDueDays: "366" });
    expect(errors).toEqual({ name: "required", ico: "invalid_ico", defaultDueDays: "invalid" });
  });

  it("round-trips to the wire shape with nulls for blanks", () => {
    const company = toCompany({ ...toCompanyDraft(null), name: "Me", ico: "25596641", vatPayer: true });
    expect(company).toMatchObject({ name: "Me", ico: "25596641", dic: null, vatPayer: true, defaultDueDays: 14, web: null });
  });
});
