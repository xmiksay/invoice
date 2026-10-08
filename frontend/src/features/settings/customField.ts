import type { FieldErrors } from "@/api/types";
import { collectErrors, textRule } from "@/lib/formErrors";
import type { CustomField, CustomFieldInput, CustomFieldScope, CustomFieldType } from "./types";

/** Form state; select options are edited one per line. */
export interface CustomFieldDraft {
  key: string;
  label: string;
  type: CustomFieldType;
  options: string;
  appliesTo: CustomFieldScope;
  required: boolean;
  active: boolean;
  position: number | string;
}

export const KEY_PATTERN = /^[a-z][a-z0-9_]{0,39}$/;

export const toCustomFieldDraft = (f: CustomField | undefined, nextPosition: number): CustomFieldDraft => ({
  key: f?.key ?? "",
  label: f?.label ?? "",
  type: f?.type ?? "text",
  options: f?.options.join("\n") ?? "",
  appliesTo: f?.appliesTo ?? "both",
  required: f?.required ?? false,
  active: f?.active ?? true,
  position: f?.position ?? nextPosition,
});

export const parseOptions = (text: string): string[] =>
  text
    .split("\n")
    .map((o) => o.trim())
    .filter(Boolean);

/** Mirrors the server: key pattern, label ≤ 100, select options 1..50 unique and ≤ 100 each. */
export function validateCustomField(d: CustomFieldDraft): FieldErrors {
  const options = parseOptions(d.options);
  const optionsReason =
    d.type !== "select"
      ? null
      : options.length === 0
        ? "required"
        : options.length > 50 || options.some((o) => o.length > 100)
          ? "invalid"
          : new Set(options).size !== options.length && "duplicate";
  return collectErrors({
    key: d.key.trim() === "" ? "required" : !KEY_PATTERN.test(d.key.trim()) && "invalid",
    label: textRule(d.label, { required: true, max: 100 }),
    options: optionsReason,
  });
}

export const toCustomFieldInput = (d: CustomFieldDraft): CustomFieldInput => ({
  key: d.key.trim(),
  label: d.label.trim(),
  type: d.type,
  options: d.type === "select" ? parseOptions(d.options) : [],
  appliesTo: d.appliesTo,
  required: d.required,
  active: d.active,
  position: Number(d.position) || 0,
});
