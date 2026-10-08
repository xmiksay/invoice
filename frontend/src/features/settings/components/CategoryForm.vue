<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import FormField from "@/components/form/FormField.vue";
import { useFormSubmit } from "@/composables/useFormSubmit";
import { toCategoryDraft, toCategoryInput, validateCategory } from "../category";
import { useCategoriesStore } from "../stores";
import { CATEGORY_KINDS, type Category } from "../types";

const props = defineProps<{ category?: Category; nextPosition: number }>();
const emit = defineEmits<{ done: [] }>();

const { t } = useI18n();
const store = useCategoriesStore();
const { fieldErrors, error, submitting, submit } = useFormSubmit();
const draft = ref(toCategoryDraft(props.category, props.nextPosition));

async function onSubmit() {
  const ok = await submit(
    () => validateCategory(draft.value),
    () => store.save(toCategoryInput(draft.value), props.category?.id),
  );
  if (ok) emit("done");
}
</script>

<template>
  <form class="card space-y-4" novalidate data-test="category-form" @submit.prevent="onSubmit">
    <h2 class="font-semibold">{{ category ? t("settings.categories.edit") : t("settings.categories.add") }}</h2>
    <div class="grid gap-4 sm:grid-cols-3">
      <FormField :label="t('settings.categories.name')" for="category-name" :error="fieldErrors.name">
        <input id="category-name" v-model="draft.name" maxlength="100" class="input" :class="{ 'input-error': fieldErrors.name }" />
      </FormField>
      <FormField :label="t('settings.categories.kind')" for="category-kind" :error="fieldErrors.kind">
        <select id="category-kind" v-model="draft.kind" class="input">
          <option v-for="k in CATEGORY_KINDS" :key="k" :value="k">{{ t(`settings.categories.kinds.${k}`) }}</option>
        </select>
      </FormField>
      <FormField :label="t('settings.vatRates.position')" for="category-position" :error="fieldErrors.position">
        <input id="category-position" v-model.number="draft.position" type="number" min="0" class="input" />
      </FormField>
    </div>
    <label class="flex items-center gap-2 text-sm">
      <input v-model="draft.active" type="checkbox" class="size-4 rounded" />
      {{ t("settings.vatRates.active") }}
    </label>
    <p v-if="error" role="alert" class="alert-error">{{ error }}</p>
    <div class="flex gap-2">
      <button type="submit" class="btn btn-primary" :disabled="submitting">{{ submitting ? t("common.saving") : t("common.save") }}</button>
      <button type="button" class="btn" @click="emit('done')">{{ t("common.cancel") }}</button>
    </div>
  </form>
</template>
