import { defineStore } from "pinia";
import { ref } from "vue";
import type { DocLocale } from "@/api/types";
import { emailApi } from "./api";
import type { EmailLogEntry, EmailSettings, EmailTemplate, TemplateText } from "./types";

/** SMTP status (env only, so it never changes while the app runs) and the editable templates. */
export const useEmailSettingsStore = defineStore("email/settings", () => {
  const settings = ref<EmailSettings | null>(null);
  const templates = ref<EmailTemplate[]>([]);

  async function load(): Promise<void> {
    settings.value = await emailApi.settings();
  }

  /** Loads once; the send button calls this on every detail view. */
  async function ensureLoaded(): Promise<void> {
    if (!settings.value) await load();
  }

  async function loadTemplates(): Promise<void> {
    templates.value = (await emailApi.templates()).templates;
  }

  function put(entry: EmailTemplate): EmailTemplate {
    templates.value = templates.value.map((t) => (t.locale === entry.locale ? entry : t));
    return entry;
  }

  const saveTemplate = async (locale: DocLocale, input: TemplateText) => put(await emailApi.saveTemplate(locale, input));
  const restoreTemplate = async (locale: DocLocale) => put(await emailApi.restoreTemplate(locale));

  return { settings, templates, load, ensureLoaded, loadTemplates, saveTemplate, restoreTemplate };
});

/** Send attempts of the document open in the detail view. */
export const useEmailHistoryStore = defineStore("email/history", () => {
  const documentId = ref<string | null>(null);
  const entries = ref<EmailLogEntry[]>([]);

  // Drops responses that arrive after a newer load (another document, or a reload) was started.
  let seq = 0;

  async function load(id: string): Promise<void> {
    const mine = ++seq;
    if (documentId.value !== id) entries.value = [];
    documentId.value = id;
    const loaded = await emailApi.history(id);
    if (mine === seq) entries.value = loaded;
  }

  return { documentId, entries, load };
});
