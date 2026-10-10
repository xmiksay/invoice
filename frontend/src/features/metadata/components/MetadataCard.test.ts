import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { i18n } from "@/i18n";
import { useDocumentStore } from "@/features/documents/store";
import { document } from "@/features/documents/testData";
import { mockFetchRoutes, signIn } from "@/test-utils";
import MetadataCard from "./MetadataCard.vue";

async function mountCard() {
  const pinia = createPinia();
  setActivePinia(pinia);
  signIn("owner");
  const store = useDocumentStore();
  store.doc = document({ status: "issued", internalNote: "stored" });
  const w = mount({ components: { MetadataCard }, template: '<MetadataCard v-if="store.doc" :doc="store.doc" />', setup: () => ({ store }) }, { global: { plugins: [pinia, i18n] } });
  await flushPromises();
  return { w, store };
}

describe("MetadataCard", () => {
  beforeEach(() => {
    i18n.global.locale.value = "en";
    mockFetchRoutes({
      "GET /api/settings/categories": [],
      "GET /api/settings/custom-fields": [],
      "PUT /api/documents/d1/metadata": (body: unknown) => document({ status: "issued", ...(body as object) }),
    });
  });
  afterEach(() => vi.unstubAllGlobals());

  it("keeps unsaved edits across a reload of the same document, takes new values when clean", async () => {
    const { w, store } = await mountCard();
    const note = () => (w.find("#meta-internalNote").element as HTMLTextAreaElement).value;
    expect(note()).toBe("stored");
    expect(w.find('[data-test="save-metadata"]').attributes("disabled")).toBeDefined();

    await w.find("#meta-internalNote").setValue("typing");
    store.doc = { ...store.doc!, paid: "100.00", internalNote: "stored" };
    await flushPromises();
    expect(note()).toBe("typing");

    await w.find('[data-test="metadata-card"]').trigger("submit");
    await flushPromises();
    expect(note()).toBe("typing");
    expect(w.find('[data-test="save-metadata"]').attributes("disabled")).toBeDefined();

    store.doc = { ...store.doc!, internalNote: "changed elsewhere" };
    await flushPromises();
    expect(note()).toBe("changed elsewhere");
  });

  it("another document starts from its own values even with unsaved edits", async () => {
    const { w, store } = await mountCard();
    await w.find("#meta-internalNote").setValue("typing");
    store.doc = document({ id: "d2", status: "issued", internalNote: "other" });
    await flushPromises();
    expect((w.find("#meta-internalNote").element as HTMLTextAreaElement).value).toBe("other");
  });
});
