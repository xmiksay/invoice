<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useAction } from "@/composables/useAction";
import { leaveTo } from "@/lib/navigate";
import { useSessionStore } from "@/stores/session";
import { membersApi } from "../api";

const { t } = useI18n();
const session = useSessionStore();
const { error, run } = useAction();
const lastOwner = ref(false);
const leaving = ref(false);
const spaceName = session.me?.space?.name ?? "";

// Only an owner can be the last one, and owners may list members; everyone else relies on the
// server's 409 `last_owner`. A failed lookup just leaves the button there (the server still refuses).
onMounted(async () => {
  if (session.role !== "owner") return;
  try {
    lastOwner.value = (await membersApi.list()).filter((m) => m.role === "owner").length <= 1;
  } catch {
    // See above.
  }
});

async function leave() {
  if (!window.confirm(t("members.leave.confirm", { name: spaceName }))) return;
  leaving.value = true;
  // The membership and this host's session are gone: back to the hub.
  if (await run(membersApi.leave)) leaveTo(session.baseUrl);
  leaving.value = false;
}
</script>

<template>
  <section class="card space-y-3" data-test="leave-space">
    <h2 class="text-lg font-semibold">{{ t("members.leave.title") }}</h2>
    <p v-if="lastOwner" class="text-sm text-gray-600 dark:text-gray-400" data-test="leave-last-owner">{{ t("members.leave.lastOwner") }}</p>
    <template v-else>
      <p class="text-sm text-gray-600 dark:text-gray-400">{{ t("members.leave.intro", { name: spaceName }) }}</p>
      <p v-if="error" role="alert" class="alert-error" data-test="leave-error">{{ error }}</p>
      <button type="button" class="btn btn-danger" :disabled="leaving" data-test="leave-submit" @click="leave">{{ t("members.leave.submit") }}</button>
    </template>
  </section>
</template>
