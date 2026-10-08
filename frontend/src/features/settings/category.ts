import type { FieldErrors } from "@/api/types";
import { collectErrors, textRule } from "@/lib/formErrors";
import type { Category, CategoryInput, CategoryKind } from "./types";

export interface CategoryDraft {
  name: string;
  kind: CategoryKind;
  active: boolean;
  position: number | string;
}

export const toCategoryDraft = (c: Category | undefined, nextPosition: number, kind: CategoryKind = "expense"): CategoryDraft => ({
  name: c?.name ?? "",
  kind: c?.kind ?? kind,
  active: c?.active ?? true,
  position: c?.position ?? nextPosition,
});

export const validateCategory = (d: CategoryDraft): FieldErrors => collectErrors({ name: textRule(d.name, { required: true, max: 100 }) });

export const toCategoryInput = (d: CategoryDraft): CategoryInput => ({
  name: d.name.trim(),
  kind: d.kind,
  active: d.active,
  position: Number(d.position) || 0,
});
