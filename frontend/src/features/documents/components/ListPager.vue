<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { PAGE_SIZE } from "../store";

/** "1–50 of 120" + previous / next; emits the new offset. */
const props = defineProps<{ offset: number; count: number; total: number }>();
const emit = defineEmits<{ go: [offset: number] }>();
const { t } = useI18n();

const rangeFrom = computed(() => (props.total === 0 ? 0 : props.offset + 1));
const rangeTo = computed(() => Math.min(props.offset + props.count, props.total));
</script>

<template>
  <div class="flex items-center justify-between gap-2 text-sm">
    <span class="text-gray-600 dark:text-gray-400">
      {{ t("common.range", { from: rangeFrom, to: rangeTo, total }) }}
    </span>
    <div class="flex gap-2">
      <button type="button" class="btn btn-sm" :disabled="offset <= 0" @click="emit('go', offset - PAGE_SIZE)">
        {{ t("common.prev") }}
      </button>
      <button type="button" class="btn btn-sm" :disabled="offset + PAGE_SIZE >= total" @click="emit('go', offset + PAGE_SIZE)">
        {{ t("common.next") }}
      </button>
    </div>
  </div>
</template>
