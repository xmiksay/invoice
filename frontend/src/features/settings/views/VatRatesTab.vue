<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useAction } from "@/composables/useAction";
import { fieldErrorsOf } from "@/lib/formErrors";
import VatRateForm from "../components/VatRateForm.vue";
import { useVatRatesStore } from "../stores";
import type { VatRate, VatRateInput } from "../types";
import { toVatRateInput, validateVatRate } from "../vatRate";

const { t } = useI18n();
const store = useVatRatesStore();
const { error, run, errorText } = useAction();
/** null = form closed, "new" = create, otherwise the rate being edited. */
const editing = ref<VatRate | "new" | null>(null);

const nextPosition = computed(() => Math.max(-1, ...store.items.map((r) => r.position)) + 1);

onMounted(() => void run(store.load));

// The control is reset to the stored value right away; the reloaded list
// re-renders it on success, so a failed save never leaves it out of sync.
function update(event: Event, r: VatRate, patch: Partial<VatRate>) {
  const input = event.target as HTMLInputElement;
  input.checked = patch.isDefault !== undefined ? r.isDefault : r.active;
  const body = { ...toVatRateInput(r), ...patch };
  if (validateVatRate(body).isDefault) {
    error.value = t("settings.vatRates.defaultMustBeActive");
    return;
  }
  void persist(body, r.id);
}

async function persist(body: VatRateInput, id: string) {
  error.value = null;
  try {
    await store.save(body, id);
  } catch (err) {
    error.value = fieldErrorsOf(err)?.isDefault ? t("settings.vatRates.defaultMustBeActive") : errorText(err);
  }
}

function onDelete(r: VatRate) {
  if (!window.confirm(t("settings.vatRates.confirmDelete", { rate: r.rate, label: r.label }))) return;
  void run(() => store.remove(r.id));
}
</script>

<template>
  <div class="space-y-4">
    <p v-if="error" role="alert" class="alert-error">{{ error }}</p>

    <VatRateForm
      v-if="editing"
      :key="editing === 'new' ? 'new' : editing.id"
      :vat-rate="editing === 'new' ? undefined : editing"
      :next-position="nextPosition"
      @done="editing = null"
    />
    <button v-else type="button" class="btn btn-primary" @click="editing = 'new'">
      {{ t("settings.vatRates.add") }}
    </button>

    <div class="card overflow-x-auto !p-0">
      <table class="table">
        <thead>
          <tr>
            <th>{{ t("settings.vatRates.rate") }}</th>
            <th>{{ t("settings.vatRates.label") }}</th>
            <th class="text-center">{{ t("settings.vatRates.isDefault") }}</th>
            <th class="text-center">{{ t("settings.vatRates.active") }}</th>
            <th class="w-0"><span class="sr-only">{{ t("common.actions") }}</span></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="r in store.items" :key="r.id" :class="{ 'opacity-60': !r.active }">
            <td class="font-mono">{{ r.rate }} %</td>
            <td>{{ r.label }}</td>
            <td class="text-center">
              <input
                type="radio"
                class="size-4"
                :checked="r.isDefault"
                :aria-label="t('settings.vatRates.setDefault', { rate: r.rate })"
                @change="update($event, r, { isDefault: true })"
              />
            </td>
            <td class="text-center">
              <input
                type="checkbox"
                class="size-4 rounded"
                :checked="r.active"
                :aria-label="t('settings.vatRates.toggleActive', { rate: r.rate })"
                @change="update($event, r, { active: !r.active })"
              />
            </td>
            <td class="whitespace-nowrap">
              <div class="flex gap-1">
                <button type="button" class="btn btn-sm" @click="editing = r">{{ t("common.edit") }}</button>
                <button type="button" class="btn btn-sm btn-danger" @click="onDelete(r)">{{ t("common.delete") }}</button>
              </div>
            </td>
          </tr>
          <tr v-if="store.loaded && store.items.length === 0">
            <td colspan="5" class="py-8 text-center text-gray-500">{{ t("settings.vatRates.empty") }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
