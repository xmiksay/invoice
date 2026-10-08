<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useAction } from "@/composables/useAction";
import { useNumberSeriesStore } from "../stores";
import type { NumberSeries } from "../types";

const props = defineProps<{ series: NumberSeries; currentYear: number }>();

const { t } = useI18n();
const store = useNumberSeriesStore();
const { error, run } = useAction();

const sorted = computed(() => [...props.series.counters].sort((a, b) => b.year - a.year));
/** Edited lastNumber per year, as typed. */
const drafts = ref<Record<number, string>>({});

function defaultNewYear(): string {
  const taken = props.series.counters.some((c) => c.year === props.currentYear);
  return String(taken ? props.currentYear + 1 : props.currentYear);
}

const newYear = ref(defaultNewYear());
const newLast = ref("0");

// Only seed drafts for years that appear; existing drafts (possibly unsaved
// edits in other rows) are left alone.
watch(
  () => props.series.counters,
  (counters) => {
    for (const c of counters) {
      if (!(c.year in drafts.value)) drafts.value[c.year] = String(c.lastNumber);
    }
  },
  { immediate: true },
);

const isCount = (v: string) => /^\d{1,9}$/.test(v.trim());

async function save(yearText: string, value: string, isNewRow = false) {
  if (!/^\d{4}$/.test(yearText.trim()) || !isCount(value)) {
    error.value = t("settings.numberSeries.invalidCounter");
    return;
  }
  const year = Number(yearText);
  const lastNumber = Number(value.trim());
  if (await run(() => store.setCounter(props.series.docType, year, lastNumber))) {
    // Reset just the saved row; other rows keep whatever is being typed.
    drafts.value[year] = String(lastNumber);
    if (isNewRow) {
      newYear.value = String(year + 1);
      newLast.value = "0";
    }
  }
}
</script>

<template>
  <div class="space-y-2">
    <h3 class="text-sm font-medium">{{ t("settings.numberSeries.counters") }}</h3>
    <table class="table max-w-md">
      <thead>
        <tr>
          <th>{{ t("settings.numberSeries.year") }}</th>
          <th>{{ t("settings.numberSeries.lastNumber") }}</th>
          <th class="w-0"><span class="sr-only">{{ t("common.actions") }}</span></th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="c in sorted" :key="c.year">
          <td class="font-mono">{{ c.year }}</td>
          <td>
            <input
              v-model="drafts[c.year]"
              inputmode="numeric"
              class="input py-1"
              :aria-label="t('settings.numberSeries.lastNumberFor', { year: c.year })"
            />
          </td>
          <td>
            <button
              type="button"
              class="btn btn-sm"
              :disabled="drafts[c.year] === String(c.lastNumber)"
              @click="save(String(c.year), drafts[c.year] ?? '')"
            >
              {{ t("common.save") }}
            </button>
          </td>
        </tr>
        <tr>
          <td>
            <input v-model="newYear" inputmode="numeric" maxlength="4" class="input py-1" :aria-label="t('settings.numberSeries.year')" />
          </td>
          <td>
            <input v-model="newLast" inputmode="numeric" class="input py-1" :aria-label="t('settings.numberSeries.lastNumber')" />
          </td>
          <td>
            <button type="button" class="btn btn-sm" @click="save(newYear, newLast, true)">{{ t("common.add") }}</button>
          </td>
        </tr>
      </tbody>
    </table>
    <p v-if="error" role="alert" class="alert-error">{{ error }}</p>
  </div>
</template>
