<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import FormField from "@/components/form/FormField.vue";
import { useAction } from "@/composables/useAction";
import { documentsApi } from "../api";
import { relatedTargetType } from "../routes";
import type { Direction, DocumentSummary, RelatedDocument } from "../types";

const LIMIT = 50;
const SEARCH_DEBOUNCE_MS = 300;

/** Informational link of a received / imported document to its proforma (or a credit note to its invoice). */
const model = defineModel<string | null>({ required: true });
const props = defineProps<{
  direction: Direction;
  docType: string;
  /** Narrows the choice to the same counterparty once one is picked. */
  contactId: string | null;
  /** The currently linked document, so its label shows even when outside the loaded page. */
  current?: RelatedDocument | null;
  excludeId?: string;
  error?: string;
}>();

const { t } = useI18n();
const { error: loadError, run } = useAction();
const options = ref<DocumentSummary[]>([]);
/** More matches than shown: the search narrows them (number, supplier number, counterparty, VS). */
const more = ref(false);
const query = ref("");
const target = computed(() => relatedTargetType(props.docType));
let seq = 0;
let timer: ReturnType<typeof setTimeout> | undefined;

function load() {
  const type = target.value;
  options.value = [];
  more.value = false;
  if (!type) return;
  const mine = ++seq;
  void run(async () => {
    // Drafts cannot be linked (the server rejects them); the newest come first.
    const page = await documentsApi.list({
      direction: props.direction,
      docType: type,
      status: "issued",
      contactId: props.contactId ?? undefined,
      q: query.value,
      limit: LIMIT,
      offset: 0,
    });
    if (mine !== seq) return;
    options.value = page.items.filter((d) => d.id !== props.excludeId);
    more.value = page.total > page.items.length;
  });
}

watch(() => [target.value, props.contactId] as const, load, { immediate: true });

function onSearch() {
  clearTimeout(timer);
  timer = setTimeout(load, SEARCH_DEBOUNCE_MS);
}
onBeforeUnmount(() => clearTimeout(timer));

const missingCurrent = computed(() => !!model.value && !options.value.some((d) => d.id === model.value));
const label = (d: { number: string | null; supplierNumber?: string | null; customerName?: string | null }) =>
  [d.number ?? t("documents.draftNumber"), d.supplierNumber, d.customerName].filter(Boolean).join(" · ");
</script>

<template>
  <FormField v-if="target" :label="t(`documents.related.pick.${target}`)" for="related-document" :error="error" :hint="t('documents.related.pickHint')">
    <input
      v-model="query"
      type="search"
      class="input mb-1"
      :placeholder="t('documents.related.search')"
      :aria-label="t('documents.related.search')"
      autocomplete="off"
      data-test="related-search"
      @input="onSearch"
    />
    <select id="related-document" :value="model ?? ''" class="input" :class="{ 'input-error': error }" data-test="related-picker" @change="model = ($event.target as HTMLSelectElement).value || null">
      <option value="">{{ t("documents.related.none") }}</option>
      <option v-if="missingCurrent" :value="model ?? ''">{{ current ? label(current) : model }}</option>
      <option v-for="d in options" :key="d.id" :value="d.id">{{ label(d) }}</option>
    </select>
    <p v-if="more" class="text-xs text-gray-500" data-test="related-more">{{ t("documents.related.more", { n: options.length }) }}</p>
    <p v-if="loadError" role="alert" class="alert-error">{{ loadError }}</p>
  </FormField>
</template>
