<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { useAction } from "@/composables/useAction";
import InvoiceFilters from "../components/InvoiceFilters.vue";
import StatusBadges from "../components/StatusBadges.vue";
import { formatDate, formatMoney } from "../format";
import { PAGE_SIZE, useInvoiceListStore, type InvoiceFilters as Filters } from "../store";

const { t, locale } = useI18n();
const router = useRouter();
const store = useInvoiceListStore();
const { error, run } = useAction();

onMounted(() => void run(store.load));

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
      <h1 class="text-2xl font-semibold">{{ t("documents.list.title") }}</h1>
      <RouterLink :to="{ name: 'invoice-new' }" class="btn btn-primary">{{ t("documents.list.new") }}</RouterLink>
    </div>

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
              <div class="mt-1 sm:hidden"><StatusBadges :status="d.status" :payment-state="d.paymentState" :overdue="d.overdue" :sent-at="d.sentAt" /></div>
            </td>
            <td class="hidden whitespace-nowrap sm:table-cell">{{ formatDate(d.issueDate, locale) }}</td>
            <td class="hidden whitespace-nowrap md:table-cell" :class="{ 'text-red-600 dark:text-red-400': d.overdue }">
              {{ formatDate(d.dueDate, locale) }}
            </td>
            <td class="whitespace-nowrap text-right tabular-nums">{{ formatMoney(d.payable, d.currency, locale) }}</td>
            <td class="hidden sm:table-cell">
              <StatusBadges :status="d.status" :payment-state="d.paymentState" :overdue="d.overdue" :sent-at="d.sentAt" />
            </td>
          </tr>
          <tr v-if="!store.loading && store.items.length === 0">
            <td colspan="6" class="py-8 text-center text-gray-500">
              {{ filtered ? t("documents.list.noResults") : t("documents.list.empty") }}
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
