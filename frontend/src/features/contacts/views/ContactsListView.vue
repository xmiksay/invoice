<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { useAction } from "@/composables/useAction";
import { PAGE_SIZE, useContactsStore } from "../store";
import { useSessionStore } from "@/stores/session";

const SEARCH_DEBOUNCE_MS = 300;

const { t } = useI18n();
const session = useSessionStore();
const router = useRouter();
const store = useContactsStore();

const query = ref(store.q);
const { error, run } = useAction();
let timer: ReturnType<typeof setTimeout> | undefined;

function onSearchInput() {
  clearTimeout(timer);
  timer = setTimeout(() => void run(() => store.search(query.value)), SEARCH_DEBOUNCE_MS);
}

onMounted(() => void run(store.load));
onBeforeUnmount(() => clearTimeout(timer));

const rangeFrom = computed(() => (store.total === 0 ? 0 : store.offset + 1));
const rangeTo = computed(() => Math.min(store.offset + store.items.length, store.total));
const hasPrev = computed(() => store.offset > 0);
const hasNext = computed(() => store.offset + PAGE_SIZE < store.total);

function open(id: string) {
  void router.push({ name: "contact-edit", params: { id } });
}
</script>

<template>
  <section class="space-y-4">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <h1 class="text-2xl font-semibold">{{ t("contacts.title") }}</h1>
      <RouterLink v-if="session.can('write')" :to="{ name: 'contact-new' }" class="btn btn-primary" data-test="new-contact">{{ t("contacts.new") }}</RouterLink>
    </div>

    <div>
      <label for="contact-search" class="sr-only">{{ t("common.search") }}</label>
      <input
        id="contact-search"
        v-model="query"
        type="search"
        class="input"
        :placeholder="t('contacts.searchPlaceholder')"
        @input="onSearchInput"
      />
    </div>

    <p v-if="error" role="alert" class="alert-error">{{ error }}</p>

    <div class="card overflow-x-auto !p-0">
      <table class="table">
        <thead>
          <tr>
            <th>{{ t("party.name") }}</th>
            <th class="hidden sm:table-cell">{{ t("party.ico") }}</th>
            <th class="hidden sm:table-cell">{{ t("party.city") }}</th>
            <th class="hidden md:table-cell">{{ t("party.email") }}</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="c in store.items"
            :key="c.id"
            class="cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-800/60"
            @click="open(c.id)"
          >
            <td>
              <RouterLink :to="{ name: 'contact-edit', params: { id: c.id } }" class="font-medium hover:underline" @click.stop>
                {{ c.name }}
              </RouterLink>
              <div class="text-xs text-gray-500 sm:hidden">{{ [c.ico, c.city].filter(Boolean).join(" · ") }}</div>
            </td>
            <td class="hidden sm:table-cell">{{ c.ico }}</td>
            <td class="hidden sm:table-cell">{{ c.city }}</td>
            <td class="hidden md:table-cell">{{ c.email }}</td>
          </tr>
          <tr v-if="!store.loading && store.items.length === 0">
            <td colspan="4" class="py-8 text-center text-gray-500">
              {{ store.q ? t("contacts.noResults") : t("contacts.empty") }}
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
