<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import AccountantExportDialog from "./AccountantExportDialog.vue";

/** "Export pro účetní" on both lists: opens the period dialog, which does the download. */
const { t } = useI18n();
const open = ref(false);
</script>

<template>
  <button type="button" class="btn" data-test="accountant-export" @click="open = true">{{ t("csvExport.accountant.action") }}</button>
  <AccountantExportDialog v-if="open" @close="open = false" />
</template>
