<script setup lang="ts">
import { computed, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useRoute, useRouter } from "vue-router";
import { useAction } from "@/composables/useAction";
import InvoiceFilters from "../components/InvoiceFilters.vue";
import StatusBadges from "../components/StatusBadges.vue";
import { formatDate, formatMoney, signed } from "../format";
import { listDocTypeOf, listLocation } from "../routes";
import { LIST_DOC_TYPES, PAGE_SIZE, useInvoiceListStore, type InvoiceFilters as Filters } from "../store";

const { t, locale } = useI18n();
const route = useRoute();
const router = useRouter();
const store = useInvoiceListStore();
const { error, run } = useAction();

const docType = computed(() => listDocTypeOf(route.query.type));
watch(docType, (type) => void run(() => store.setDocType(type)), { immediate: true });
/** Credit notes come from an invoice, DDPPs from proforma payments: only these two are created here. */
const newLink = computed(() =>
  docType.value === "invoice"
    ? { to: { name: "invoice-new" }, label: t("documents.list.new.invoice") }
    : docType.value === "proforma"
      ? { to: { name: "invoice-new", query: { docType: "proforma" } }, label: t("documents.list.new.proforma") }
      : null,
);

const onFilters = (f: Filters) => void run(() => store.applyFilters(f));

const rangeFrom = computed(() => (store.total === 0 ? 0 : store.offset + 1));
const rangeTo = computed(() => Math.min(store.offset + store.items.length, store.total));
const hasPrev = computed(() => store.offset > 0);
const hasNext = computed(() => store.offset + PAGE_SIZE < store.total);
const filtered = computed(() => Object.values(store.filters).some(Boolean));

function open(id: string) {
  void router.push({ name: "invoice-detail", params: { id } });
}
</script>

<template>
  <section class="space-y-4">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <h1 class="text-2xl font-semibold">{{ t(`documents.list.title.${docType}`) }}</h1>
      <RouterLink v-if="newLink" :to="newLink.to" class="btn btn-primary" data-test="new-document">{{ newLink.label }}</RouterLink>
    </div>

    <nav :aria-label="t('documents.list.tabs')" class="-mx-4 overflow-x-auto [scrollbar-width:none] border-b border-gray-200 px-4 sm:mx-0 sm:px-0 dark:border-gray-800">
      <ul class="flex gap-1 whitespace-nowrap">
        <li v-for="type in LIST_DOC_TYPES" :key="type">
          <RouterLink
            :to="listLocation(type)"
            class="inline-block border-b-2 border-transparent px-3 py-2 text-sm text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-gray-100"
            :class="{ '!border-blue-600 !text-gray-900 font-medium dark:!text-gray-100': type === docType }"
            :aria-current="type === docType ? 'page' : undefined"
            :data-test="`tab-${type}`"
          >
            {{ t(`documents.list.tab.${type}`) }}
          </RouterLink>
        </li>
      </ul>
    </nav>

    <InvoiceFilters :filters="store.filters" @change="onFilters" />

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
              <div class="mt-1 sm:hidden"><StatusBadges :status="d.status" :payment-state="d.paymentState" :overdue="d.overdue" :sent-at="d.sentAt" :sign="d.sign" /></div>
            </td>
            <td class="hidden whitespace-nowrap sm:table-cell">{{ formatDate(d.issueDate, locale) }}</td>
            <td class="hidden whitespace-nowrap md:table-cell" :class="{ 'text-red-600 dark:text-red-400': d.overdue }">
              {{ formatDate(d.dueDate, locale) }}
            </td>
            <td class="whitespace-nowrap text-right tabular-nums">{{ formatMoney(signed(d.payable, d.sign), d.currency, locale) }}</td>
            <td class="hidden sm:table-cell">
              <StatusBadges :status="d.status" :payment-state="d.paymentState" :overdue="d.overdue" :sent-at="d.sentAt" :sign="d.sign" />
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

    <div class="flex items-center justify-between gap-2 text-sm">
      <span class="text-gray-600 dark:text-gray-400">
        {{ t("common.range", { from: rangeFrom, to: rangeTo, total: store.total }) }}
      </span>
      <div class="flex gap-2">
        <button type="button" class="btn btn-sm" :disabled="!hasPrev" @click="run(() => store.goTo(store.offset - PAGE_SIZE))">
          {{ t("common.prev") }}
        </button>
        <button type="button" class="btn btn-sm" :disabled="!hasNext" @click="run(() => store.goTo(store.offset + PAGE_SIZE))">
          {{ t("common.next") }}
        </button>
      </div>
    </div>
  </section>
</template>
