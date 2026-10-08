import type { AresSubject, FieldErrors } from "@/api/types";
import { isValidIco } from "@/lib/ico";
import { textRule } from "@/lib/formErrors";

/** Identity + address block shared by the company profile and contacts (form state, all strings). */
export interface PartyDraft {
  name: string;
  ico: string;
  dic: string;
  street: string;
  city: string;
  zip: string;
  country: string;
}

export function validateParty(d: PartyDraft): FieldErrors {
  const out: FieldErrors = {};
  const name = textRule(d.name, { required: true, max: 200 });
  if (name) out.name = name;
  const ico = d.ico.trim();
  if (ico !== "" && !isValidIco(ico)) out.ico = "invalid_ico";
  if (d.dic.trim().length > 14) out.dic = "too_long";
  if (!/^[A-Za-z]{2}$/.test(d.country.trim())) out.country = "invalid";
  return out;
}

/** ARES fills identity + address; other fields of a larger draft stay untouched. */
export function applyAres<T extends PartyDraft>(d: T, s: AresSubject): T {
  return {
    ...d,
    name: s.name,
    ico: s.ico,
    dic: s.dic ?? "",
    street: s.street,
    city: s.city,
    zip: s.zip,
    country: s.country,
  };
}
