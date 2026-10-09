<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { usePdf } from "@/composables/usePdf";
import { isdocApi } from "../api";

/** "Download ISDOC" on an issued (or cancelled) document: `.isdocx`, or a plain `.isdoc` without a PDF. */
const props = defineProps<{ id: string; number: string }>();

const { t } = useI18n();
// The PDF download plumbing is file-type agnostic: blob + Content-Disposition filename.
const { busy, error, download } = usePdf();

const run = () => void download(() => isdocApi.document(props.id), `${props.number}.isdocx`);
</script>

<template>
  <span class="inline-flex flex-wrap items-center gap-2">
    <button type="button" class="btn" :disabled="busy" data-test="isdoc-download" @click="run">{{ t("isdoc.export.document") }}</button>
    <span v-if="error" role="alert" class="alert-error" data-test="isdoc-download-error">{{ error }}</span>
  </span>
</template>
