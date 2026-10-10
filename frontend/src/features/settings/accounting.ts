import type { FieldErrors } from "@/api/types";
import type { Direction } from "@/features/documents/types";
import { nullIfEmpty, textRule } from "@/lib/formErrors";
import { ACCOUNTING_DIRECTIONS, ACCOUNTING_DOC_TYPES, type AccountingSettings, type PohodaCodes } from "./types";

/** The code columns of the Pohoda table, in display order. */
export const POHODA_CODE_FIELDS = ["accounting", "classificationVat", "classificationVatNonDeductible", "numberSeries"] as const;
export type PohodaCodeField = (typeof POHODA_CODE_FIELDS)[number];

/** Whether the column has an input in a row of that direction (VAT deduction exists only for received documents). */
export const codeApplies = (field: PohodaCodeField, direction: Direction): boolean =>
  field !== "classificationVatNonDeductible" || direction === "received";
/** Server limit per code (after trim): Pohoda `typ:idsType` maxLength. */
export const CODE_MAX_LENGTH = 19;

export type PohodaRowDraft = Pick<PohodaCodes, "direction" | "docType"> & Record<PohodaCodeField, string>;

export interface AccountingDraft {
  ico: string;
  /** Every direction × doc type in display order; the index is the `N` of the 422 keys `pohoda.codes.N.*`. */
  rows: PohodaRowDraft[];
  /** Absent until 3c (the server omits it); passed back only when loaded. */
  money?: AccountingSettings["money"];
}

/** Rows are rebuilt in display order rather than taken as sent, so a missing row still gets its inputs. */
export function toAccountingDraft(settings: AccountingSettings | null): AccountingDraft {
  const codes = settings?.pohoda.codes ?? [];
  const rows = ACCOUNTING_DIRECTIONS.flatMap((direction) =>
    ACCOUNTING_DOC_TYPES.map((docType) => {
      const row = codes.find((c) => c.direction === direction && c.docType === docType);
      return {
        direction,
        docType,
        accounting: row?.accounting ?? "",
        classificationVat: row?.classificationVat ?? "",
        numberSeries: row?.numberSeries ?? "",
        classificationVatNonDeductible: row?.classificationVatNonDeductible ?? "",
      };
    }),
  );
  return { ico: settings?.pohoda.ico ?? "", rows, money: settings?.money };
}

/** `""` → null; `money` goes back exactly as loaded (edited in 3c), and is left out when it was absent. */
export function toAccountingSettings(draft: AccountingDraft): AccountingSettings {
  return {
    ...(draft.money ? { money: draft.money } : {}),
    pohoda: {
      ico: nullIfEmpty(draft.ico),
      codes: draft.rows.map((r) => ({
        direction: r.direction,
        docType: r.docType,
        accounting: nullIfEmpty(r.accounting),
        classificationVat: nullIfEmpty(r.classificationVat),
        numberSeries: nullIfEmpty(r.numberSeries),
        classificationVatNonDeductible: codeApplies("classificationVatNonDeductible", r.direction)
          ? nullIfEmpty(r.classificationVatNonDeductible)
          : null,
      })),
    },
  };
}

/** Client mirror of the server's length check (≤ 19), keyed like its 422 (`pohoda.codes.N.<field>`). */
export function validateAccounting(draft: AccountingDraft): FieldErrors {
  const errors: FieldErrors = {};
  draft.rows.forEach((row, i) => {
    for (const field of POHODA_CODE_FIELDS) {
      const reason = textRule(row[field], { max: CODE_MAX_LENGTH });
      if (reason) errors[`pohoda.codes.${i}.${field}`] = reason;
    }
  });
  return errors;
}
