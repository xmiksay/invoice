<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import FormField from "@/components/form/FormField.vue";
import { codeFieldReason } from "../validation";

/**
 * A TOTP / recovery code input. `mode`: `totp` (6 digits only, the enrolment check), `recovery`
 * (a recovery code), `any` (step-up: either works, the server decides).
 */
const props = withDefaults(defineProps<{ id: string; error?: string; mode?: "totp" | "recovery" | "any"; test?: string }>(), { error: undefined, mode: "any", test: undefined });
const code = defineModel<string>({ required: true });
const { t } = useI18n();

const label = computed(() => (props.mode === "recovery" ? t("mfa.code.recovery") : t("mfa.code.totp")));
const hint = computed(() => (props.mode === "any" ? t("mfa.code.anyHint") : undefined));
</script>

<template>
  <FormField :label="label" :for="id" :error="codeFieldReason(error)" :hint="hint">
    <input
      :id="id"
      v-model="code"
      :inputmode="mode === 'totp' ? 'numeric' : 'text'"
      autocomplete="one-time-code"
      autocapitalize="off"
      spellcheck="false"
      maxlength="32"
      class="input font-mono"
      :class="{ 'input-error': error }"
      :data-test="test ?? id"
    />
  </FormField>
</template>
