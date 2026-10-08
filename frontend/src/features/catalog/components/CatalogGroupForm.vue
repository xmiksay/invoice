<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import FormField from "@/components/form/FormField.vue";
import { useAction } from "@/composables/useAction";
import { useFormSubmit } from "@/composables/useFormSubmit";
import { catalogItemsApi } from "../api";
import { toGroupDraft, toGroupInput, validateGroup } from "../form";
import { useCatalogGroupsStore } from "../store";
import type { CatalogGroup, CatalogItem } from "../types";
import GroupMembersEditor from "./GroupMembersEditor.vue";

const props = defineProps<{ group?: CatalogGroup }>();
const emit = defineEmits<{ done: [] }>();

const { t } = useI18n();
const store = useCatalogGroupsStore();
const { fieldErrors, error, submitting, submit } = useFormSubmit();
const { error: loadError, run } = useAction();

const draft = ref(toGroupDraft(props.group));
/** Every item, unfiltered by the items tab search, as member choices. */
const items = ref<CatalogItem[]>([]);
onMounted(() => void run(async () => void (items.value = await catalogItemsApi.list())));

async function onSubmit() {
  const ok = await submit(
    () => validateGroup(draft.value),
    () => store.save(toGroupInput(draft.value), props.group?.id),
  );
  if (ok) emit("done");
}
</script>

<template>
  <form class="card space-y-4" novalidate data-test="group-form" @submit.prevent="onSubmit">
    <h2 class="font-semibold">{{ group ? t("catalog.groups.edit") : t("catalog.groups.add") }}</h2>
    <FormField :label="t('catalog.groups.name')" for="group-name" :error="fieldErrors.name" :hint="t('catalog.groups.nameHint')">
      <input id="group-name" v-model="draft.name" maxlength="200" class="input" :class="{ 'input-error': fieldErrors.name }" />
    </FormField>
    <label class="flex items-center gap-2 text-sm">
      <input id="group-collapse" v-model="draft.collapse" type="checkbox" class="size-4 rounded" />
      {{ t("catalog.groups.collapse") }}
    </label>
    <p v-if="loadError" role="alert" class="alert-error">{{ loadError }}</p>
    <GroupMembersEditor v-model="draft.members" :items="items" :errors="fieldErrors" />
    <p v-if="error" role="alert" class="alert-error" data-test="group-form-error">{{ error }}</p>
    <div class="flex gap-2">
      <button type="submit" class="btn btn-primary" :disabled="submitting" data-test="save-group">
        {{ submitting ? t("common.saving") : t("common.save") }}
      </button>
      <button type="button" class="btn" @click="emit('done')">{{ t("common.cancel") }}</button>
    </div>
  </form>
</template>
