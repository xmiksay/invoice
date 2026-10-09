import type { PreviewEntry } from "./types";

export const entry = (overrides: Partial<PreviewEntry> = {}): PreviewEntry => ({
  key: "export.zip/f1.isdoc",
  status: "ok",
  error: null,
  warnings: [],
  direction: "issued",
  docType: "invoice",
  number: "20260001",
  counterparty: { name: "Acme s.r.o.", ico: "12345678" },
  contactMatch: "existing",
  issueDate: "2026-09-01",
  taxPointDate: "2026-09-01",
  dueDate: "2026-09-15",
  currency: "CZK",
  total: "1210.00",
  hasPdf: true,
  relatedNumber: null,
  relatedFound: false,
  ...overrides,
});

export const isdocFile = (name = "f.isdoc", size = 100) => {
  const file = new File(["<Invoice/>"], name);
  Object.defineProperty(file, "size", { value: size });
  return file;
};
