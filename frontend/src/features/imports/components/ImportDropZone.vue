<script setup lang="ts">
import { ref } from "vue";

/** Drop area + file picker; emits every picked or dropped file (unfiltered — the page checks the types). */
const props = defineProps<{ accept: string; multiple?: boolean; disabled?: boolean; hint: string; pick: string; limits: string }>();
const emit = defineEmits<{ files: [files: File[]] }>();

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
    data-test="import-drop"
    @dragover.prevent="over = !disabled"
    @dragleave="over = false"
    @drop.prevent="onDrop"
  >
    <p class="text-sm text-gray-600 dark:text-gray-400">{{ hint }}</p>
    <input ref="input" type="file" :multiple="multiple" :accept="accept" class="hidden" data-test="import-input" @change="onPick" />
    <div class="flex flex-wrap justify-center gap-2">
      <button type="button" class="btn btn-primary" :disabled="disabled" data-test="import-pick" @click="input?.click()">
        {{ pick }}
      </button>
      <slot name="actions" />
    </div>
    <p class="text-xs text-gray-500 dark:text-gray-400">{{ limits }}</p>
  </div>
</template>
