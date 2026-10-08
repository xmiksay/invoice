import type { FieldErrors } from "@/api/types";
import { collectErrors, textRule } from "@/lib/formErrors";
import type { VatRate, VatRateInput } from "./types";

/** Accepts a decimal comma; the wire format is a dot-decimal string. */
export function normalizeRate(input: string): string {
  return input.trim().replace(",", ".");
}

export function validateVatRate(d: { rate: string; label: string; isDefault?: boolean; active?: boolean }): FieldErrors {
  const rate = normalizeRate(d.rate);
  const rateOk = /^\d{1,3}(\.\d{1,2})?$/.test(rate) && Number(rate) <= 100;
  return collectErrors({
    rate: rate === "" ? "required" : !rateOk && "invalid",
    label: textRule(d.label, { required: true, max: 100 }),
    // Mirrors the server: the default rate must be active.
    isDefault: d.isDefault === true && d.active === false && "invalid",
  });
}

export function toVatRateInput({ id: _id, ...rest }: VatRate): VatRateInput {
  return rest;
}
