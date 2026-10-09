import { request } from "@/api/client";
import type { DocLocale } from "@/api/types";
import type { EmailLogEntry, EmailPrefill, EmailSettings, EmailTemplate, SendEmailInput, TemplatesResponse, TemplateText } from "./types";

const SETTINGS = "/api/settings/email";
const template = (locale: DocLocale) => `${SETTINGS}/templates/${locale}`;
const doc = (id: string) => `/api/documents/${encodeURIComponent(id)}`;

export const emailApi = {
  settings: () => request<EmailSettings>(SETTINGS),
  /** Without `to` the server sends to the company e-mail. */
  test: (to: string | null) => request<void>(`${SETTINGS}/test`, { method: "POST", body: to ? { to } : {} }),
  templates: () => request<TemplatesResponse>(`${SETTINGS}/templates`),
  saveTemplate: (locale: DocLocale, input: TemplateText) => request<EmailTemplate>(template(locale), { method: "PUT", body: input }),
  /** Removes the override; answers the default entry. */
  restoreTemplate: (locale: DocLocale) => request<EmailTemplate>(template(locale), { method: "DELETE" }),
  previewTemplate: (locale: DocLocale, input: TemplateText) =>
    request<TemplateText>(`${template(locale)}/preview`, { method: "POST", body: input }),
  /** Without `locale` the server uses the document's locale. */
  prefill: (id: string, locale?: DocLocale) => request<EmailPrefill>(`${doc(id)}/email${locale ? `?locale=${locale}` : ""}`),
  send: (id: string, input: SendEmailInput) => request<EmailLogEntry>(`${doc(id)}/email`, { method: "POST", body: input }),
  history: (id: string) => request<EmailLogEntry[]>(`${doc(id)}/emails`),
};
