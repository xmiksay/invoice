import { request } from "@/api/client";
import type {
  ComputeRequest,
  ComputeResult,
  Document,
  DocumentInput,
  DocumentListQuery,
  DocumentPage,
  ExchangeRate,
  Payment,
  PaymentInput,
} from "./types";

const BASE = "/api/documents";
const item = (id: string) => `${BASE}/${encodeURIComponent(id)}`;

function listParams(query: DocumentListQuery): string {
  const params = new URLSearchParams();
  const { limit, offset, overdue, q, ...rest } = query;
  for (const [key, value] of Object.entries(rest)) {
    if (value) params.set(key, value);
  }
  if (overdue) params.set("overdue", "true");
  if (q?.trim()) params.set("q", q.trim());
  params.set("limit", String(limit));
  params.set("offset", String(offset));
  return params.toString();
}

export const documentsApi = {
  list: (query: DocumentListQuery) => request<DocumentPage>(`${BASE}?${listParams(query)}`),
  get: (id: string) => request<Document>(item(id)),
  create: (input: DocumentInput) => request<Document>(BASE, { method: "POST", body: input }),
  update: (id: string, input: DocumentInput) => request<Document>(item(id), { method: "PUT", body: input }),
  remove: (id: string) => request<void>(item(id), { method: "DELETE" }),
  compute: (body: ComputeRequest, signal?: AbortSignal) =>
    request<ComputeResult>(`${BASE}/compute`, { method: "POST", body, signal }),
  issue: (id: string) => request<Document>(`${item(id)}/issue`, { method: "POST" }),
  cancel: (id: string, reason: string | null) =>
    request<Document>(`${item(id)}/cancel`, { method: "POST", body: reason ? { reason } : {} }),
  markSent: (id: string) => request<Document>(`${item(id)}/mark-sent`, { method: "POST", body: {} }),
  setInternalNote: (id: string, internalNote: string | null) =>
    request<Document>(`${item(id)}/internal-note`, { method: "PUT", body: { internalNote } }),
  /** Draft final invoice from an issued proforma. */
  settle: (id: string) => request<Document>(`${item(id)}/settle`, { method: "POST" }),
  /** Credit-note draft for an issued invoice. */
  creditNote: (id: string, correctionReason: string | null) =>
    request<Document>(`${item(id)}/credit-note`, { method: "POST", body: correctionReason ? { correctionReason } : {} }),
  payments: (id: string) => request<Payment[]>(`${item(id)}/payments`),
  addPayment: (id: string, input: PaymentInput) =>
    request<Payment>(`${item(id)}/payments`, { method: "POST", body: input }),
  removePayment: (id: string, paymentId: string) =>
    request<void>(`${item(id)}/payments/${encodeURIComponent(paymentId)}`, { method: "DELETE" }),
};

export const exchangeRatesApi = {
  get: (currency: string, date: string) =>
    request<ExchangeRate>(
      `/api/exchange-rates/${encodeURIComponent(currency)}?${new URLSearchParams({ date }).toString()}`,
    ),
};
