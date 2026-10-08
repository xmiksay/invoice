import type { DocLocale, FieldErrors } from "@/api/types";
import { collectErrors, nullIfEmpty, textRule } from "@/lib/formErrors";
import { validateParty, type PartyDraft } from "@/components/form/party";
import type { Company } from "./types";

export interface CompanyDraft extends PartyDraft {
  vatPayer: boolean;
  email: string;
  phone: string;
  web: string;
  registration: string;
  defaultDueDays: string;
  defaultLocale: DocLocale;
}

export function toCompanyDraft(c: Company | null): CompanyDraft {
  return {
    name: c?.name ?? "",
    ico: c?.ico ?? "",
    dic: c?.dic ?? "",
    vatPayer: c?.vatPayer ?? false,
    street: c?.street ?? "",
    city: c?.city ?? "",
    zip: c?.zip ?? "",
    country: c?.country || "CZ",
    email: c?.email ?? "",
    phone: c?.phone ?? "",
    web: c?.web ?? "",
    registration: c?.registration ?? "",
    defaultDueDays: String(c?.defaultDueDays ?? 14),
    defaultLocale: c?.defaultLocale ?? "cs",
  };
}

export function validateCompany(d: CompanyDraft): FieldErrors {
  const due = d.defaultDueDays.trim();
  return {
    ...validateParty(d),
    ...collectErrors({
      registration: textRule(d.registration, { max: 300 }),
      defaultDueDays: /^\d{1,3}$/.test(due) && Number(due) <= 365 ? null : "invalid",
    }),
  };
}

export function toCompany(d: CompanyDraft): Company {
  return {
    name: d.name.trim(),
    ico: nullIfEmpty(d.ico),
    dic: nullIfEmpty(d.dic)?.toUpperCase() ?? null,
    vatPayer: d.vatPayer,
    street: d.street.trim(),
    city: d.city.trim(),
    zip: d.zip.trim(),
    country: d.country.trim().toUpperCase(),
    email: nullIfEmpty(d.email),
    phone: nullIfEmpty(d.phone),
    web: nullIfEmpty(d.web),
    registration: nullIfEmpty(d.registration),
    defaultDueDays: Number(d.defaultDueDays.trim()),
    defaultLocale: d.defaultLocale,
  };
}
