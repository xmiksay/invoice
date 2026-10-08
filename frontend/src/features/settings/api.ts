import { request } from "@/api/client";
import type {
  BankAccount,
  BankAccountInput,
  Company,
  DocType,
  NumberSeries,
  VatRate,
  VatRateInput,
} from "./types";

const BASE = "/api/settings";

export interface CrudApi<T, I> {
  list(): Promise<T[]>;
  create(input: I): Promise<T>;
  update(id: string, input: I): Promise<T>;
  remove(id: string): Promise<void>;
}

function crudApi<T, I>(path: string): CrudApi<T, I> {
  const url = `${BASE}/${path}`;
  const item = (id: string) => `${url}/${encodeURIComponent(id)}`;
  return {
    list: () => request<T[]>(url),
    create: (input) => request<T>(url, { method: "POST", body: input }),
    update: (id, input) => request<T>(item(id), { method: "PUT", body: input }),
    remove: (id) => request<void>(item(id), { method: "DELETE" }),
  };
}

export const companyApi = {
  get: () => request<Company>(`${BASE}/company`),
  update: (company: Company) => request<Company>(`${BASE}/company`, { method: "PUT", body: company }),
};

export const bankAccountsApi = crudApi<BankAccount, BankAccountInput>("bank-accounts");
export const vatRatesApi = crudApi<VatRate, VatRateInput>("vat-rates");

export const numberSeriesApi = {
  list: () => request<NumberSeries[]>(`${BASE}/number-series`),
  updatePattern: (docType: DocType, pattern: string) =>
    request<NumberSeries>(`${BASE}/number-series/${docType}`, { method: "PUT", body: { pattern } }),
  setCounter: (docType: DocType, year: number, lastNumber: number) =>
    request<NumberSeries>(`${BASE}/number-series/${docType}/counters/${year}`, {
      method: "PUT",
      body: { lastNumber },
    }),
};
