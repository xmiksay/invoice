import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { i18n } from "@/i18n";
import { mockFetch, sentRequest } from "@/test-utils";
import type { NumberSeries } from "../types";
import CountersEditor from "./CountersEditor.vue";
import NumberSeriesCard from "./NumberSeriesCard.vue";

const series = (counters: NumberSeries["counters"], pattern = "{YYYY}{NNNN}"): NumberSeries => ({
  docType: "invoice",
  pattern,
  counters,
  nextNumberPreview: "",
});

function plugins() {
  const pinia = createPinia();
  setActivePinia(pinia);
  return { global: { plugins: [pinia, i18n] } };
}

const mountEditor = (s: NumberSeries) => mount(CountersEditor, { props: { series: s, currentYear: 2026 }, ...plugins() });
const mountCard = (s: NumberSeries) => mount(NumberSeriesCard, { props: { series: s, currentYear: 2026 }, ...plugins() });

const input = (w: ReturnType<typeof mountEditor>, label: string) => w.find(`input[aria-label="${label}"]`);

describe("CountersEditor", () => {
  beforeEach(() => {
    i18n.global.locale.value = "en";
  });
  afterEach(() => vi.unstubAllGlobals());

  it("saving one row keeps unsaved edits in other rows and the new-year row", async () => {
    const initial = series([
      { year: 2026, lastNumber: 41 },
      { year: 2025, lastNumber: 120 },
    ]);
    const w = mountEditor(initial);
    await input(w, "Last number for 2025").setValue("130");
    await input(w, "Year").setValue("2030");
    await input(w, "Last number for 2026").setValue("50");

    const saved = series([
      { year: 2026, lastNumber: 50 },
      { year: 2025, lastNumber: 120 },
    ]);
    const fetch = mockFetch(200, saved);
    const row2026 = w.findAll("tbody tr")[0]!;
    await row2026.find("button").trigger("click");
    await flushPromises();
    await w.setProps({ series: saved });

    expect(sentRequest(fetch)).toEqual({
      url: "/api/settings/number-series/invoice/counters/2026",
      method: "PUT",
      body: { lastNumber: 50 },
    });
    expect((input(w, "Last number for 2026").element as HTMLInputElement).value).toBe("50");
    expect((input(w, "Last number for 2025").element as HTMLInputElement).value).toBe("130");
    expect((input(w, "Year").element as HTMLInputElement).value).toBe("2030");
  });
});

describe("CountersEditor errors", () => {
  beforeEach(() => {
    i18n.global.locale.value = "en";
  });
  afterEach(() => vi.unstubAllGlobals());

  it("shows the below_issued reason when lowering a counter under issued numbers", async () => {
    const w = mountEditor(series([{ year: 2026, lastNumber: 41 }]));
    await input(w, "Last number for 2026").setValue("10");
    mockFetch(422, { code: "validation", fields: { lastNumber: "below_issued" } });
    await w.findAll("tbody tr")[0]!.find("button").trigger("click");
    await flushPromises();
    expect(w.find('[data-test="counter-error"]').text()).toBe("Cannot be lower than the highest number already issued.");
    expect((input(w, "Last number for 2026").element as HTMLInputElement).value).toBe("10");
  });
});

describe("NumberSeriesCard", () => {
  beforeEach(() => {
    i18n.global.locale.value = "en";
  });
  afterEach(() => vi.unstubAllGlobals());

  it("flags a pattern without a year token client-side", async () => {
    const w = mountCard(series([]));
    await w.find("#pattern-invoice").setValue("F{NNNN}");
    expect(w.find('[data-test="client-preview"]').text()).toBe("Invalid number pattern.");
  });

  it("renders a server duplicate-pattern 422 inline", async () => {
    const w = mountCard(series([]));
    await w.find("#pattern-invoice").setValue("D{YYYY}{NNNN}");
    mockFetch(422, { code: "validation", fields: { pattern: "duplicate" } });
    await w.find("form").trigger("submit");
    await flushPromises();
    expect(w.find("#pattern-invoice-error").text()).toBe("This value already exists.");
  });
});
