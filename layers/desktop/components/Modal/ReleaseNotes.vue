<template lang="pug">
BaseModal(:is-open="isOpen" shell-class="scrollbar-visible" @close="closeModal")
  div(class="vd-modal-card relative w-full max-w-[40rem] max-h-[75vh] flex flex-col")
        // Header
      
        div(class="flex items-center justify-between px-6 pt-6 pb-3 border-b border-themeBorder2")
          div(class="flex flex-col gap-y-1")
              h3(class="text-3xl font-bold text-themeText2 tracking-tight") {{$t('modals.release_notes.title')}}
              p(class="text-sm text-themeText3") {{$t('modals.release_notes.current_version')}} {{appVersion}}
          button(v-if="!mainStore.getUpdate?.autoUpdate" @click="closeModal" class="vd-modal-close-btn")
            span(class="sr-only") {{$t('modals.release_notes.close')}}
            svg(class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")
        
        // Body
        div(class="px-6 pb-6 overflow-y-auto release-notes-content")
          div(v-if="update" class="gap-y-4 relative")
            div(class="absolute top-2 right-4 float-right bg-themeInfo/20 text-themeInfo px-3 py-1 rounded-full text-xs font-medium") {{$t('modals.release_notes.release_notes')}}
            
            MDC(v-if="formattedNotes" class="prose prose-invert max-w-none" :value="formattedNotes")
          div(v-else class="text-center py-8")
            p(class="text-themeText3") {{$t('modals.release_notes.no_notes')}}

        // Footer
        div(class="vd-modal-footer px-6 py-4 flex justify-between items-center rounded-b-xl mt-auto")
          //- div(class="flex items-center" v-if="!mainStore.getUpdate?.autoUpdate")
          //-   input(type="checkbox" v-model="mainStore.dontShowReleaseNotes" id="dontShowAgain" class="mr-2 rounded bg-themeBackground2 border-white/20")
          //-   label(for="dontShowAgain" class="text-sm text-themeText3") {{$t('modals.release_notes.dont_show_again')}}
          
          div(class="flex gap-3 ml-auto")
            button(@click="installUpdate" class="vd-modal-primary-btn px-4 py-2") {{$t('modals.release_notes.install_update')}}
</template>

<script setup lang="ts">
import BaseModal from "./BaseModal.vue";
import { ref, computed } from "vue";
import { useMainStore } from "@/stores/Main";
import { relaunch } from "@tauri-apps/plugin-process";
import { parseMarkdown } from "@nuxtjs/mdc/runtime";
import { getVersion } from "@tauri-apps/api/app";
import { startUpdate } from "@desktop/composables/useInstallUpdate";
const props = defineProps<{
  isOpen: boolean;
  update?: any;
}>();

const closeModal = () => {
  if (mainStore.getUpdate?.autoUpdate === false) {
    mainStore.setUpdate(null);
  }
};

const appVersion = await getVersion();

console.log(props.update);

const emit = defineEmits(["close", "updatePreference"]);
const mainStore = useMainStore();

const dontShowAgain = ref(false);

const formattedNotes = computed(() => {
  if (!props.update?.body) return "";

  // Remove unwanted flags
  let cleanNotes = props.update.body.split("$auto_update")[0];

  // Parse Markdown to HTML
  return parseMarkdown(cleanNotes);
});

const formatDate = (dateString: string) => {
  if (!dateString) return "";

  // Normalize date format
  const cleanedDateString = dateString
    .replace(/\.\d+/, "") // Remove milliseconds (e.g., .0)
    .replace(/ \+00:00:00$/, "Z"); // Convert timezone format to 'Z'

  const date = new Date(cleanedDateString);

  if (isNaN(date.getTime())) return dateString; // Return original if invalid

  return date.toLocaleDateString(undefined, {
    year: "numeric",
    month: "long",
    day: "numeric",
  });
};

const installUpdate = async () => {
  try {
    await startUpdate(props.update);
  } catch (error) {
    alert(error);
  }
};
</script>

<style lang="scss">
@use "./modalTheme.scss" as *;

.release-notes-content .prose {
  h2 {
    font-size: 1.5rem;
    font-weight: 600;
    margin-top: 1.5rem;
    margin-bottom: 0.75rem;
    color: hsl(var(--text2));
  }

  h3 {
    font-size: 1.25rem;
    font-weight: 600;
    margin-top: 1.25rem;
    margin-bottom: 0.5rem;
    color: hsl(var(--text2));
    &.emoji-heading {
      display: flex;
      align-items: center;
      gap: 0.5rem;

      &:before {
        font-size: 1.2em;
      }

      &:has(.emoji-🚀):before {
        content: "🚀";
      }

      &:has(.emoji-🐛):before {
        content: "🐛";
      }

      &:has(.emoji-🚜):before {
        content: "🚜";
      }
    }
  }

  ul {
    list-style-type: disc;
    padding-left: 1.5rem;
    margin-bottom: 1rem;
  }

  li {
    margin-bottom: 0.25rem;
    color: hsl(var(--text3));
  }

  p {
    margin-bottom: 0.75rem;
    color: hsl(var(--text3));
  }

  code {
    background-color: rgba(0, 0, 0, 0.2);
    padding: 0.2em 0.4em;
    border-radius: 3px;
    font-size: 0.9em;
  }
}
</style>
