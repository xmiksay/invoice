<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { reasonKey } from "@/lib/formErrors";

defineProps<{
  label: string;
  /** id of the control inside the slot, for the label's `for`. */
  for: string;
  /** Reason code from client or server validation. */
  error?: string;
  hint?: string;
}>();

const { t } = useI18n();
</script>

<template>
  <div class="space-y-1">
    <label :for="$props.for" class="block text-sm font-medium">{{ label }}</label>
    <slot />
    <p v-if="error" :id="`${$props.for}-error`" class="text-xs text-red-600 dark:text-red-400" data-test="field-error">
      {{ t(reasonKey(error)) }}
    </p>
    <p v-else-if="hint" class="text-xs text-gray-500 dark:text-gray-400">{{ hint }}</p>
  </div>
</template>
