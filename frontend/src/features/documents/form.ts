import type { DocLocale, FieldErrors } from "@/api/types";
import { collectErrors, nullIfEmpty, textRule } from "@/lib/formErrors";
import type { Contact } from "@/features/contacts/types";
import type { BankAccount, Company, VatRate } from "@/features/settings/types";
import { toLineDraft, toWireLine, type LineDraft } from "./lines";
import type { ComputeRequest, Document, DocumentInput, PaymentMethod, VatMode } from "./types";

/** Editor form state; nullable wire strings are "" here. */
export interface DocumentDraft {
  contactId: string | null;
  issueDate: string;
  taxPointDate: string;
  dueDate: string;
  currency: string;
  exchangeRate: string;
  locale: DocLocale;
  vatMode: VatMode;
  bankAccountId: string;
  paymentMethod: PaymentMethod;
  variableSymbol: string;
  constantSymbol: string;
  orderRef: string;
  headerNote: string;
  footerNote: string;
  internalNote: string;
  roundTotal: boolean;
  lines: LineDraft[];
}

/** Settings the defaults depend on. */
export interface DraftContext {
  company: Company;
  vatRates: VatRate[];
  bankAccounts: BankAccount[];
  today: string;
}

/** Local calendar date as `YYYY-MM-DD`. */
export function todayIso(now = new Date()): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

export function addDays(date: string, days: number): string {
  const d = new Date(`${date}T00:00:00Z`);
  if (Number.isNaN(d.getTime())) return date;
  d.setUTCDate(d.getUTCDate() + days);
  return d.toISOString().slice(0, 10);
}

export function defaultBankAccountId(accounts: BankAccount[], currency: string): string {
  const matching = accounts.filter((a) => a.currency === currency);
  return (matching.find((a) => a.isDefault) ?? matching[0])?.id ?? "";
}

/** Rate for a new item line: the default active rate, "0" for non-payers. */
export function defaultVatRate(rates: VatRate[], vatMode: VatMode): string {
  if (vatMode === "non_payer") return "0";
  const active = rates.filter((r) => r.active);
  return (active.find((r) => r.isDefault) ?? active[0])?.rate ?? "0";
}

/** Mirrors the server's create defaults so the form shows them before the first save. */
export function newDocumentDraft(ctx: DraftContext): DocumentDraft {
  const { company, bankAccounts, today } = ctx;
  return {
    contactId: null,
    issueDate: today,
    taxPointDate: today,
    dueDate: addDays(today, company.defaultDueDays),
    currency: "CZK",
    exchangeRate: "",
    locale: company.defaultLocale,
    vatMode: company.vatPayer ? "standard" : "non_payer",
    bankAccountId: defaultBankAccountId(bankAccounts, "CZK"),
    paymentMethod: "bank_transfer",
    variableSymbol: "",
    constantSymbol: "",
    orderRef: "",
    headerNote: "",
    footerNote: "",
    internalNote: "",
    roundTotal: false,
    lines: [],
  };
}

/** Picking a customer applies its due days, locale and currency (and that currency's bank account). */
export function applyContact(draft: DocumentDraft, contact: Contact, ctx: DraftContext): DocumentDraft {
  const currency = contact.defaultCurrency ?? draft.currency;
  const next: DocumentDraft = {
    ...draft,
    contactId: contact.id,
    dueDate: addDays(draft.issueDate, contact.defaultDueDays ?? ctx.company.defaultDueDays),
    locale: contact.defaultLocale ?? ctx.company.defaultLocale,
  };
  return currency === draft.currency ? next : changeCurrency(next, currency, ctx.bankAccounts);
}

/**
 * Keeps currency-dependent fields consistent: bank account, CZK-only rounding,
 * and the manual rate — a rate for one currency is meaningless for another.
 */
export function changeCurrency(draft: DocumentDraft, currency: string, accounts: BankAccount[]): DocumentDraft {
  const code = currency.trim().toUpperCase();
  const account = accounts.find((a) => a.id === draft.bankAccountId);
  return {
    ...draft,
    currency: code,
    bankAccountId: account?.currency === code ? draft.bankAccountId : defaultBankAccountId(accounts, code),
    exchangeRate: code === draft.currency ? draft.exchangeRate : "",
    roundTotal: code === "CZK" ? draft.roundTotal : false,
  };
}

export function toDraft(doc: Document): DocumentDraft {
  return {
    contactId: doc.contactId,
    issueDate: doc.issueDate,
    taxPointDate: doc.taxPointDate ?? "",
    dueDate: doc.dueDate,
    currency: doc.currency,
    exchangeRate: doc.exchangeRate ?? "",
    locale: doc.locale,
    vatMode: doc.vatMode,
    bankAccountId: doc.bankAccountId ?? "",
    paymentMethod: doc.paymentMethod,
    variableSymbol: doc.variableSymbol ?? "",
    constantSymbol: doc.constantSymbol ?? "",
    orderRef: doc.orderRef ?? "",
    headerNote: doc.headerNote ?? "",
    footerNote: doc.footerNote ?? "",
    internalNote: doc.internalNote ?? "",
    roundTotal: doc.roundTotal,
    lines: doc.lines.map(toLineDraft),
  };
}

const DATE = /^\d{4}-\d{2}-\d{2}$/;
const DECIMAL = /^-?\d+([.,]\d+)?$/;

export function validateDocument(d: DocumentDraft): FieldErrors {
  const vs = d.variableSymbol.trim();
  const ks = d.constantSymbol.trim();
  const rate = d.exchangeRate.trim();
  const errors = collectErrors({
    issueDate: !DATE.test(d.issueDate) && "required",
    dueDate: !DATE.test(d.dueDate) ? "required" : d.dueDate < d.issueDate && "invalid",
    taxPointDate: d.taxPointDate !== "" && !DATE.test(d.taxPointDate) && "invalid",
    currency: !/^[A-Za-z]{3}$/.test(d.currency.trim()) && "invalid",
    exchangeRate: rate !== "" && !DECIMAL.test(rate) && "invalid",
    variableSymbol: vs !== "" && !/^\d{1,10}$/.test(vs) && "invalid",
    constantSymbol: ks !== "" && !/^\d{1,4}$/.test(ks) && "invalid",
    orderRef: textRule(d.orderRef, { max: 100 }),
    headerNote: textRule(d.headerNote, { max: 2000 }),
    footerNote: textRule(d.footerNote, { max: 2000 }),
    internalNote: textRule(d.internalNote, { max: 2000 }),
  });
  d.lines.forEach((line, i) => {
    const description = textRule(line.description, { required: line.kind !== "subtotal", max: 500 });
    if (description) errors[`lines.${i}.description`] = description;
  });
  return errors;
}

export function toInput(d: DocumentDraft): DocumentInput {
  return {
    docType: "invoice",
    direction: "issued",
    contactId: d.contactId,
    issueDate: d.issueDate,
    taxPointDate: nullIfEmpty(d.taxPointDate),
    dueDate: d.dueDate,
    currency: d.currency.trim().toUpperCase(),
    exchangeRate: nullIfEmpty(d.exchangeRate)?.replace(",", ".") ?? null,
    locale: d.locale,
    vatMode: d.vatMode,
    bankAccountId: nullIfEmpty(d.bankAccountId),
    paymentMethod: d.paymentMethod,
    variableSymbol: nullIfEmpty(d.variableSymbol),
    constantSymbol: nullIfEmpty(d.constantSymbol),
    orderRef: nullIfEmpty(d.orderRef),
    headerNote: nullIfEmpty(d.headerNote),
    footerNote: nullIfEmpty(d.footerNote),
    internalNote: nullIfEmpty(d.internalNote),
    roundTotal: d.roundTotal,
    lines: d.lines.map(toWireLine),
  };
}

/** Body for the live totals (no DB writes on the server). */
/**
 * Body for the live totals. Without a manual rate the indicative ČNB rate is
 * used for the CZK preview only — it is never saved, so issue fixes the real one.
 */
export function toComputeRequest(d: DocumentDraft, indicativeRate: string | null = null): ComputeRequest {
  const { lines, vatMode, currency, exchangeRate, roundTotal } = toInput(d);
  return { lines, vatMode, currency, exchangeRate: exchangeRate ?? indicativeRate, roundTotal };
}
