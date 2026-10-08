import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { createMemoryHistory, createRouter } from "vue-router";
import { i18n } from "@/i18n";
import { calls, mockFetchRoutes, reply } from "@/test-utils";
import { company, contact, document } from "../testData";
import type { Payment } from "../types";
import InvoiceDetailView from "./InvoiceDetailView.vue";

const stub = { template: "<div />" };

async function mountDetail() {
  const pinia = createPinia();
  setActivePinia(pinia);
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: "/invoices", name: "invoices", component: stub },
      { path: "/invoices/:id", name: "invoice-detail", component: InvoiceDetailView },
      { path: "/invoices/:id/edit", name: "invoice-edit", component: stub },
    ],
  });
  await router.push("/invoices/d1");
  const w = mount({ template: "<RouterView />" }, { global: { plugins: [pinia, i18n, router] } });
  await flushPromises();
  return w;
}

const issued = document({ status: "issued", number: "20260001", paymentState: "unpaid", customer: { ...contact(), registration: null, vatPayer: null } });

describe("InvoiceDetailView", () => {
  beforeEach(() => {
    i18n.global.locale.value = "en";
    vi.stubGlobal("confirm", () => true);
  });
  afterEach(() => vi.unstubAllGlobals());

  it("issues a draft and switches to the issued actions", async () => {
    const fetch = mockFetchRoutes({
      "GET /api/documents/d1": document(),
      "GET /api/settings/company": company,
      "GET /api/contacts/c1": contact(),
      "POST /api/documents/d1/issue": issued,
    });
    const w = await mountDetail();
    expect(w.find('[data-test="party-customer"]').text()).toContain("Acme");
    expect(w.find('[data-test="payments-panel"]').exists()).toBe(false);

    await w.find('[data-test="issue"]').trigger("click");
    await flushPromises();

    expect(calls(fetch)).toContain("POST /api/documents/d1/issue");
    expect(w.find("h1").text()).toContain("20260001");
    expect(w.find('[data-test="issue"]').exists()).toBe(false);
    expect(w.find('[data-test="mark-sent"]').exists()).toBe(true);
    expect(w.find('[data-test="payments-panel"]').exists()).toBe(true);
  });

  it("lists 422 reasons when issuing fails", async () => {
    mockFetchRoutes({
      "GET /api/documents/d1": document({ contactId: null }),
      "GET /api/settings/company": company,
      "POST /api/documents/d1/issue": reply(422, { code: "validation", fields: { contactId: "required", exchangeRate: "required" } }),
    });
    const w = await mountDetail();
    await w.find('[data-test="issue"]').trigger("click");
    await flushPromises();

    const items = w.findAll('[data-test="action-error"] li').map((li) => li.text());
    expect(items).toEqual(["Customer: This field is required.", "Exchange rate (CZK per unit): This field is required."]);
  });

  it("translates a 409 document_locked", async () => {
    mockFetchRoutes({
      "GET /api/documents/d1": document(),
      "GET /api/settings/company": company,
      "GET /api/contacts/c1": contact(),
      "POST /api/documents/d1/issue": reply(409, { code: "document_locked" }),
    });
    const w = await mountDetail();
    await w.find('[data-test="issue"]').trigger("click");
    await flushPromises();
    expect(w.find('[data-test="action-error"]').text()).toBe("An issued document can no longer be changed.");
  });

  it("adds a payment defaulting to the remaining amount and reloads", async () => {
    let paid = "210.00";
    const payments: Payment[] = [{ id: "p1", date: "2026-10-02", amount: "210.00", note: null, createdAt: "" }];
    const fetch = mockFetchRoutes({
      "GET /api/documents/d1": () => ({ ...issued, paid, paymentState: paid === "1210.00" ? "paid" : "partial" }),
      "GET /api/documents/d1/payments": () => payments,
      "POST /api/documents/d1/payments": (body: unknown) => {
        const p = { id: "p2", note: null, createdAt: "", ...(body as object) } as Payment;
        payments.push(p);
        paid = "1210.00";
        return reply(201, p);
      },
    });
    const w = await mountDetail();
    expect(w.findAll('[data-test="payment-row"]')).toHaveLength(1);
    expect((w.find("#payment-amount").element as HTMLInputElement).value).toBe("1000.00");

    await w.find("#payment-note").setValue("bank");
    await w.find('[data-test="payments-panel"] form').trigger("submit");
    await flushPromises();

    const post = fetch.mock.calls.find(([, init]) => init?.method === "POST");
    expect(JSON.parse(String(post?.[1]?.body))).toMatchObject({ amount: "1000.00", note: "bank" });
    expect(calls(fetch).slice(-2)).toEqual(["GET /api/documents/d1", "GET /api/documents/d1/payments"]);
    expect(w.findAll('[data-test="payment-row"]')).toHaveLength(2);
    expect(w.find('[data-test="payment-badge"]').text()).toBe("Paid");
    // Settled: the form hides behind a toggle instead of offering 0.00.
    expect(w.find("#payment-amount").exists()).toBe(false);
    await w.find('[data-test="add-another"]').trigger("click");
    expect((w.find("#payment-amount").element as HTMLInputElement).value).toBe("0.00");
  });

  it("cancels with a reason from the prompt", async () => {
    vi.stubGlobal("prompt", () => " duplicate ");
    const fetch = mockFetchRoutes({
      "GET /api/documents/d1": issued,
      "GET /api/documents/d1/payments": [],
      "POST /api/documents/d1/cancel": { ...issued, status: "cancelled", paymentState: null, cancelReason: "duplicate" },
    });
    const w = await mountDetail();
    await w.find('[data-test="cancel"]').trigger("click");
    await flushPromises();

    const post = fetch.mock.calls.find(([, init]) => init?.method === "POST");
    expect(JSON.parse(String(post?.[1]?.body))).toEqual({ reason: "duplicate" });
    expect(w.find('[data-test="status-badge"]').text()).toBe("Cancelled");
    expect(w.find('[data-test="add-payment"]').exists()).toBe(false);
  });
});
