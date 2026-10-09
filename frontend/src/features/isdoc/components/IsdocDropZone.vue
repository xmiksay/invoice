<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { ACCEPT_ATTR } from "../upload";

/** Drop area + file picker for `.isdoc` / `.isdocx` / `.zip`; emits every picked or dropped file (unfiltered). */
const props = defineProps<{ disabled?: boolean }>();
const emit = defineEmits<{ files: [files: File[]] }>();

const { t } = useI18n();
const input = ref<HTMLInputElement | null>(null);
const over = ref(false);

function onPick(event: Event) {
  const el = event.target as HTMLInputElement;
  const files = Array.from(el.files ?? []);
  el.value = "";
  if (files.length) emit("files", files);
}

function onDrop(event: DragEvent) {
  over.value = false;
  if (props.disabled) return;
  const files = Array.from(event.dataTransfer?.files ?? []);
  if (files.length) emit("files", files);
}
</script>

<template>
  <div
    class="flex flex-col items-center justify-center gap-3 rounded-lg border-2 border-dashed p-8 text-center transition-colors"
    :class="over ? 'border-blue-500 bg-blue-50 dark:bg-blue-950/40' : 'border-gray-300 dark:border-gray-700'"
    data-test="isdoc-drop"
    @dragover.prevent="over = !disabled"
    @dragleave="over = false"
    @drop.prevent="onDrop"
  >
    <p class="text-sm text-gray-600 dark:text-gray-400">{{ t("isdoc.import.dropHint") }}</p>
    <input ref="input" type="file" multiple :accept="ACCEPT_ATTR" class="hidden" data-test="isdoc-input" @change="onPick" />
    <button type="button" class="btn btn-primary" :disabled="disabled" data-test="isdoc-pick" @click="input?.click()">
      {{ t("isdoc.import.pick") }}
    </button>
    <p class="text-xs text-gray-500 dark:text-gray-400">{{ t("isdoc.import.limits") }}</p>
  </div>
</template>
