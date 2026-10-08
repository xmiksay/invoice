<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useAction } from "@/composables/useAction";
import CategoryForm from "../components/CategoryForm.vue";
import { useCategoriesStore } from "../stores";
import type { Category } from "../types";

const { t } = useI18n();
const store = useCategoriesStore();
const { error, run } = useAction();
/** null = form closed, "new" = create, otherwise the category being edited. */
const editing = ref<Category | "new" | null>(null);

const nextPosition = computed(() => Math.max(-1, ...store.items.map((c) => c.position)) + 1);

onMounted(() => void run(store.load));

function toggleActive(event: Event, c: Category) {
  // Reset to the stored value; the reloaded list re-renders it on success.
  (event.target as HTMLInputElement).checked = c.active;
  const { id, ...input } = c;
  void run(() => store.save({ ...input, active: !c.active }, id));
}

function onDelete(c: Category) {
  if (!window.confirm(t("settings.categories.confirmDelete", { name: c.name }))) return;
  void run(() => store.remove(c.id));
}
</script>

<template>
  <div class="space-y-4">
    <p class="text-sm text-gray-600 dark:text-gray-400">{{ t("settings.categories.help") }}</p>
    <p v-if="error" role="alert" class="alert-error" data-test="categories-error">{{ error }}</p>

    <CategoryForm
      v-if="editing"
      :key="editing === 'new' ? 'new' : editing.id"
      :category="editing === 'new' ? undefined : editing"
      :next-position="nextPosition"
      @done="editing = null"
    />
    <button v-else type="button" class="btn btn-primary" data-test="add-category" @click="editing = 'new'">{{ t("settings.categories.add") }}</button>

    <div class="card overflow-x-auto !p-0">
      <table class="table">
        <thead>
          <tr>
            <th>{{ t("settings.categories.name") }}</th>
            <th>{{ t("settings.categories.kind") }}</th>
            <th class="text-center">{{ t("settings.vatRates.active") }}</th>
            <th class="w-0"><span class="sr-only">{{ t("common.actions") }}</span></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="c in store.items" :key="c.id" :class="{ 'opacity-60': !c.active }" data-test="category-row">
            <td>{{ c.name }}</td>
            <td>{{ t(`settings.categories.kinds.${c.kind}`) }}</td>
            <td class="text-center">
              <input type="checkbox" class="size-4 rounded" :checked="c.active" :aria-label="t('settings.categories.toggleActive', { name: c.name })" @change="toggleActive($event, c)" />
            </td>
            <td class="whitespace-nowrap">
              <div class="flex gap-1">
                <button type="button" class="btn btn-sm" @click="editing = c">{{ t("common.edit") }}</button>
                <button type="button" class="btn btn-sm btn-danger" data-test="delete-category" @click="onDelete(c)">{{ t("common.delete") }}</button>
              </div>
            </td>
          </tr>
          <tr v-if="store.loaded && store.items.length === 0">
            <td colspan="4" class="py-8 text-center text-gray-500">{{ t("settings.categories.empty") }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
