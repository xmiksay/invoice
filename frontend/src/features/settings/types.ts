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

export const DOC_TYPES = ["invoice", "credit_note", "proforma", "advance_tax_doc", "received"] as const;
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
