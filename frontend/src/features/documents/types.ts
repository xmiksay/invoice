import type { DocLocale } from "@/api/types";
import type { DocType } from "@/features/settings/types";

/** Wire types for `/api/documents` (Phases 1b/1c). Decimals are strings, dates `YYYY-MM-DD`. */

export type Direction = "issued" | "received";
export const DOC_STATUSES = ["draft", "issued", "cancelled"] as const;
export type DocStatus = (typeof DOC_STATUSES)[number];
export const PAYMENT_STATES = ["unpaid", "partial", "paid", "overpaid"] as const;
export type PaymentState = (typeof PAYMENT_STATES)[number];
export const VAT_MODES = ["standard", "reverse_charge", "exempt", "non_payer"] as const;
export type VatMode = (typeof VAT_MODES)[number];
export const PAYMENT_METHODS = ["bank_transfer", "cash", "card", "other"] as const;
export type PaymentMethod = (typeof PAYMENT_METHODS)[number];

export interface ItemLine {
  kind: "item";
  description: string;
  quantity: string;
  unit: string | null;
  unitPrice: string;
  discountPct: string;
  vatRate: string;
}
export interface TextLine {
  kind: "text";
  description: string;
}
export interface SubtotalLine {
  kind: "subtotal";
  description: string;
  /** 1-based positions of other lines. */
  refs: number[];
  collapse: boolean;
}
/** Deducts an issued DDPP (or, for non-payers, the proforma itself) on a final invoice. */
export interface AdvanceLine {
  kind: "advance";
  advanceDocumentId: string;
}
export type DocumentLine = ItemLine | TextLine | SubtotalLine | AdvanceLine;

/** Deducted amounts of an advance line per VAT rate (negative). */
export interface AdvanceRecapRow {
  vatRate: string;
  base: string;
  vat: string;
}

/** Server-generated display fields of an advance line. */
export interface AdvanceDisplay {
  description: string;
  base: string;
  recap: AdvanceRecapRow[];
}

/** Response lines carry their position and, for item/subtotal/advance, the computed base. */
export type ComputedLine =
  | (ItemLine & { position: number; base: string })
  | (TextLine & { position: number })
  | (SubtotalLine & { position: number; base: string; vatRate?: string | null })
  | (AdvanceLine & AdvanceDisplay & { position: number });

/** Doc types the editor can create or edit (DDPPs are server-made, credit notes come from an invoice). */
export type EditableDocType = "invoice" | "proforma" | "credit_note";

export interface DocumentInput {
  docType: DocType;
  direction: Direction;
  contactId: string | null;
  issueDate: string;
  taxPointDate: string | null;
  dueDate: string;
  currency: string;
  exchangeRate: string | null;
  locale: DocLocale;
  vatMode: VatMode;
  bankAccountId: string | null;
  paymentMethod: PaymentMethod;
  variableSymbol: string | null;
  constantSymbol: string | null;
  orderRef: string | null;
  headerNote: string | null;
  footerNote: string | null;
  internalNote: string | null;
  roundTotal: boolean;
  /** Credit notes only; required at issue. */
  correctionReason: string | null;
  lines: DocumentLine[];
}

export interface RecapRow {
  vatRate: string;
  base: string;
  vat: string;
  baseCzk: string | null;
  vatCzk: string | null;
}

export interface Totals {
  recap: RecapRow[];
  base: string;
  vat: string;
  total: string;
  rounding: string;
  payable: string;
  totalCzk: string | null;
}

export interface PartySnapshot {
  name: string;
  ico: string | null;
  dic: string | null;
  street: string;
  city: string;
  zip: string;
  country: string;
  registration: string | null;
  vatPayer: boolean | null;
}

export interface BankSnapshot {
  accountNumber: string | null;
  iban: string | null;
  bic: string | null;
}

export interface Document extends Omit<DocumentInput, "lines"> {
  id: string;
  number: string | null;
  status: DocStatus;
  paymentState: PaymentState | null;
  overdue: boolean;
  sentAt: string | null;
  cancelledAt: string | null;
  cancelReason: string | null;
  exchangeRateDate: string | null;
  /** `original`: a credit note's rate copied from its invoice. */
  exchangeRateSource: "cnb" | "manual" | "original" | null;
  supplier: PartySnapshot | null;
  customer: PartySnapshot | null;
  bankSnapshot: BankSnapshot | null;
  lines: ComputedLine[];
  totals: Totals;
  paid: string;
  /** credit_note → invoice, invoice → the proforma it settles, DDPP → proforma. */
  relatedDocumentId: string | null;
  /** DDPP only: the proforma payment it documents. */
  paymentId: string | null;
  /** The document `relatedDocumentId` points at (invoice of a credit note, proforma of a DDPP / final invoice). */
  parent: RelatedDocument | null;
  /** Documents whose `relatedDocumentId` is this one. */
  relatedDocuments: RelatedDocument[];
  /** Proforma only: a non-cancelled invoice settles it. */
  settled: boolean | null;
  /** -1 for credit notes: amounts are stored positive, shown negated. */
  sign: Sign;
  createdAt: string;
  updatedAt: string;
}

export type Sign = 1 | -1;

export interface RelatedDocument {
  id: string;
  docType: DocType;
  number: string | null;
  status: DocStatus;
  payable: string;
}

export interface DocumentSummary {
  id: string;
  docType: DocType;
  direction: Direction;
  number: string | null;
  status: DocStatus;
  paymentState: PaymentState | null;
  overdue: boolean;
  contactId: string | null;
  customerName: string | null;
  issueDate: string;
  dueDate: string;
  currency: string;
  payable: string;
  paid: string;
  sentAt: string | null;
  sign: Sign;
  relatedDocumentId: string | null;
}

export interface DocumentPage {
  items: DocumentSummary[];
  total: number;
}

export interface DocumentListQuery {
  direction?: Direction;
  docType?: DocType;
  status?: DocStatus;
  paymentState?: PaymentState;
  overdue?: boolean;
  contactId?: string;
  q?: string;
  from?: string;
  to?: string;
  limit: number;
  offset: number;
}

/**
 * `POST /api/documents/compute`. `contactId`, `docType`, `documentId` (the edited draft) let
 * advance lines validate as on save; `locale` picks the advance description language.
 */
export interface ComputeRequest {
  lines: DocumentLine[];
  vatMode: VatMode;
  currency: string;
  exchangeRate: string | null;
  roundTotal: boolean;
  contactId: string | null;
  docType: EditableDocType;
  locale: DocLocale;
  documentId?: string;
}
export interface ComputeResult {
  lines: ComputedLine[];
  totals: Totals;
}

export interface Payment {
  id: string;
  date: string;
  amount: string;
  note: string | null;
  /** The DDPP this payment created (proforma of a VAT payer). */
  advanceDocumentId: string | null;
  createdAt: string;
}
export interface PaymentInput {
  date: string;
  amount: string;
  note: string | null;
  /** Foreign-currency proforma only; null = ČNB rate for the payment date. */
  exchangeRate?: string | null;
}

/** `GET /api/exchange-rates/{currency}?date=` — `date` is the ČNB publication date used. */
export interface ExchangeRate {
  currency: string;
  date: string;
  rate: string;
}
