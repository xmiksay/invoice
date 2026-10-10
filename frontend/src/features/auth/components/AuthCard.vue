<script setup lang="ts">
/** Centered card shared by the signed-out screens (login, registration, verify, reset). */
defineProps<{ title: string }>();
</script>

<template>
  <div class="flex min-h-[70dvh] items-center justify-center">
    <div class="card w-full max-w-sm space-y-4">
      <h1 class="text-xl font-semibold">{{ title }}</h1>
      <slot />
    </div>
  </div>
</template>
