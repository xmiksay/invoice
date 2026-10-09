/** Template context variables (docs/api/email.md); the description key is `email.vars.<name with dots → _>`. */
export const TEMPLATE_VARIABLES = [
  "doc.type",
  "doc.typeLabel",
  "doc.number",
  "doc.issueDate",
  "doc.taxDate",
  "doc.dueDate",
  "doc.total",
  "doc.payable",
  "doc.currency",
  "doc.paid",
  "doc.toPay",
  "doc.cancelled",
  "doc.variableSymbol",
  "doc.bankAccount",
  "doc.iban",
  "doc.originalNumber",
  "company.name",
  "company.email",
  "company.phone",
  "company.web",
  "contact.name",
  "contact.email",
] as const;

export const variableKey = (name: string) => `email.vars.${name.replace(".", "_")}`;
