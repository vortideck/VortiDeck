<template lang="pug">
BaseModal(:is-open="isOpen" shell-class="scrollbar-visible" @close="handleClose")
  div(class="vd-modal-card relative w-full max-w-[34rem] max-h-[75vh] flex flex-col")
    // Header
    div(class="flex items-center justify-between p-6 border-b border-white/10")
      h3(class="text-3xl font-bold text-themeText tracking-tight") {{$t('modals.instruction.title')}}
      button(@click="handleClose" class="vd-modal-close-btn")
        span(class="sr-only") {{$t('modals.instruction.close')}}
        svg(class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")

    // Body
    div(class="p-6 overflow-y-auto")
      p(class="text-lg text-themeText2") {{$t('modals.instruction.description')}}
      ul(class="list-disc list-inside mt-4 text-themeText2 text-sm space-y-2")
        li(class="flex items-center")
          svg(class="h-4 w-4 text-green-500" fill="none" viewBox="0 0 24 24" stroke="currentColor")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7")
          span.ml-2 {{$t('modals.instruction.install_app')}}
        li.ml-1 {{$t('modals.instruction.scan_qr')}}
        li.ml-1 {{$t('modals.instruction.customize_apps')}}
        li.ml-1 {{$t('modals.instruction.seamless_experience')}}

      div(class="mt-6 bg-themeBackground3/30 p-4 rounded-lg border border-white/10")
        div(class="flex flex-col md:flex-row items-center gap-4")
          div(class="flex-shrink-0")
            img(:src="qrCodeUrl" alt="QR Code" class="w-36 h-36 border border-white/10 rounded-lg shadow-lg bg-white p-1")
          div(class="flex-grow")
            h4(class="text-md font-semibold text-themeText mb-2") {{$t('modals.instruction.get_mobile_app')}}
            p(class="text-sm text-themeText2 mb-3") {{$t('modals.instruction.scan_qr_mobile')}}
            a(
              href="https://vortideck.com/download"
              @click.prevent="openDownloadPage"
              class="inline-flex items-center text-sm text-blue-400 hover:text-blue-300 transition-colors"
            )
              span {{$t('modals.instruction.visit_website')}} vortideck.com/download
              svg(class="ml-1 w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor")
                path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14")

    // Footer
    div(class="vd-modal-footer px-6 py-4 flex justify-between items-center rounded-b-xl mt-auto")
      div(class="flex items-center")
        input(type="checkbox" v-model="dontShowAgain" id="dontShowAgain" class="mr-2 rounded bg-themeBackground2 border-white/20")
        label(for="dontShowAgain" class="text-sm text-themeText2") {{$t('modals.instruction.dont_show_again')}}
      button(@click="handleClose" class="vd-modal-primary-btn px-4 py-2") {{$t('modals.instruction.got_it')}}
</template>

<script setup lang="ts">
import BaseModal from "./BaseModal.vue";
import { open } from "@tauri-apps/plugin-shell";
import { ref } from "vue";

defineProps<{ isOpen: boolean; qrCodeUrl: string }>();
const emit = defineEmits(["close", "updatePreference"]);

const dontShowAgain = ref(false);

const handleClose = () => {
  emit("updatePreference", dontShowAgain.value);
  emit("close");
};

const openDownloadPage = async () => {
  await open("https://vortideck.com/download");
};
</script>

<style lang="scss">
@use "./modalTheme.scss" as *;
</style>
