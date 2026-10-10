import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { i18n } from "@/i18n";
import { calls, mockFetchRoutes, reply, signIn } from "@/test-utils";
import { vatRates } from "@/features/documents/testData";
import { catalogGroup, catalogItem } from "../testData";
import CatalogGroupsTab from "./CatalogGroupsTab.vue";
import CatalogItemsTab from "./CatalogItemsTab.vue";

function mountTab(component: typeof CatalogItemsTab | typeof CatalogGroupsTab) {
  const pinia = createPinia();
  setActivePinia(pinia);
  signIn("owner");
  return mount(component, { global: { plugins: [pinia, i18n] } });
}

describe("catalog tabs", () => {
  beforeEach(() => {
    i18n.global.locale.value = "en";
  });
  afterEach(() => vi.unstubAllGlobals());

  it("creates an item with the default VAT rate and reloads the list", async () => {
    const items = [catalogItem()];
    const fetch = mockFetchRoutes({
      "GET /api/catalog/items": () => items,
      "GET /api/settings/vat-rates": vatRates,
      "POST /api/catalog/items": (body: unknown) => {
        items.push(catalogItem({ id: "i9", ...(body as object) }));
        return reply(201, items.at(-1));
      },
    });
    const w = mountTab(CatalogItemsTab);
    await flushPromises();
    expect(w.findAll('[data-test="item-row"]')).toHaveLength(1);

    await w.find('[data-test="add-item"]').trigger("click");
    await flushPromises();
    await w.find("#item-name").setValue("Support");
    await w.find("#item-unitPrice").setValue("900,5");
    await w.find('[data-test="item-form"]').trigger("submit");
    await flushPromises();

    const post = fetch.mock.calls.find(([, init]) => init?.method === "POST");
    expect(JSON.parse(String(post?.[1]?.body))).toEqual({ name: "Support", unit: null, unitPrice: "900.5", currency: "CZK", vatRate: "21", active: true, note: null });
    expect(calls(fetch).at(-1)).toBe("GET /api/catalog/items");
    expect(w.findAll('[data-test="item-row"]')).toHaveLength(2);
    expect(w.find('[data-test="item-form"]').exists()).toBe(false);
  });

  it("explains catalog_item_in_use on delete and mixed_vat on an item rate change", async () => {
    vi.stubGlobal("confirm", () => true);
    mockFetchRoutes({
      "GET /api/catalog/items": [catalogItem()],
      "GET /api/settings/vat-rates": vatRates,
      "DELETE /api/catalog/items/i1": reply(409, { code: "catalog_item_in_use" }),
      "PUT /api/catalog/items/i1": reply(422, { code: "validation", fields: { vatRate: "mixed_vat" } }),
    });
    const w = mountTab(CatalogItemsTab);
    await flushPromises();
    await w.find('[data-test="item-row"] .btn-danger').trigger("click");
    await flushPromises();
    expect(w.find('[role="alert"]').text()).toBe("The item cannot be deleted: it is the only item of a group. Edit or delete the group first.");

    await w.find('[data-test="item-row"] .btn').trigger("click");
    await flushPromises();
    await w.find("#item-vatRate").setValue("12");
    await w.find('[data-test="item-form"]').trigger("submit");
    await flushPromises();
    expect(w.find("#item-vatRate-error").text()).toBe("All items of a group must share one VAT rate.");
  });

  it("group editor: picks items, sets quantities and shows the server's mixed_vat", async () => {
    const fetch = mockFetchRoutes({
      "GET /api/catalog/groups": [],
      "GET /api/catalog/items": [catalogItem(), catalogItem({ id: "i2", name: "Domain", vatRate: "21" })],
      "POST /api/catalog/groups": reply(422, { code: "validation", fields: { members: "mixed_vat" } }),
    });
    const w = mountTab(CatalogGroupsTab);
    await flushPromises();
    await w.find('[data-test="add-group"]').trigger("click");
    await flushPromises();

    await w.find("#group-name").setValue("Web");
    await w.find("#member-pick").setValue("i2");
    await w.find('[data-test="add-member"]').trigger("click");
    await w.find("#member-pick").setValue("i1");
    await w.find('[data-test="add-member"]').trigger("click");
    // A picked item is not offered again.
    expect(w.findAll("#member-pick option").map((o) => o.attributes("value"))).toEqual([""]);
    await w.find("#member-0-quantity").setValue("3");
    await w.find('[data-test="group-form"]').trigger("submit");
    await flushPromises();

    const post = fetch.mock.calls.find(([, init]) => init?.method === "POST");
    expect(JSON.parse(String(post?.[1]?.body))).toEqual({
      name: "Web",
      collapse: true,
      members: [
        { itemId: "i2", quantity: "3" },
        { itemId: "i1", quantity: "1" },
      ],
    });
    expect(w.find('[data-test="members-error"]').text()).toBe("All items of a group must share one VAT rate.");
  });

  it("lists groups with their members in order", async () => {
    mockFetchRoutes({ "GET /api/catalog/groups": [catalogGroup()] });
    const w = mountTab(CatalogGroupsTab);
    await flushPromises();
    expect(w.find('[data-test="group-row"]').text()).toContain("12× Hosting, 2× Domain");
  });
});
