<script setup lang="ts">
import { onMounted, reactive, ref, useTemplateRef } from "vue";
import { useI18n } from "vue-i18n";
import FormField from "@/components/form/FormField.vue";
import { useFormSubmit } from "@/composables/useFormSubmit";
import { rolesUpTo } from "@/features/spaces/roles";
import type { Role } from "@/features/spaces/types";
import { collectErrors, nullIfEmpty, textRule } from "@/lib/formErrors";
import { useTokensStore } from "../store";

const props = defineProps<{ role: Role }>();
const emit = defineEmits<{ close: [] }>();
const { t } = useI18n();
const store = useTokensStore();

const roles = rolesUpTo(props.role);
// Least privilege by default; the user raises it deliberately.
const form = reactive<{ name: string; role: Role; expiresAt: string }>({ name: "", role: "accountant", expiresAt: "" });
const { fieldErrors, error, submitting, submit } = useFormSubmit();
/** The secret, held only in this dialog; gone when it closes. */
const secret = ref<string | null>(null);
const copied = ref(false);

/** Local "today" as `YYYY-MM-DD` (the date input's `min`). */
function today(): string {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}
const minDate = today();

async function onSubmit() {
  await submit(
    () => collectErrors({ name: textRule(form.name, { required: true, max: 100 }), expiresAt: form.expiresAt !== "" && form.expiresAt < minDate && "invalid" }),
    async () => {
      const created = await store.create({ name: form.name.trim(), role: form.role, expiresAt: nullIfEmpty(form.expiresAt) });
      secret.value = created.token;
    },
  );
}

async function copy() {
  if (!secret.value) return;
  try {
    await navigator.clipboard.writeText(secret.value);
    copied.value = true;
  } catch {
    // Clipboard blocked (permissions, insecure context): the token stays selectable in the field.
  }
}

const panel = useTemplateRef<HTMLElement>("panel");
onMounted(() => panel.value?.focus());
</script>

<template>
  <div class="fixed inset-0 z-40 flex items-start justify-center overflow-y-auto bg-black/40 p-4 sm:p-8" @click.self="secret || emit('close')">
    <div ref="panel" role="dialog" aria-modal="true" aria-labelledby="token-dialog-title" tabindex="-1" class="card w-full max-w-md space-y-4 outline-none" data-test="token-dialog" @keydown.esc="emit('close')">
      <template v-if="secret">
        <h2 id="token-dialog-title" class="text-lg font-semibold">{{ t("tokens.created.title") }}</h2>
        <p class="rounded-md bg-amber-50 px-3 py-2 text-sm text-amber-800 dark:bg-amber-950/50 dark:text-amber-300">{{ t("tokens.created.warning") }}</p>
        <FormField :label="t('tokens.created.label')" for="token-secret">
          <div class="flex gap-2">
            <input id="token-secret" :value="secret" readonly class="input font-mono" data-test="token-secret" @focus="($event.target as HTMLInputElement).select()" />
            <button type="button" class="btn" data-test="token-copy" @click="copy">{{ t("tokens.created.copy") }}</button>
          </div>
        </FormField>
        <p v-if="copied" role="status" class="text-sm text-green-700 dark:text-green-400">{{ t("tokens.created.copied") }}</p>
        <div class="flex justify-end">
          <button type="button" class="btn btn-primary" data-test="token-done" @click="emit('close')">{{ t("tokens.created.done") }}</button>
        </div>
      </template>
      <form v-else class="space-y-4" novalidate @submit.prevent="onSubmit">
        <h2 id="token-dialog-title" class="text-lg font-semibold">{{ t("tokens.create.title") }}</h2>
        <FormField :label="t('tokens.create.name')" for="token-name" :error="fieldErrors.name">
          <input id="token-name" v-model="form.name" maxlength="100" class="input" :class="{ 'input-error': fieldErrors.name }" data-test="token-name" />
        </FormField>
        <FormField :label="t('tokens.create.role')" for="token-role" :error="fieldErrors.role">
          <select id="token-role" v-model="form.role" class="input" :class="{ 'input-error': fieldErrors.role }" data-test="token-role">
            <option v-for="r in roles" :key="r" :value="r">{{ t(`spaces.roles.${r}`) }}</option>
          </select>
        </FormField>
        <FormField :label="t('tokens.create.expires')" for="token-expires" :error="fieldErrors.expiresAt" :hint="t('tokens.create.expiresHint')">
          <input id="token-expires" v-model="form.expiresAt" type="date" :min="minDate" class="input" :class="{ 'input-error': fieldErrors.expiresAt }" data-test="token-expires" />
        </FormField>
        <p v-if="error" role="alert" class="alert-error" data-test="token-error">{{ error }}</p>
        <div class="flex justify-end gap-2">
          <button type="button" class="btn" @click="emit('close')">{{ t("common.cancel") }}</button>
          <button type="submit" class="btn btn-primary" :disabled="submitting" data-test="token-create">
            {{ submitting ? t("tokens.create.submitting") : t("tokens.create.submit") }}
          </button>
        </div>
      </form>
    </div>
  </div>
</template>
