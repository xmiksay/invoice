<script setup lang="ts">
import { computed } from "vue";
import { useRoute } from "vue-router";
import AppHeader from "@/components/AppHeader.vue";

const route = useRoute();
const showHeader = computed(() => route.name !== undefined && route.name !== "login");
</script>

<template>
  <div class="flex min-h-dvh flex-col">
    <AppHeader v-if="showHeader" />
    <main class="mx-auto w-full max-w-5xl flex-1 px-4 py-6 sm:px-6">
      <RouterView />
    </main>
  </div>
</template>
