<template lang="pug">
BaseModal(v-if="!routing" :is-open="isOpen" shell-class="scrollbar-visible" @close="$emit('close')")
  div(class="vd-modal-card relative w-full max-w-[30rem]")
        // Header
        div(class="flex items-center justify-between p-6 border-b")
          h3(class="text-2xl font-semibold text-themeText2") {{ $t('modals.paired.title') }}
          button(@click="$emit('close')" class="vd-modal-close-btn")
            span(class="sr-only") {{ $t('modals.paired.close') }}
            svg(class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")

        // Body
        div(class="p-4 pr-2")
          div(class="flex flex-col gap-y-5 max-h-[60vh] overflow-y-auto pr-2")
            div(class="flex w-full gap-x-4 items-center p-4 hover:bg-themeBackground3 rounded-lg " v-for="(device, idx) in connectionStore.getPaired" :key="idx")
              BiPhone(class="w-8 h-8 text-themeText2")
              div(class="flex flex-col")
                span(class="font-medium") {{device.device}}
                span(class="text-sm text-gray-500") {{device.device_type}}
                span(class="text-xs text-themeText3") {{ $t('modals.paired.paired_date') }}: {{formatDate(device.paired_date)}}
              div(class="ml-auto flex gap-x-2")
                button(class="p-2 text-gray-500 hover:text-gray-700 rounded-full hover:bg-gray-100")
                  BiTrash(class="w-5 h-5")
                //- button(class="p-2 text-gray-500 hover:text-gray-700 rounded-full hover:bg-gray-100")
                //-   BiPencil(class="w-5 h-5")

        // Footer
        div(class="vd-modal-footer px-6 py-4 flex justify-between items-center rounded-b-lg")
          span(class="text-sm text-themeText3") {{connectionStore.getPairedActive.length}} {{ $t('modals.paired.devices_count') }}
          button( @click="addDevice" class="vd-modal-primary-btn px-4 py-2") {{ $t('modals.paired.add_device') }}

</template>

<script setup lang="ts">
import BaseModal from "./BaseModal.vue";
import { ref } from "vue";
import BiPhone from "~icons/bi/phone";
import BiTrash from "~icons/bi/trash";
import BiPencil from "~icons/bi/pencil";
import { useConnectionStore } from "@/stores/Connection";

const router = useRouter();
const routing = ref(false);

const addDevice = async () => {
  routing.value = true;
  await emit("close");

  router.push("/pair");
};

defineProps<{
  isOpen: boolean;
}>();

const emit = defineEmits(["close"]);
const connectionStore = useConnectionStore();

const formatDate = (date: string) => {
  return new Date(date).toLocaleDateString("en-US", {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
};
</script>

<style lang="scss">
@use "./modalTheme.scss" as *;
</style>
