import { afterEach, describe, expect, it } from "vitest";
import { i18n } from "@/i18n";
import { localizeTemplateDetail } from "./templateDetail";

const t = (key: string, params?: Record<string, unknown>) => i18n.global.t(key, params ?? {});

describe("localizeTemplateDetail", () => {
  afterEach(() => {
    i18n.global.locale.value = "en";
  });

  it("localizes the contactless prefix and the line, with a guard hint", () => {
    i18n.global.locale.value = "cs";
    expect(localizeTemplateDetail("without contact: line 12: undefined value", t)).toEqual({
      text: "Na dokladu bez kontaktu: řádek 12: undefined value",
      hint: "Zjednodušený doklad nemusí mít kontakt – každé použití contact ošetřete podmínkou {% if contact %}…{% endif %}.",
    });
    i18n.global.locale.value = "en";
    expect(localizeTemplateDetail("without contact: line 12: undefined value", t).text).toBe("On a document without a contact: line 12: undefined value");
  });

  it.each([
    ["paid: line 2: x", "Na uhrazeném dokladu: řádek 2: x"],
    ["cancelled: line 2: x", "Na stornovaném dokladu: řádek 2: x"],
    ["credit note: line 2: x", "Na dobropisu: řádek 2: x"],
    ["no bank account: line 2: x", "Na dokladu bez bankovního účtu: řádek 2: x"],
  ])("localizes the sample prefix of %s", (detail, text) => {
    i18n.global.locale.value = "cs";
    expect(localizeTemplateDetail(detail, t)).toEqual({ text, hint: null });
  });

  it("keeps a plain detail without a hint", () => {
    i18n.global.locale.value = "cs";
    expect(localizeTemplateDetail("line 3: syntax error", t)).toEqual({ text: "řádek 3: syntax error", hint: null });
    expect(localizeTemplateDetail("unexpected end", t)).toEqual({ text: "unexpected end", hint: null });
  });
});
