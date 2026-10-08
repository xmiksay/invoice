<script setup lang="ts">
import { useI18n } from "vue-i18n";
import type { PartySnapshot } from "../types";

/** Issued documents show their stored snapshots; drafts the live company / contact. */
defineProps<{ supplier: PartySnapshot | null; customer: PartySnapshot | null }>();
const { t } = useI18n();
</script>

<template>
  <section class="grid gap-4 sm:grid-cols-2">
    <div v-for="[role, party] in [['supplier', supplier], ['customer', customer]] as const" :key="role" class="card space-y-1 text-sm" :data-test="`party-${role}`">
      <h2 class="text-xs font-semibold tracking-wide text-gray-500 uppercase">{{ t(`documents.fields.${role}`) }}</h2>
      <template v-if="party">
        <p class="text-base font-medium">{{ party.name }}</p>
        <p v-if="party.street">{{ party.street }}</p>
        <p v-if="party.zip || party.city">{{ [party.zip, party.city].filter(Boolean).join(" ") }}<template v-if="party.country !== 'CZ'">, {{ party.country }}</template></p>
        <p v-if="party.ico">{{ t("party.ico") }}: {{ party.ico }}</p>
        <p v-if="party.dic">{{ t("party.dic") }}: {{ party.dic }}</p>
        <p v-if="party.registration" class="text-xs text-gray-500">{{ party.registration }}</p>
      </template>
      <p v-else class="text-gray-500">—</p>
    </div>
  </section>
</template>
