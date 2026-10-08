<script setup lang="ts">
import { onBeforeUnmount, ref } from "vue";
import { useI18n } from "vue-i18n";
import type { Category } from "@/features/settings/types";
import type { InvoiceFilters } from "../store";
import { DOC_STATUSES, PAYMENT_STATES } from "../types";

const SEARCH_DEBOUNCE_MS = 300;

/** `categories`: those of the list's kind; `showImported`: the issued list's imported / native filter. */
const props = defineProps<{ filters: InvoiceFilters; categories: Category[]; showImported?: boolean; searchPlaceholder?: string }>();
const emit = defineEmits<{ change: [filters: InvoiceFilters] }>();

const { t } = useI18n();
const local = ref<InvoiceFilters>({ ...props.filters });
let timer: ReturnType<typeof setTimeout> | undefined;

function emitNow() {
  clearTimeout(timer);
  emit("change", { ...local.value });
}

function onSearchInput() {
  clearTimeout(timer);
  timer = setTimeout(emitNow, SEARCH_DEBOUNCE_MS);
}

onBeforeUnmount(() => clearTimeout(timer));
</script>

<template>
  <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-6">
    <div class="sm:col-span-2">
      <label for="invoice-search" class="sr-only">{{ t("common.search") }}</label>
      <input
        id="invoice-search"
        v-model="local.q"
        type="search"
        class="input"
        :placeholder="searchPlaceholder ?? t('documents.list.searchPlaceholder')"
        @input="onSearchInput"
      />
    </div>
    <div>
      <label for="filter-status" class="sr-only">{{ t("documents.list.status") }}</label>
      <select id="filter-status" v-model="local.status" class="input" @change="emitNow">
        <option value="">{{ t("documents.list.anyStatus") }}</option>
        <option v-for="s in DOC_STATUSES" :key="s" :value="s">{{ t(`documents.status.${s}`) }}</option>
      </select>
    </div>
    <div>
      <label for="filter-payment" class="sr-only">{{ t("documents.list.paymentState") }}</label>
      <select id="filter-payment" v-model="local.paymentState" class="input" @change="emitNow">
        <option value="">{{ t("documents.list.anyPaymentState") }}</option>
        <option v-for="s in PAYMENT_STATES" :key="s" :value="s">{{ t(`documents.paymentState.${s}`) }}</option>
      </select>
    </div>
    <div class="flex items-center gap-2">
      <label for="filter-from" class="sr-only">{{ t("documents.list.from") }}</label>
      <input id="filter-from" v-model="local.from" type="date" class="input" :title="t('documents.list.from')" @change="emitNow" />
      <span aria-hidden="true">–</span>
      <label for="filter-to" class="sr-only">{{ t("documents.list.to") }}</label>
      <input id="filter-to" v-model="local.to" type="date" class="input" :title="t('documents.list.to')" @change="emitNow" />
    </div>
    <label class="flex items-center gap-2 text-sm lg:justify-end">
      <input id="filter-overdue" v-model="local.overdue" type="checkbox" class="size-4" @change="emitNow" />
      {{ t("documents.list.overdueOnly") }}
    </label>
    <div class="sm:col-span-2">
      <label for="filter-category" class="sr-only">{{ t("metadata.category") }}</label>
      <select id="filter-category" v-model="local.categoryId" class="input" @change="emitNow">
        <option value="">{{ t("documents.list.anyCategory") }}</option>
        <option v-for="c in categories" :key="c.id" :value="c.id">{{ c.name }}</option>
      </select>
    </div>
    <div v-if="showImported" class="sm:col-span-2">
      <label for="filter-imported" class="sr-only">{{ t("documents.list.origin") }}</label>
      <select id="filter-imported" v-model="local.imported" class="input" @change="emitNow">
        <option value="">{{ t("documents.list.anyOrigin") }}</option>
        <option value="false">{{ t("documents.list.nativeOnly") }}</option>
        <option value="true">{{ t("documents.list.importedOnly") }}</option>
      </select>
    </div>
  </div>
</template>
