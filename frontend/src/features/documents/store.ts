import { defineStore } from "pinia";
import { ref } from "vue";
import { documentsApi } from "./api";
import type {
  DocStatus,
  Document,
  DocumentInput,
  DocumentSummary,
  Payment,
  PaymentInput,
  PaymentState,
} from "./types";

export const PAGE_SIZE = 50;

/** Doc types the list has a tab for, in tab order. */
export const LIST_DOC_TYPES = ["invoice", "proforma", "credit_note", "advance_tax_doc"] as const;
export type ListDocType = (typeof LIST_DOC_TYPES)[number];

export interface InvoiceFilters {
  status: DocStatus | "";
  paymentState: PaymentState | "";
  overdue: boolean;
  q: string;
  from: string;
  to: string;
}

export const emptyFilters = (): InvoiceFilters => ({ status: "", paymentState: "", overdue: false, q: "", from: "", to: "" });

/** Issued-document list; tab, filters + page live here so they survive a trip to the detail view. */
export const useInvoiceListStore = defineStore("documents/invoices", () => {
  const items = ref<DocumentSummary[]>([]);
  const total = ref(0);
  const docType = ref<ListDocType>("invoice");
  const filters = ref<InvoiceFilters>(emptyFilters());
  const offset = ref(0);
  const loading = ref(false);
  // Drops responses that arrive after a newer query was started.
  let seq = 0;

  async function load(): Promise<void> {
    const mine = ++seq;
    loading.value = true;
    const f = filters.value;
    try {
      const page = await documentsApi.list({
        direction: "issued",
        docType: docType.value,
        status: f.status || undefined,
        paymentState: f.paymentState || undefined,
        overdue: f.overdue,
        q: f.q,
        from: f.from || undefined,
        to: f.to || undefined,
        limit: PAGE_SIZE,
        offset: offset.value,
      });
      if (mine !== seq) return;
      items.value = page.items;
      total.value = page.total;
    } finally {
      if (mine === seq) loading.value = false;
    }
  }

  async function applyFilters(next: InvoiceFilters): Promise<void> {
    filters.value = { ...next };
    offset.value = 0;
    await load();
  }

  async function goTo(newOffset: number): Promise<void> {
    offset.value = Math.max(0, newOffset);
    await load();
  }

  /** Switching the tab keeps the filters but starts at the first page. */
  async function setDocType(next: ListDocType): Promise<void> {
    if (next !== docType.value) {
      docType.value = next;
      offset.value = 0;
      items.value = [];
    }
    await load();
  }

  return { items, total, docType, filters, offset, loading, load, applyFilters, goTo, setDocType };
});

/** The document open in the detail view, with its payments. Every action stores the server's answer. */
export const useDocumentStore = defineStore("documents/current", () => {
  const doc = ref<Document | null>(null);
  const payments = ref<Payment[]>([]);

  const id = () => {
    if (!doc.value) throw new Error("no document loaded");
    return doc.value.id;
  };

  async function load(docId: string): Promise<void> {
    if (doc.value?.id !== docId) {
      doc.value = null;
      payments.value = [];
    }
    const loaded = await documentsApi.get(docId);
    // A DDPP has no payments of its own.
    payments.value = loaded.status === "draft" || loaded.docType === "advance_tax_doc" ? [] : await documentsApi.payments(docId);
    doc.value = loaded;
  }

  async function save(input: DocumentInput, docId?: string): Promise<Document> {
    doc.value = docId ? await documentsApi.update(docId, input) : await documentsApi.create(input);
    return doc.value;
  }

  async function remove(): Promise<void> {
    await documentsApi.remove(id());
    doc.value = null;
  }

  async function issue(): Promise<void> {
    doc.value = await documentsApi.issue(id());
  }

  async function cancel(reason: string | null): Promise<void> {
    doc.value = await documentsApi.cancel(id(), reason);
  }

  async function markSent(): Promise<void> {
    doc.value = await documentsApi.markSent(id());
  }

  async function setInternalNote(note: string | null): Promise<void> {
    doc.value = await documentsApi.setInternalNote(id(), note);
  }

  /** Final-invoice draft settling the loaded proforma. */
  async function settle(): Promise<Document> {
    return documentsApi.settle(id());
  }

  /** Credit-note draft for the loaded invoice. */
  async function creditNote(reason: string | null): Promise<Document> {
    return documentsApi.creditNote(id(), reason);
  }

  // Payments change `paid` / `paymentState` (and may issue or cancel a DDPP), so the document is reloaded too.
  async function addPayment(input: PaymentInput): Promise<Payment> {
    const payment = await documentsApi.addPayment(id(), input);
    await load(id());
    return payment;
  }

  async function removePayment(paymentId: string): Promise<void> {
    await documentsApi.removePayment(id(), paymentId);
    await load(id());
  }

  return {
    doc,
    payments,
    load,
    save,
    remove,
    issue,
    cancel,
    markSent,
    setInternalNote,
    settle,
    creditNote,
    addPayment,
    removePayment,
  };
});
