import { defineStore } from "pinia";
import { ref } from "vue";
import { useSessionStore } from "@/stores/session";
import { mfaApi } from "./api";
import type { MfaConfirmBody, MfaSetup, MfaStatus } from "./types";

/** Account → two-factor authentication. Recovery codes are returned to the caller, never kept here. */
export const useMfaStore = defineStore("mfa", () => {
  const status = ref<MfaStatus | null>(null);

  async function load(): Promise<void> {
    status.value = await mfaApi.status();
  }

  function patch(next: Partial<MfaStatus>): void {
    if (status.value) status.value = { ...status.value, ...next };
  }

  const setup = (password: string): Promise<MfaSetup> => mfaApi.setup(password);

  async function enable(code: string): Promise<string[]> {
    const { recoveryCodes } = await mfaApi.enable(code);
    patch({ enabled: true, recoveryCodesLeft: recoveryCodes.length });
    useSessionStore().setMfaEnabled(true);
    return recoveryCodes;
  }

  async function regenerate(body: MfaConfirmBody): Promise<string[]> {
    const { recoveryCodes } = await mfaApi.regenerate(body);
    patch({ recoveryCodesLeft: recoveryCodes.length });
    return recoveryCodes;
  }

  async function disable(body: MfaConfirmBody): Promise<void> {
    await mfaApi.disable(body);
    patch({ enabled: false, recoveryCodesLeft: 0 });
    useSessionStore().setMfaEnabled(false);
  }

  return { status, load, setup, enable, regenerate, disable };
});
