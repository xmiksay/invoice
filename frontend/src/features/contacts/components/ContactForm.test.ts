import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { i18n } from "@/i18n";
import { mockFetch, sentRequest } from "@/test-utils";
import ContactForm from "./ContactForm.vue";

const subject = {
  name: "Acme s.r.o.",
  ico: "25596641",
  dic: "CZ25596641",
  street: "Hlavní 1/2a",
  city: "Praha",
  zip: "11000",
  country: "CZ",
};

function mountForm() {
  const pinia = createPinia();
  setActivePinia(pinia);
  return mount(ContactForm, { global: { plugins: [pinia, i18n] } });
}

const value = (w: ReturnType<typeof mountForm>, id: string) =>
  (w.find(`#${id}`).element as HTMLInputElement).value;

describe("ContactForm", () => {
  beforeEach(() => {
    i18n.global.locale.value = "en";
  });
  afterEach(() => vi.unstubAllGlobals());

  it("prefills identity and address from ARES by IČO", async () => {
    const w = mountForm();
    await w.find("#contact-email").setValue("keep@example.com");
    await w.find("#contact-ico").setValue("25596641");
    const fetch = mockFetch(200, subject);

    await w.find('[data-test="ares-button"]').trigger("click");
    await flushPromises();

    expect(sentRequest(fetch).url).toBe("/api/ares/25596641");
    expect(value(w, "contact-name")).toBe("Acme s.r.o.");
    expect(value(w, "contact-dic")).toBe("CZ25596641");
    expect(value(w, "contact-street")).toBe("Hlavní 1/2a");
    expect(value(w, "contact-city")).toBe("Praha");
    expect(value(w, "contact-zip")).toBe("11000");
    // Fields ARES does not know about stay as typed.
    expect(value(w, "contact-email")).toBe("keep@example.com");
  });

  it("keeps edits typed while the ARES lookup is pending", async () => {
    const w = mountForm();
    await w.find("#contact-ico").setValue("25596641");
    let respond: (r: Response) => void = () => {};
    vi.stubGlobal(
      "fetch",
      vi.fn(() => new Promise<Response>((resolve) => (respond = resolve))),
    );

    await w.find('[data-test="ares-button"]').trigger("click");
    await w.find("#contact-email").setValue("typed@example.com");
    respond(new Response(JSON.stringify(subject), { status: 200, headers: { "Content-Type": "application/json" } }));
    await flushPromises();

    expect(value(w, "contact-name")).toBe("Acme s.r.o.");
    expect(value(w, "contact-email")).toBe("typed@example.com");
  });

  it("does not call ARES for an IČO with a bad checksum", async () => {
    const w = mountForm();
    const fetch = mockFetch(200, subject);
    await w.find("#contact-ico").setValue("25596642");
    await w.find('[data-test="ares-button"]').trigger("click");
    await flushPromises();

    expect(fetch).not.toHaveBeenCalled();
    expect(w.find('[data-test="ares-error"]').text()).toBe("Invalid company ID (IČO).");
  });

  it("shows the ARES not-found message", async () => {
    const w = mountForm();
    mockFetch(404, { code: "ares_not_found" });
    await w.find("#contact-ico").setValue("25596641");
    await w.find('[data-test="ares-button"]').trigger("click");
    await flushPromises();

    expect(w.find('[data-test="ares-error"]').text()).toBe("Subject not found in ARES.");
  });

  it("blocks submit on client-side errors without a request", async () => {
    const w = mountForm();
    const fetch = mockFetch(200, {});
    await w.find("form").trigger("submit");
    await flushPromises();

    expect(fetch).not.toHaveBeenCalled();
    expect(w.find("#contact-name-error").text()).toBe("This field is required.");
  });

  it("maps 422 field errors from the server inline", async () => {
    const w = mountForm();
    await w.find("#contact-name").setValue("Acme");
    await w.find("#contact-ico").setValue("25596641");
    const fetch = mockFetch(422, { code: "validation", fields: { ico: "duplicate", zip: "too_long" } });

    await w.find("form").trigger("submit");
    await flushPromises();

    expect(sentRequest(fetch)).toMatchObject({ url: "/api/contacts", method: "POST" });
    expect(w.find("#contact-ico-error").text()).toBe("This value already exists.");
    expect(w.find("#contact-zip-error").text()).toBe("The value is too long.");
    expect(w.find('[data-test="form-error"]').text()).toBe("The form contains errors.");
    expect(w.emitted("saved")).toBeUndefined();
  });

  it("emits saved with the created contact", async () => {
    const w = mountForm();
    await w.find("#contact-name").setValue("Acme");
    mockFetch(200, { id: "c1", name: "Acme" });
    await w.find("form").trigger("submit");
    await flushPromises();

    expect(w.emitted("saved")?.[0]).toEqual([{ id: "c1", name: "Acme" }]);
  });
});
