<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import FormField from "@/components/form/FormField.vue";
import { useFormSubmit } from "@/composables/useFormSubmit";
import { collectErrors, textRule } from "@/lib/formErrors";
import { baseHost, slugError } from "../slug";
import type { CreateSpaceBody } from "../types";

const props = defineProps<{ baseUrl: string; create: (body: CreateSpaceBody) => Promise<void> }>();
const { t } = useI18n();
const form = reactive({ name: "", slug: "" });
const { fieldErrors, error, submitting, submit } = useFormSubmit();
/** Live feedback starts once the user typed into the slug (not on an untouched empty field). */
const slugTouched = ref(false);

// Slugs are lowercase only; normalizing as you type beats an "invalid" for "Firma".
// A server-side reason (`taken`) is stale once the slug changes, so it is dropped.
watch(
  () => form.slug,
  (value) => {
    const lower = value.toLowerCase();
    if (lower !== value) {
      form.slug = lower;
      return;
    }
    if (value !== "") slugTouched.value = true;
    if (fieldErrors.value.slug) {
      const { slug: _stale, ...rest } = fieldErrors.value;
      fieldErrors.value = rest;
    }
  },
);

const liveSlugError = computed(() => (slugTouched.value ? slugError(form.slug) : null));
const slugFieldError = computed(() => fieldErrors.value.slug ?? liveSlugError.value ?? undefined);
// Shown from the first keystroke so the user sees where the space will live.
const preview = computed(() => `${form.slug || "…"}.${baseHost(props.baseUrl)}`);

async function onSubmit() {
  const ok = await submit(
    () => collectErrors({ name: textRule(form.name, { required: true, max: 200 }), slug: slugError(form.slug) }),
    () => props.create({ slug: form.slug, name: form.name.trim() }),
  );
  if (!ok) return;
  form.name = "";
  form.slug = "";
  slugTouched.value = false;
}
</script>

<template>
  <form class="card space-y-4" novalidate data-test="new-space" @submit.prevent="onSubmit">
    <h2 class="text-lg font-semibold">{{ t("spaces.create.title") }}</h2>
    <div class="grid gap-4 sm:grid-cols-2">
      <FormField :label="t('spaces.create.name')" for="space-name" :error="fieldErrors.name">
        <input id="space-name" v-model="form.name" maxlength="200" class="input" :class="{ 'input-error': fieldErrors.name }" data-test="space-name" />
      </FormField>
      <FormField :label="t('spaces.create.slug')" for="space-slug" :error="slugFieldError" :hint="t('spaces.create.slugHint')">
        <input
          id="space-slug"
          v-model.trim="form.slug"
          maxlength="30"
          autocapitalize="none"
          spellcheck="false"
          class="input font-mono"
          :class="{ 'input-error': slugFieldError }"
          :aria-invalid="!!slugFieldError"
          data-test="space-slug"
        />
      </FormField>
    </div>
    <p class="text-sm text-gray-600 dark:text-gray-400">
      {{ t("spaces.create.preview") }} <span class="font-mono text-gray-900 dark:text-gray-100" data-test="slug-preview">{{ preview }}</span>
    </p>
    <p v-if="error" role="alert" class="alert-error" data-test="new-space-error">{{ error }}</p>
    <button type="submit" class="btn btn-primary" :disabled="submitting" data-test="create-space">
      {{ submitting ? t("spaces.create.submitting") : t("spaces.create.submit") }}
    </button>
  </form>
</template>
