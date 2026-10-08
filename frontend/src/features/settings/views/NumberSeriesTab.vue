<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useI18n } from "vue-i18n";
import { useAction } from "@/composables/useAction";
import NumberSeriesCard from "../components/NumberSeriesCard.vue";
import { useNumberSeriesStore } from "../stores";
import { DOC_TYPES } from "../types";

const { t } = useI18n();
const store = useNumberSeriesStore();
const { error, run } = useAction();
const currentYear = new Date().getFullYear();

const ordered = computed(() =>
  [...store.series].sort((a, b) => DOC_TYPES.indexOf(a.docType) - DOC_TYPES.indexOf(b.docType)),
);

const tokens = [
  { token: "{YYYY}", key: "yyyy" },
  { token: "{YY}", key: "yy" },
  { token: "{NNNN}", key: "n" },
] as const;

onMounted(() => void run(store.load));
</script>

<template>
  <div class="space-y-4">
    <aside class="card text-sm">
      <h2 class="mb-2 font-semibold">{{ t("settings.numberSeries.helpTitle") }}</h2>
      <ul class="space-y-1">
        <li v-for="tok in tokens" :key="tok.key">
          <code class="rounded bg-gray-100 px-1 font-mono dark:bg-gray-800">{{ tok.token }}</code>
          — {{ t(`settings.numberSeries.tokens.${tok.key}`) }}
        </li>
      </ul>
      <p class="mt-2 text-gray-600 dark:text-gray-400">{{ t("settings.numberSeries.helpRules") }}</p>
    </aside>

    <p v-if="error" role="alert" class="alert-error">{{ error }}</p>

    <NumberSeriesCard v-for="s in ordered" :key="s.docType" :series="s" :current-year="currentYear" />
  </div>
</template>
