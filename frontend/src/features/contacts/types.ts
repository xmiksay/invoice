import type { DocLocale } from "@/api/types";

/** `/api/contacts` — one address book for customers and suppliers. */
export interface Contact {
  id: string;
  name: string;
  ico: string | null;
  dic: string | null;
  street: string;
  city: string;
  zip: string;
  country: string;
  email: string | null;
  phone: string | null;
  note: string | null;
  defaultDueDays: number | null;
  defaultLocale: DocLocale | null;
  defaultCurrency: string | null;
  createdAt: string;
  updatedAt: string;
}

export type ContactInput = Omit<Contact, "id" | "createdAt" | "updatedAt">;

export interface ContactListQuery {
  q?: string;
  limit: number;
  offset: number;
}

/** `GET /api/contacts` */
export interface ContactPage {
  items: Contact[];
  total: number;
}
