<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useAction } from "@/composables/useAction";
import BankAccountForm from "../components/BankAccountForm.vue";
import { useBankAccountsStore } from "../stores";
import type { BankAccount } from "../types";

const { t } = useI18n();
const store = useBankAccountsStore();
const { error, run } = useAction();
/** null = form closed, "new" = create, otherwise the account being edited. */
const editing = ref<BankAccount | "new" | null>(null);

onMounted(() => void run(store.load));

function onDelete(a: BankAccount) {
  if (!window.confirm(t("settings.bankAccounts.confirmDelete", { name: a.label ?? a.accountNumber ?? a.iban ?? "" }))) return;
  void run(() => store.remove(a.id));
}
</script>

<template>
  <div class="space-y-4">
    <p v-if="error" role="alert" class="alert-error">{{ error }}</p>

    <BankAccountForm
      v-if="editing"
      :key="editing === 'new' ? 'new' : editing.id"
      :account="editing === 'new' ? undefined : editing"
      @done="editing = null"
    />
    <button v-else type="button" class="btn btn-primary" @click="editing = 'new'">
      {{ t("settings.bankAccounts.add") }}
    </button>

    <div class="card overflow-x-auto !p-0">
      <table class="table">
        <thead>
          <tr>
            <th>{{ t("settings.bankAccounts.label") }}</th>
            <th class="hidden sm:table-cell">{{ t("settings.bankAccounts.currency") }}</th>
            <th>{{ t("settings.bankAccounts.account") }}</th>
            <th class="w-0"><span class="sr-only">{{ t("common.actions") }}</span></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="a in store.items" :key="a.id">
            <td>
              {{ a.label }}
              <span v-if="a.isDefault" class="badge ml-1" data-test="default-badge">
                {{ t("settings.bankAccounts.defaultFor", { currency: a.currency }) }}
              </span>
            </td>
            <td class="hidden sm:table-cell">{{ a.currency }}</td>
            <td class="font-mono text-xs">
              <div v-if="a.accountNumber">{{ a.accountNumber }}</div>
              <div v-if="a.iban" class="text-gray-500">{{ a.iban }}</div>
            </td>
            <td class="whitespace-nowrap">
              <div class="flex gap-1">
                <button type="button" class="btn btn-sm" @click="editing = a">{{ t("common.edit") }}</button>
                <button type="button" class="btn btn-sm btn-danger" @click="onDelete(a)">{{ t("common.delete") }}</button>
              </div>
            </td>
          </tr>
          <tr v-if="store.loaded && store.items.length === 0">
            <td colspan="4" class="py-8 text-center text-gray-500">{{ t("settings.bankAccounts.empty") }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
