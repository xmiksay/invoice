import { request } from "@/api/client";
import type { Contact, ContactInput, ContactListQuery, ContactPage } from "./types";

const BASE = "/api/contacts";
const item = (id: string) => `${BASE}/${encodeURIComponent(id)}`;

export const contactsApi = {
  list({ q, limit, offset }: ContactListQuery): Promise<ContactPage> {
    const params = new URLSearchParams();
    if (q?.trim()) params.set("q", q.trim());
    params.set("limit", String(limit));
    params.set("offset", String(offset));
    return request<ContactPage>(`${BASE}?${params.toString()}`);
  },
  get: (id: string) => request<Contact>(item(id)),
  create: (input: ContactInput) => request<Contact>(BASE, { method: "POST", body: input }),
  update: (id: string, input: ContactInput) => request<Contact>(item(id), { method: "PUT", body: input }),
  remove: (id: string) => request<void>(item(id), { method: "DELETE" }),
};
