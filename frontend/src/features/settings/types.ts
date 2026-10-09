import type { DocLocale } from "@/api/types";

/** `GET/PUT /api/settings/company` — singleton, seeded with name "". */
export interface Company {
  name: string;
  ico: string | null;
  dic: string | null;
  vatPayer: boolean;
  street: string;
  city: string;
  zip: string;
  country: string;
  email: string | null;
  phone: string | null;
  web: string | null;
  registration: string | null;
  defaultDueDays: number;
  defaultLocale: DocLocale;
}

/** `/api/settings/bank-accounts` */
export interface BankAccount {
  id: string;
  label: string | null;
  currency: string;
  accountNumber: string | null;
  iban: string | null;
  bic: string | null;
  isDefault: boolean;
}
export type BankAccountInput = Omit<BankAccount, "id">;

/** `/api/settings/vat-rates` — `rate` is a decimal string, e.g. "21" or "12.5". */
export interface VatRate {
  id: string;
  rate: string;
  label: string;
  isDefault: boolean;
  active: boolean;
  position: number;
}
export type VatRateInput = Omit<VatRate, "id">;

/** Number-series keys of issued documents, in display order. */
export const ISSUED_SERIES = [
  "invoice",
  "simplified",
  "proforma",
  "credit_note",
  "debit_note",
  "advance_tax_doc",
  "advance_credit_note",
] as const;
/** The series of received documents (1e, 1f-a); `received` is the received invoices'. */
export const RECEIVED_SERIES = [
  "received",
  "received_simplified",
  "received_proforma",
  "received_credit_note",
  "received_debit_note",
  "received_advance_tax_doc",
  "received_advance_credit_note",
] as const;
/** All fourteen number-series keys. */
export const DOC_TYPES = [...ISSUED_SERIES, ...RECEIVED_SERIES] as const;
export type DocType = (typeof DOC_TYPES)[number];

export interface NumberCounter {
  year: number;
  lastNumber: number;
}

/** `/api/settings/number-series` — `nextNumberPreview` is for the current year. */
export interface NumberSeries {
  docType: DocType;
  pattern: string;
  counters: NumberCounter[];
  nextNumberPreview: string;
}

/** `GET /api/pdf/design` — the effective design files (built-in ∪ `INVOICE__DESIGN_DIR`), sorted by path. */
export interface DesignInfo {
  /** null = only the built-in design. */
  designDir: string | null;
  files: DesignFile[];
}
export interface DesignFile {
  path: string;
  /** `custom` = from the design dir (overrides the built-in file of the same path). */
  source: "custom" | "default";
  /** Bytes. */
  size: number;
}

export const CATEGORY_KINDS = ["expense", "income"] as const;
export type CategoryKind = (typeof CATEGORY_KINDS)[number];

/** `/api/settings/categories` — received documents take `expense`, issued `income`. */
export interface Category {
  id: string;
  name: string;
  kind: CategoryKind;
  active: boolean;
  position: number;
}
export type CategoryInput = Omit<Category, "id">;

export const CUSTOM_FIELD_TYPES = ["text", "number", "date", "bool", "select"] as const;
export type CustomFieldType = (typeof CUSTOM_FIELD_TYPES)[number];
export const CUSTOM_FIELD_SCOPES = ["issued", "received", "both"] as const;
export type CustomFieldScope = (typeof CUSTOM_FIELD_SCOPES)[number];

/** `/api/settings/custom-fields` — `key` and `type` are immutable after create. */
export interface CustomField {
  id: string;
  key: string;
  label: string;
  type: CustomFieldType;
  /** `select` only. */
  options: string[];
  appliesTo: CustomFieldScope;
  required: boolean;
  active: boolean;
  position: number;
}
export type CustomFieldInput = Omit<CustomField, "id">;
