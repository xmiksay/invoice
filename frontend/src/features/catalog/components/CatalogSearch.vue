<script setup lang="ts">
import { onBeforeUnmount, ref } from "vue";
import { useI18n } from "vue-i18n";

const SEARCH_DEBOUNCE_MS = 300;

/** Debounced search box for the catalog lists. */
const props = defineProps<{ id: string; initial: string; placeholder: string }>();
const emit = defineEmits<{ search: [q: string] }>();

const { t } = useI18n();
const q = ref(props.initial);
let timer: ReturnType<typeof setTimeout> | undefined;

function onInput() {
  clearTimeout(timer);
  timer = setTimeout(() => emit("search", q.value), SEARCH_DEBOUNCE_MS);
}

onBeforeUnmount(() => clearTimeout(timer));
</script>

<template>
  <div>
    <label :for="id" class="sr-only">{{ t("common.search") }}</label>
    <input :id="id" v-model="q" type="search" class="input sm:max-w-sm" :placeholder="placeholder" @input="onInput" />
  </div>
</template>
