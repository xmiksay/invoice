<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { CONTACT_CONDITION as condition } from "../templateDetail";
import { TEMPLATE_VARIABLES, variableKey } from "../variables";

const { t } = useI18n();
// Kept out of the locale files: `{` starts a vue-i18n placeholder.
const placeholder = (name: string) => `{{ ${name} }}`;
</script>

<template>
  <details class="text-sm" data-test="template-variables">
    <summary class="cursor-pointer font-medium select-none">{{ t("email.templates.variables") }}</summary>
    <p class="mt-2 text-gray-600 dark:text-gray-400">
      {{ t("email.templates.syntax") }} <code class="font-mono text-xs">{{ condition }}</code>
    </p>
    <dl class="mt-2 grid grid-cols-1 gap-x-4 gap-y-1 sm:grid-cols-[auto_1fr]">
      <template v-for="name in TEMPLATE_VARIABLES" :key="name">
        <dt><code class="font-mono text-xs">{{ placeholder(name) }}</code></dt>
        <dd class="text-gray-600 dark:text-gray-400">{{ t(variableKey(name)) }}</dd>
      </template>
    </dl>
  </details>
</template>
