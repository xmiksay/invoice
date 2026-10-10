import { describe, expect, it } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { i18n } from "@/i18n";
import { useSessionStore } from "@/stores/session";
import BootFailureView from "./BootFailureView.vue";

function mountWith(boot: Parameters<ReturnType<typeof useSessionStore>["applyBoot"]>[0]) {
  const pinia = createPinia();
  setActivePinia(pinia);
  useSessionStore().applyBoot(boot);
  return mount(BootFailureView, { global: { plugins: [pinia, i18n] } });
}

describe("BootFailureView", () => {
  it("shows 'space not found' with a link to the base host", async () => {
    i18n.global.locale.value = "cs";
    const w = mountWith({ status: "not_found", baseUrl: "https://invoiceapp.cz" });
    await flushPromises();
    expect(w.find("h1").text()).toBe("Space nenalezen");
    expect(w.find('[data-test="base-link"]').attributes("href")).toBe("https://invoiceapp.cz");
  });

  it("shows the unreachable message otherwise", () => {
    const w = mountWith({ status: "error" });
    expect(w.find('[data-test="boot-error"]').exists()).toBe(true);
  });
});
