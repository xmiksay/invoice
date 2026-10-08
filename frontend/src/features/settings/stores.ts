import { defineStore } from "pinia";
import { ref, type Ref } from "vue";
import { bankAccountsApi, categoriesApi, companyApi, customFieldsApi, numberSeriesApi, pdfApi, vatRatesApi, type CrudApi } from "./api";
import type {
  BankAccount,
  BankAccountInput,
  Category,
  CategoryInput,
  Company,
  CustomField,
  CustomFieldInput,
  DesignInfo,
  DocType,
  NumberSeries,
  VatRate,
  VatRateInput,
} from "./types";

export const useCompanyStore = defineStore("settings/company", () => {
  const company = ref<Company | null>(null);

  async function load(): Promise<void> {
    company.value = await companyApi.get();
  }

  async function save(input: Company): Promise<void> {
    company.value = await companyApi.update(input);
  }

  return { company, load, save };
});

/**
 * List settings whose default flag is owned by the server: saving one default
 * unsets the others (per currency for bank accounts, globally for VAT rates)
 * and the first bank account of a currency becomes default. Rather than mirror
 * those rules, every mutation reloads the list.
 */
function defineCrudStore<T extends { id: string }, I>(id: string, api: CrudApi<T, I>) {
  return defineStore(id, () => {
    const items = ref([]) as Ref<T[]>;
    const loaded = ref(false);

    async function load(): Promise<void> {
      items.value = await api.list();
      loaded.value = true;
    }

    async function save(input: I, itemId?: string): Promise<void> {
      if (itemId) await api.update(itemId, input);
      else await api.create(input);
      await load();
    }

    async function remove(itemId: string): Promise<void> {
      await api.remove(itemId);
      await load();
    }

    return { items, loaded, load, save, remove };
  });
}

export const useBankAccountsStore = defineCrudStore<BankAccount, BankAccountInput>(
  "settings/bankAccounts",
  bankAccountsApi,
);

export const useVatRatesStore = defineCrudStore<VatRate, VatRateInput>("settings/vatRates", vatRatesApi);
export const useCategoriesStore = defineCrudStore<Category, CategoryInput>("settings/categories", categoriesApi);
export const useCustomFieldsStore = defineCrudStore<CustomField, CustomFieldInput>("settings/customFields", customFieldsApi);

export const useNumberSeriesStore = defineStore("settings/numberSeries", () => {
  const series = ref<NumberSeries[]>([]);

  async function load(): Promise<void> {
    series.value = await numberSeriesApi.list();
  }

  function replace(saved: NumberSeries): void {
    series.value = series.value.map((s) => (s.docType === saved.docType ? saved : s));
  }

  async function savePattern(docType: DocType, pattern: string): Promise<void> {
    replace(await numberSeriesApi.updatePattern(docType, pattern));
  }

  async function setCounter(docType: DocType, year: number, lastNumber: number): Promise<void> {
    replace(await numberSeriesApi.setCounter(docType, year, lastNumber));
  }

  return { series, load, savePattern, setCounter };
});

/** Read-only: the design dir is re-read by the server on every render, so this reloads on each visit. */
export const useDesignStore = defineStore("settings/design", () => {
  const design = ref<DesignInfo | null>(null);

  async function load(): Promise<void> {
    design.value = await pdfApi.design();
  }

  return { design, load };
});
