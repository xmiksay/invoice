<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useAction } from "@/composables/useAction";
import CustomFieldForm from "../components/CustomFieldForm.vue";
import { useCustomFieldsStore } from "../stores";
import type { CustomField } from "../types";

const { t } = useI18n();
const store = useCustomFieldsStore();
const { error, run } = useAction();
/** null = form closed, "new" = create, otherwise the field being edited. */
const editing = ref<CustomField | "new" | null>(null);

const nextPosition = computed(() => Math.max(-1, ...store.items.map((f) => f.position)) + 1);
const sorted = computed(() => [...store.items].sort((a, b) => a.position - b.position || a.label.localeCompare(b.label)));

onMounted(() => void run(store.load));

function onDelete(f: CustomField) {
  if (!window.confirm(t("settings.customFields.confirmDelete", { label: f.label }))) return;
  void run(() => store.remove(f.id));
}
</script>

<template>
  <div class="space-y-4">
    <p class="text-sm text-gray-600 dark:text-gray-400">{{ t("settings.customFields.help") }}</p>
    <p v-if="error" role="alert" class="alert-error">{{ error }}</p>

    <CustomFieldForm
      v-if="editing"
      :key="editing === 'new' ? 'new' : editing.id"
      :field="editing === 'new' ? undefined : editing"
      :next-position="nextPosition"
      @done="editing = null"
    />
    <button v-else type="button" class="btn btn-primary" data-test="add-custom-field" @click="editing = 'new'">{{ t("settings.customFields.add") }}</button>

    <div class="card overflow-x-auto !p-0">
      <table class="table">
        <thead>
          <tr>
            <th>{{ t("settings.customFields.label") }}</th>
            <th>{{ t("settings.customFields.key") }}</th>
            <th>{{ t("settings.customFields.type") }}</th>
            <th class="hidden sm:table-cell">{{ t("settings.customFields.appliesTo") }}</th>
            <th class="w-0"><span class="sr-only">{{ t("common.actions") }}</span></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="f in sorted" :key="f.id" :class="{ 'opacity-60': !f.active }" data-test="custom-field-row">
            <td>
              {{ f.label }}
              <span v-if="f.required" class="badge ml-1">{{ t("settings.customFields.required") }}</span>
              <span v-if="!f.active" class="ml-1 text-xs text-gray-500">({{ t("settings.customFields.inactive") }})</span>
            </td>
            <td class="font-mono">{{ f.key }}</td>
            <td>
              {{ t(`settings.customFields.types.${f.type}`) }}
              <span v-if="f.type === 'select'" class="block text-xs text-gray-500">{{ f.options.join(", ") }}</span>
            </td>
            <td class="hidden sm:table-cell">{{ t(`settings.customFields.scopes.${f.appliesTo}`) }}</td>
            <td class="whitespace-nowrap">
              <div class="flex gap-1">
                <button type="button" class="btn btn-sm" @click="editing = f">{{ t("common.edit") }}</button>
                <button type="button" class="btn btn-sm btn-danger" data-test="delete-custom-field" @click="onDelete(f)">{{ t("common.delete") }}</button>
              </div>
            </td>
          </tr>
          <tr v-if="store.loaded && store.items.length === 0">
            <td colspan="5" class="py-8 text-center text-gray-500">{{ t("settings.customFields.empty") }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
