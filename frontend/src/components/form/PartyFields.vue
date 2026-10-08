<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { aresApi } from "@/api/ares";
import type { FieldErrors } from "@/api/types";
import { useErrorText } from "@/composables/useAction";
import { fieldErrorsOf } from "@/lib/formErrors";
import { isValidIco } from "@/lib/ico";
import FormField from "./FormField.vue";
import { applyAres, type PartyDraft } from "./party";

const model = defineModel<PartyDraft>({ required: true });
const props = defineProps<{ errors: FieldErrors; idPrefix: string }>();

const { t } = useI18n();
const errorText = useErrorText();
const loading = ref(false);
const aresError = ref<string | null>(null);

const id = (name: keyof PartyDraft) => `${props.idPrefix}-${name}`;

async function loadAres() {
  const ico = model.value.ico.trim();
  aresError.value = null;
  if (!isValidIco(ico)) {
    aresError.value = t("validation.invalid_ico");
    return;
  }
  loading.value = true;
  try {
    const subject = await aresApi.lookup(ico);
    // Read model.value only after the await so edits typed meanwhile survive.
    model.value = applyAres(model.value, subject);
  } catch (err) {
    if (fieldErrorsOf(err)?.ico) {
      aresError.value = t("validation.invalid_ico");
    } else {
      aresError.value = errorText(err);
    }
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <div class="grid gap-4 sm:grid-cols-2">
    <div class="space-y-1 sm:col-span-2">
      <FormField :label="t('party.ico')" :for="id('ico')" :error="errors.ico">
        <div class="flex gap-2">
          <input
            :id="id('ico')"
            v-model="model.ico"
            class="input"
            :class="{ 'input-error': errors.ico }"
            inputmode="numeric"
            maxlength="8"
            autocomplete="off"
          />
          <button
            type="button"
            class="btn shrink-0"
            :disabled="loading || model.ico.trim() === ''"
            data-test="ares-button"
            @click="loadAres"
          >
            {{ loading ? t("party.aresLoading") : t("party.aresLoad") }}
          </button>
        </div>
      </FormField>
      <p v-if="aresError" role="alert" class="text-xs text-red-600 dark:text-red-400" data-test="ares-error">
        {{ aresError }}
      </p>
    </div>

    <FormField class="sm:col-span-2" :label="t('party.name')" :for="id('name')" :error="errors.name">
      <input :id="id('name')" v-model="model.name" class="input" :class="{ 'input-error': errors.name }" maxlength="200" />
    </FormField>

    <FormField :label="t('party.dic')" :for="id('dic')" :error="errors.dic">
      <input :id="id('dic')" v-model="model.dic" class="input" :class="{ 'input-error': errors.dic }" maxlength="14" />
    </FormField>

    <FormField :label="t('party.country')" :for="id('country')" :error="errors.country" :hint="t('party.countryHint')">
      <input :id="id('country')" v-model="model.country" class="input uppercase" :class="{ 'input-error': errors.country }" maxlength="2" />
    </FormField>

    <FormField class="sm:col-span-2" :label="t('party.street')" :for="id('street')" :error="errors.street">
      <input :id="id('street')" v-model="model.street" class="input" :class="{ 'input-error': errors.street }" />
    </FormField>

    <FormField :label="t('party.city')" :for="id('city')" :error="errors.city">
      <input :id="id('city')" v-model="model.city" class="input" :class="{ 'input-error': errors.city }" />
    </FormField>

    <FormField :label="t('party.zip')" :for="id('zip')" :error="errors.zip">
      <input :id="id('zip')" v-model="model.zip" class="input" :class="{ 'input-error': errors.zip }" />
    </FormField>
  </div>
</template>
