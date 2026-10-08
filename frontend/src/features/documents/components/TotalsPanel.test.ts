import { beforeEach, describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import { i18n } from "@/i18n";
import { totals } from "../testData";
import TotalsPanel from "./TotalsPanel.vue";

const text = (w: ReturnType<typeof mount>, sel: string) => w.find(sel).text().replace(/\s/g, " ");

describe("TotalsPanel", () => {
  beforeEach(() => {
    i18n.global.locale.value = "en";
  });

  it("negative payable after advances reads as overpaid, shown positive", () => {
    const w = mount(TotalsPanel, { props: { totals: totals("-90.00"), currency: "CZK" }, global: { plugins: [i18n] } });
    expect(text(w, '[data-test="payable-label"]')).toBe("Overpaid");
    expect(text(w, '[data-test="payable"]')).toBe("CZK 90.00");
  });

  it("credit notes show every amount negated", () => {
    const w = mount(TotalsPanel, { props: { totals: totals(), currency: "CZK", sign: -1 }, global: { plugins: [i18n] } });
    expect(text(w, '[data-test="payable-label"]')).toBe("Amount due");
    expect(text(w, '[data-test="payable"]')).toBe("-CZK 1,210.00");
    expect(text(w, '[data-test="recap-row"]')).toContain("-CZK 1,000.00");
  });
});
