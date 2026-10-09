type Translate = (key: string, params?: Record<string, unknown>) => string;

export const CONTACT_CONDITION = "{% if contact %}…{% endif %}";

/**
 * Prefixes of the server's `template_invalid` detail naming the validation sample that failed
 * (`without contact: line 12: …`) → `email.templates.samples.<key>`; `hint` explains the usual fix.
 * Add a row when the backend adds a sample.
 */
const SAMPLES: { prefix: string; key: string; hint?: string }[] = [
  { prefix: "without contact: ", key: "withoutContact", hint: "contactHint" },
  { prefix: "paid: ", key: "paid" },
  { prefix: "cancelled: ", key: "cancelled" },
  { prefix: "credit note: ", key: "creditNote" },
  { prefix: "no bank account: ", key: "noBankAccount" },
];
const LINE = /^line (\d+): /;

export interface TemplateDetail {
  text: string;
  /** How to fix the failure of that sample, when there is a known one. */
  hint: string | null;
}

/** Localizes the sample and `line N: ` prefixes; the MiniJinja message itself stays as sent. */
export function localizeTemplateDetail(detail: string, t: Translate): TemplateDetail {
  const sample = SAMPLES.find((s) => detail.startsWith(s.prefix));
  let rest = sample ? detail.slice(sample.prefix.length) : detail;
  const line = LINE.exec(rest);
  if (line) rest = t("email.templates.line", { line: line[1], message: rest.slice(line[0].length) });
  return {
    text: sample ? t(`email.templates.samples.${sample.key}`, { detail: rest }) : rest,
    hint: sample?.hint ? t(`email.templates.${sample.hint}`, { condition: CONTACT_CONDITION }) : null,
  };
}
