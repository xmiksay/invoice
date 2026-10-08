<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { reasonKey } from "@/lib/formErrors";
import { useAction } from "@/composables/useAction";
import { contactsApi } from "@/features/contacts/api";
import type { Contact } from "@/features/contacts/types";

const SEARCH_DEBOUNCE_MS = 300;
const RESULTS = 10;

const props = defineProps<{ contactId: string | null; error?: string }>();
const emit = defineEmits<{ pick: [contact: Contact]; clear: [] }>();

const { t } = useI18n();
const { error: loadError, run } = useAction();

const selected = ref<Contact | null>(null);
const query = ref("");
const results = ref<Contact[]>([]);
const searching = ref(false);
let timer: ReturnType<typeof setTimeout> | undefined;
let seq = 0;

// Resolve the name of an already linked contact (editing an existing draft).
watch(
  () => props.contactId,
  (id) => {
    if (!id) selected.value = null;
    else if (selected.value?.id !== id) void run(async () => void (selected.value = await contactsApi.get(id)));
  },
  { immediate: true },
);

async function search() {
  const mine = ++seq;
  const page = await contactsApi.list({ q: query.value, limit: RESULTS, offset: 0 });
  if (mine === seq) results.value = page.items;
}

function onInput() {
  clearTimeout(timer);
  timer = setTimeout(() => void run(search), SEARCH_DEBOUNCE_MS);
}

function startSearch() {
  searching.value = true;
  void run(search);
}

function pick(c: Contact) {
  selected.value = c;
  searching.value = false;
  query.value = "";
  emit("pick", c);
}

onBeforeUnmount(() => clearTimeout(timer));
</script>

<template>
  <div class="space-y-2">
    <div class="flex items-center justify-between gap-2">
      <span id="contact-picker-label" class="text-sm font-medium">{{ t("documents.fields.customer") }}</span>
      <RouterLink :to="{ name: 'contact-new' }" target="_blank" class="text-xs text-blue-600 hover:underline dark:text-blue-400">
        {{ t("documents.editor.newContact") }}
      </RouterLink>
    </div>

    <div v-if="selected && !searching" class="flex flex-wrap items-start justify-between gap-2 rounded-md border border-gray-200 p-3 dark:border-gray-700" data-test="selected-contact">
      <div class="text-sm">
        <div class="font-medium">{{ selected.name }}</div>
        <div class="text-gray-600 dark:text-gray-400">
          {{ [selected.ico && `${t("party.ico")} ${selected.ico}`, selected.street, [selected.zip, selected.city].filter(Boolean).join(" ")].filter(Boolean).join(", ") }}
        </div>
      </div>
      <div class="flex gap-2">
        <button type="button" class="btn btn-sm" @click="startSearch">{{ t("documents.editor.changeContact") }}</button>
        <button type="button" class="btn btn-sm" @click="emit('clear')">{{ t("documents.editor.removeContact") }}</button>
      </div>
    </div>

    <div v-else class="space-y-1">
      <input
        id="contact-search"
        v-model="query"
        type="search"
        class="input"
        :class="{ 'input-error': error }"
        aria-labelledby="contact-picker-label"
        :placeholder="t('contacts.searchPlaceholder')"
        autocomplete="off"
        @focus="startSearch"
        @input="onInput"
      />
      <ul v-if="searching && results.length" class="max-h-60 overflow-y-auto rounded-md border border-gray-200 dark:border-gray-700" data-test="contact-results">
        <li v-for="c in results" :key="c.id">
          <button type="button" class="w-full px-3 py-2 text-left text-sm hover:bg-gray-100 dark:hover:bg-gray-800" @click="pick(c)">
            <span class="font-medium">{{ c.name }}</span>
            <span class="ml-2 text-gray-500">{{ [c.ico, c.city].filter(Boolean).join(" · ") }}</span>
          </button>
        </li>
      </ul>
      <p v-else-if="searching && query" class="text-xs text-gray-500">{{ t("contacts.noResults") }}</p>
    </div>

    <p v-if="error" id="contact-search-error" class="text-xs text-red-600 dark:text-red-400" data-test="field-error">{{ t(reasonKey(error)) }}</p>
    <p v-if="loadError" role="alert" class="alert-error">{{ loadError }}</p>
  </div>
</template>
