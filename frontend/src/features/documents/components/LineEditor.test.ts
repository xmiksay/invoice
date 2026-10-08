import { beforeEach, describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import { i18n } from "@/i18n";
import type { LineDraft } from "../lines";
import LineEditor from "./LineEditor.vue";

function mountEditor(props: { vatLocked?: boolean } = {}) {
  const w = mount(LineEditor, {
    props: {
      modelValue: [] as LineDraft[],
      "onUpdate:modelValue": (v: LineDraft[]) => void w.setProps({ modelValue: v }),
      vatOptions: ["21", "12"],
      defaultVatRate: "21",
      vatLocked: props.vatLocked ?? false,
      errors: {},
      bases: [],
    },
    global: { plugins: [i18n] },
  });
  return w;
}

const lines = (w: ReturnType<typeof mountEditor>) => w.props("modelValue") as LineDraft[];
const refs = (w: ReturnType<typeof mountEditor>, index: number) => {
  const line = lines(w)[index];
  return line?.kind === "subtotal" ? line.refs : null;
};

describe("LineEditor", () => {
  beforeEach(() => {
    i18n.global.locale.value = "en";
  });

  it("adds items with the default rate, text and subtotal lines", async () => {
    const w = mountEditor();
    await w.find('[data-test="add-item"]').trigger("click");
    await w.find('[data-test="add-text"]').trigger("click");
    await w.find('[data-test="add-subtotal"]').trigger("click");

    expect(lines(w).map((l) => l.kind)).toEqual(["item", "text", "subtotal"]);
    expect(lines(w)[0]).toMatchObject({ quantity: "1", vatRate: "21" });
    expect(refs(w, 2)).toEqual([1]);
    expect(w.findAll('[data-test="line-position"]').map((p) => p.text())).toEqual(["1.", "2.", "3."]);
    // A subtotal offers items/subtotals, never itself or text lines.
    expect(w.find('[data-test="line-2"]').findAll('input[type="checkbox"][data-test^="ref-"]').map((c) => c.attributes("data-test"))).toEqual(["ref-1"]);
  });

  it("edits item fields and selects subtotal refs", async () => {
    const w = mountEditor();
    await w.find('[data-test="add-item"]').trigger("click");
    await w.find('[data-test="add-item"]').trigger("click");
    await w.find('[data-test="add-subtotal"]').trigger("click");

    await w.find("#line-0-description").setValue("Work");
    await w.find("#line-0-unitPrice").setValue("100");
    await w.find("#line-1-vatRate").setValue("12");
    await w.find('[data-test="line-2"] [data-test="ref-2"]').setValue(true);
    await w.find('[data-test="line-2"] [data-test="ref-1"]').setValue(true);

    expect(lines(w)[0]).toMatchObject({ description: "Work", unitPrice: "100" });
    expect(lines(w)[1]).toMatchObject({ vatRate: "12" });
    expect(refs(w, 2)).toEqual([1, 2]);
  });

  it("remaps subtotal refs on reorder and delete", async () => {
    const w = mountEditor();
    for (const kind of ["item", "item", "item", "subtotal"]) await w.find(`[data-test="add-${kind}"]`).trigger("click");
    await w.find("#line-0-description").setValue("A");
    // A new subtotal starts with every item above it; untick C.
    expect(refs(w, 3)).toEqual([1, 2, 3]);
    await w.find('[data-test="line-3"] [data-test="ref-3"]').setValue(false);
    expect(refs(w, 3)).toEqual([1, 2]);

    // Move A (line 1) down: B, A, C, S → refs follow A to position 2, B to 1.
    await w.find('[data-test="line-0"] [data-test="move-down"]').trigger("click");
    expect(lines(w)[1]?.description).toBe("A");
    expect(refs(w, 3)).toEqual([1, 2]);
    // Move the subtotal to the top: S, B, A, C.
    await w.find('[data-test="line-3"] [data-test="move-up"]').trigger("click");
    await w.find('[data-test="line-2"] [data-test="move-up"]').trigger("click");
    await w.find('[data-test="line-1"] [data-test="move-up"]').trigger("click");
    expect(lines(w)[0]?.kind).toBe("subtotal");
    expect(refs(w, 0)).toEqual([2, 3]);
    expect((w.find('[data-test="line-0"] [data-test="ref-3"]').element as HTMLInputElement).checked).toBe(true);

    // Delete B (now position 2): its ref goes, A shifts to position 2.
    await w.find('[data-test="line-1"] [data-test="remove-line"]').trigger("click");
    expect(lines(w).map((l) => l.description)).toEqual(["", "A", ""]);
    expect(refs(w, 0)).toEqual([2]);
  });

  it("locks the rate select for non-payers", async () => {
    const w = mountEditor({ vatLocked: true });
    await w.find('[data-test="add-item"]').trigger("click");
    expect(w.find("#line-0-vatRate").attributes("disabled")).toBeDefined();
  });

  it("shows line errors on the matching inputs", async () => {
    const w = mountEditor();
    await w.find('[data-test="add-item"]').trigger("click");
    await w.find('[data-test="add-item"]').trigger("click");
    await w.setProps({ errors: { 1: { quantity: "invalid" } } });
    expect(w.find("#line-1-quantity-error").text()).toBe("Invalid value.");
    expect(w.find("#line-0-quantity-error").exists()).toBe(false);
  });
});
