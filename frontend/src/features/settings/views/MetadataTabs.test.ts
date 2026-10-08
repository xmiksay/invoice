import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { i18n } from "@/i18n";
import { calls, mockFetchRoutes, reply } from "@/test-utils";
import type { Category, CustomField } from "../types";
import CategoriesTab from "./CategoriesTab.vue";
import CustomFieldsTab from "./CustomFieldsTab.vue";

function mountTab(component: typeof CategoriesTab | typeof CustomFieldsTab) {
  const pinia = createPinia();
  setActivePinia(pinia);
  return mount(component, { global: { plugins: [pinia, i18n] } });
}

const category = (overrides: Partial<Category> = {}): Category => ({ id: "k1", name: "Software", kind: "expense", active: true, position: 0, ...overrides });
const customField = (overrides: Partial<CustomField> = {}): CustomField => ({
  id: "f1",
  key: "project",
  label: "Project",
  type: "select",
  options: ["A", "B"],
  appliesTo: "both",
  required: false,
  active: true,
  position: 0,
  ...overrides,
});

const bodyOf = (fetch: ReturnType<typeof mockFetchRoutes>, method: string) =>
  JSON.parse(String(fetch.mock.calls.find(([, init]) => init?.method === method)?.[1]?.body));

describe("settings: categories", () => {
  beforeEach(() => {
    i18n.global.locale.value = "en";
  });
  afterEach(() => vi.unstubAllGlobals());

  it("creates a category and reloads the list", async () => {
    const items = [category()];
    const fetch = mockFetchRoutes({
      "GET /api/settings/categories": () => items,
      "POST /api/settings/categories": (body: unknown) => {
        items.push(category({ id: "k2", ...(body as object) }));
        return reply(201, items.at(-1));
      },
    });
    const w = mountTab(CategoriesTab);
    await flushPromises();
    await w.find('[data-test="add-category"]').trigger("click");
    await w.find("#category-name").setValue(" Consulting ");
    await w.find("#category-kind").setValue("income");
    await w.find('[data-test="category-form"]').trigger("submit");
    await flushPromises();

    expect(bodyOf(fetch, "POST")).toEqual({ name: "Consulting", kind: "income", active: true, position: 1 });
    expect(w.findAll('[data-test="category-row"]')).toHaveLength(2);
    expect(w.find('[data-test="category-form"]').exists()).toBe(false);
  });

  it("requires a name client-side", async () => {
    const fetch = mockFetchRoutes({ "GET /api/settings/categories": [] });
    const w = mountTab(CategoriesTab);
    await flushPromises();
    await w.find('[data-test="add-category"]').trigger("click");
    await w.find('[data-test="category-form"]').trigger("submit");
    expect(w.find('[data-test="field-error"]').text()).toBe("This field is required.");
    expect(calls(fetch)).toEqual(["GET /api/settings/categories"]);
  });

  it("explains category_in_use with the deactivate hint and toggles active via PUT", async () => {
    const fetch = mockFetchRoutes({
      "GET /api/settings/categories": [category()],
      "DELETE /api/settings/categories/k1": reply(409, { code: "category_in_use" }),
      "PUT /api/settings/categories/k1": category({ active: false }),
    });
    vi.stubGlobal("confirm", vi.fn(() => true));
    const w = mountTab(CategoriesTab);
    await flushPromises();
    await w.find('[data-test="delete-category"]').trigger("click");
    await flushPromises();
    expect(w.find('[data-test="categories-error"]').text()).toContain("Deactivate it instead.");

    await w.find('[data-test="category-row"] input[type="checkbox"]').trigger("change");
    await flushPromises();
    expect(bodyOf(fetch, "PUT")).toEqual({ name: "Software", kind: "expense", active: false, position: 0 });
  });
});

describe("settings: custom fields", () => {
  beforeEach(() => {
    i18n.global.locale.value = "en";
  });
  afterEach(() => vi.unstubAllGlobals());

  it("creates a select field with options one per line", async () => {
    const items: CustomField[] = [];
    const fetch = mockFetchRoutes({
      "GET /api/settings/custom-fields": () => items,
      "POST /api/settings/custom-fields": (body: unknown) => {
        items.push(customField(body as Partial<CustomField>));
        return reply(201, items.at(-1));
      },
    });
    const w = mountTab(CustomFieldsTab);
    await flushPromises();
    await w.find('[data-test="add-custom-field"]').trigger("click");
    await w.find("#cf-key").setValue("project");
    await w.find("#cf-label").setValue("Project");
    await w.find("#cf-type").setValue("select");
    await w.find("#cf-options").setValue("A\n\n B \n");
    await w.find("#cf-appliesTo").setValue("received");
    await w.find("#cf-required").setValue(true);
    await w.find('[data-test="custom-field-form"]').trigger("submit");
    await flushPromises();

    expect(bodyOf(fetch, "POST")).toEqual({
      key: "project",
      label: "Project",
      type: "select",
      options: ["A", "B"],
      appliesTo: "received",
      required: true,
      active: true,
      position: 0,
    });
    expect(w.findAll('[data-test="custom-field-row"]')).toHaveLength(1);
  });

  it("validates the key pattern and select options", async () => {
    mockFetchRoutes({ "GET /api/settings/custom-fields": [] });
    const w = mountTab(CustomFieldsTab);
    await flushPromises();
    await w.find('[data-test="add-custom-field"]').trigger("click");
    await w.find("#cf-key").setValue("Project-1");
    await w.find("#cf-label").setValue("P");
    await w.find("#cf-type").setValue("select");
    await w.find('[data-test="custom-field-form"]').trigger("submit");
    expect(w.findAll('[data-test="field-error"]').map((e) => e.text())).toEqual(["Invalid value.", "This field is required."]);
  });

  it("locks key and type when editing", async () => {
    const fetch = mockFetchRoutes({
      "GET /api/settings/custom-fields": [customField()],
      "PUT /api/settings/custom-fields/f1": customField({ label: "Proj" }),
    });
    const w = mountTab(CustomFieldsTab);
    await flushPromises();
    await w.find('[data-test="custom-field-row"] .btn').trigger("click");
    expect(w.find("#cf-key").attributes("disabled")).toBeDefined();
    expect(w.find("#cf-type").attributes("disabled")).toBeDefined();
    await w.find("#cf-label").setValue("Proj");
    await w.find('[data-test="custom-field-form"]').trigger("submit");
    await flushPromises();
    expect(bodyOf(fetch, "PUT")).toMatchObject({ key: "project", type: "select", label: "Proj" });
  });
});
