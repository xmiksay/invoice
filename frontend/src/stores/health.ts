import { defineStore } from "pinia";
import { ref } from "vue";
import { request } from "@/api/client";
import type { HealthResponse } from "@/api/types";

export const useHealthStore = defineStore("health", () => {
  const version = ref<string | null>(null);
  const failed = ref(false);

  async function load(): Promise<void> {
    try {
      const health = await request<HealthResponse>("/api/health");
      version.value = health.version;
      failed.value = false;
    } catch {
      failed.value = true;
    }
  }

  return { version, failed, load };
});
