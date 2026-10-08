<script setup lang="ts">
import { computed, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useRoute, useRouter } from "vue-router";
import { useAction } from "@/composables/useAction";
import { useCategoriesStore } from "@/features/settings/stores";
import DocTypeTabs from "../components/DocTypeTabs.vue";
import InvoiceFilters from "../components/InvoiceFilters.vue";
import ListDocMeta from "../components/ListDocMeta.vue";
import ListPager from "../components/ListPager.vue";
import StatusBadges from "../components/StatusBadges.vue";
import { formatDate, formatMoney, signed } from "../format";
import { listDocTypeOf } from "../routes";
import { useInvoiceListStore, type InvoiceFilters as Filters } from "../store";

const { t, locale } = useI18n();
const route = useRoute();
const router = useRouter();
const store = useInvoiceListStore();
const categoriesStore = useCategoriesStore();
const { error, run } = useAction();

const docType = computed(() => listDocTypeOf(route.query.type));
watch(docType, (type) => void run(() => store.setDocType(type)), { immediate: true });
// Category names only decorate rows and feed a filter: the list works without them.
if (!categoriesStore.loaded) categoriesStore.load().catch(() => {});
const categories = computed(() => categoriesStore.items.filter((c) => c.kind === "income"));

/** Credit notes come from an invoice, DDPPs from proforma payments: only these two are created natively. */
const newLink = computed(() =>
  docType.value === "invoice" || docType.value === "proforma"
    ? { to: { name: "invoice-new", query: docType.value === "proforma" ? { docType: "proforma" } : {} }, label: t(`documents.list.new.${docType.value}`) }
    : null,
);
/** Any type can be entered as an imported document (issued elsewhere, own number, no rendered PDF). */
const importLink = computed(() => ({ name: "invoice-new", query: { docType: docType.value, imported: "1" } }));

const onFilters = (f: Filters) => void run(() => store.applyFilters(f));
const filtered = computed(() => Object.values(store.filters).some(Boolean));

function open(id: string) {
  void router.push({ name: "invoice-detail", params: { id } });
}
</script>

<template>
  <section class="space-y-4">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <h1 class="text-2xl font-semibold">{{ t(`documents.list.title.${docType}`) }}</h1>
      <div class="flex flex-wrap gap-2">
        <RouterLink :to="importLink" class="btn" data-test="import-document">{{ t("documents.import.action") }}</RouterLink>
        <RouterLink v-if="newLink" :to="newLink.to" class="btn btn-primary" data-test="new-document">{{ newLink.label }}</RouterLink>
      </div>
    </div>

    <DocTypeTabs :current="docType" direction="issued" label-prefix="documents.list.tab" />

    <InvoiceFilters :filters="store.filters" :categories="categories" show-imported @change="onFilters" />

    <p v-if="error" role="alert" class="alert-error">{{ error }}</p>

    <div class="card overflow-x-auto !p-0">
      <table class="table">
        <thead>
          <tr>
            <th>{{ t("documents.fields.number") }}</th>
            <th>{{ t("documents.fields.customer") }}</th>
            <th class="hidden sm:table-cell">{{ t("documents.fields.issueDate") }}</th>
            <th class="hidden md:table-cell">{{ t("documents.fields.dueDate") }}</th>
            <th class="text-right">{{ t("documents.totals.payable") }}</th>
            <th class="hidden sm:table-cell">{{ t("documents.list.status") }}</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="d in store.items"
            :key="d.id"
            class="cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-800/60"
            data-test="invoice-row"
            @click="open(d.id)"
          >
            <td class="whitespace-nowrap">
              <RouterLink :to="{ name: 'invoice-detail', params: { id: d.id } }" class="font-mono font-medium hover:underline" @click.stop>
                {{ d.number ?? t("documents.draftNumber") }}
              </RouterLink>
            </td>
            <td>
              {{ d.customerName ?? "—" }}
              <ListDocMeta :doc="d" :categories="categories" />
              <div class="mt-1 sm:hidden"><StatusBadges :status="d.status" :payment-state="d.paymentState" :overdue="d.overdue" :sent-at="d.sentAt" :sign="d.sign" :imported="d.imported" /></div>
            </td>
            <td class="hidden whitespace-nowrap sm:table-cell">{{ formatDate(d.issueDate, locale) }}</td>
            <td class="hidden whitespace-nowrap md:table-cell" :class="{ 'text-red-600 dark:text-red-400': d.overdue }">
              {{ formatDate(d.dueDate, locale) }}
            </td>
            <td class="whitespace-nowrap text-right tabular-nums">{{ formatMoney(signed(d.payable, d.sign), d.currency, locale) }}</td>
            <td class="hidden sm:table-cell">
              <StatusBadges :status="d.status" :payment-state="d.paymentState" :overdue="d.overdue" :sent-at="d.sentAt" :sign="d.sign" :imported="d.imported" />
            </td>
          </tr>
          <tr v-if="!store.loading && store.items.length === 0">
            <td colspan="6" class="py-8 text-center text-gray-500">
              {{ filtered ? t("documents.list.noResults") : t(`documents.list.empty.${docType}`) }}
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <ListPager :offset="store.offset" :count="store.items.length" :total="store.total" @go="run(() => store.goTo($event))" />
  </section>
</template>
