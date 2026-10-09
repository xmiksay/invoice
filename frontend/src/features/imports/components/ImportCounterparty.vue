<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { MATCH_CLASS } from "../badges";
import type { PreviewEntry } from "../types";

/** Preview cell: counterparty name + IČO and whether an existing contact matched or one will be created. */
defineProps<{ entry: Pick<PreviewEntry, "counterparty" | "contactMatch"> }>();

const { t } = useI18n();
</script>

<template>
  <template v-if="entry.counterparty">
    {{ entry.counterparty.name }}
    <div v-if="entry.counterparty.ico" class="text-xs text-gray-500">{{ t("imports.preview.ico", { ico: entry.counterparty.ico }) }}</div>
    <span v-if="entry.contactMatch" class="badge mt-1" :class="MATCH_CLASS[entry.contactMatch]" data-test="import-contact">
      {{ t(`imports.contactMatch.${entry.contactMatch}`) }}
    </span>
  </template>
  <template v-else>—</template>
</template>
