import type { CsvPreviewEntry } from "./types";

export const csvEntry = (overrides: Partial<CsvPreviewEntry> = {}): CsvPreviewEntry => ({
  key: "row:2",
  row: 2,
  field: null,
  categoryMatch: null,
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
  hasPdf: false,
  relatedNumber: null,
  relatedFound: false,
  ...overrides,
});

export const csvFile = (name = "import.csv", size = 100) => {
  const file = new File(["direction;doc_type"], name);
  Object.defineProperty(file, "size", { value: size });
  return file;
};
