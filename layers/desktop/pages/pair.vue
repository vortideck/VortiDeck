<template lang="pug">
div(class="flex  flex-col w-full h-full my-auto  ")
    //- QR Code Invalidated State
    div(v-if="isQRInvalidated" class="flex flex-col w-full h-full items-center justify-center mb-[3.6rem] gap-y-6")
        //- Icon
        div(class="w-24 h-24 rounded-full bg-orange-100 dark:bg-orange-900/20 flex items-center justify-center")
            svg(class="w-12 h-12 text-orange-500" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z")

        //- Message
        div(class="text-center space-y-2 max-w-md")
            h3(class="text-xl font-semibold text-gray-800 dark:text-gray-100") {{ $t("pages.pair.qr_invalidated_title") }}
            p(class="text-gray-600 dark:text-gray-400") {{ $t("pages.pair.qr_invalidated_message") }}

        //- Generate Button
        button(@click="handleManualRegenerate" class="mt-4 px-8 py-3 bg-gradient-to-r from-blue-500 to-blue-600 hover:from-blue-600 hover:to-blue-700 text-white rounded-lg font-medium shadow-lg hover:shadow-xl transform hover:scale-105 transition-all duration-200 flex items-center gap-x-2")
            svg(class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15")
            span {{ $t("pages.pair.generate_new_qr") }}

    //- Active QR Code State
    div(v-else-if="qrCodeImage" class="flex flex-col w-full h-full items-center justify-center mb-[3.6rem] gap-y-4")
        img(:src="qrCodeImage" alt="QR Code" class="w-[55vw] aspect-square object-contain")
        p {{ $t("pages.pair.scan_qr_code") }}

    //- No QR Code State
    div(v-else class="flex flex-col w-full h-full items-center justify-center mb-[3.6rem]")
        p {{ $t("pages.pair.no_qr_code") }}

</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useMainStore } from "@/stores/Main";

interface ServiceInfo {
  ip: string;
  key: string;
  ws: string;
  host: string;
  data: string;
}

const mainStore = useMainStore();

// Reactive variables to hold the QR code image and decoded data
const qrCodeImage = ref<string | null>(null);
const qrData = ref<ServiceInfo | null>(null);
const isQRInvalidated = ref<boolean>(false);

// Store unlisten function for cleanup
let unlistenFn: (() => void) | null = null;

async function getQRCode(data: string | null = null): Promise<void> {
  try {
    console.log("🔄 Generating new QR code...");
    // Safely invoke the Tauri command and pass the optional data parameter
    const response = await invoke("generate_qr_code", { data });

    // Set the base64 image source for the QR code
    qrCodeImage.value = `data:image/png;base64,${response}`;
    console.log("✅ QR code generated successfully");

    // Reset invalidation state when new QR is generated
    isQRInvalidated.value = false;
  } catch (error) {
    console.error("❌ Failed to generate QR code:", error);
    qrCodeImage.value = null;
    qrData.value = null;
  }
}

function handleManualRegenerate() {
  console.log("🔄 Manual QR code regeneration requested");
  getQRCode();
}

onMounted(async () => {
  // Generate initial QR code
  await getQRCode();

  // Listen for pairing success event from backend
  unlistenFn = await listen("pairing-completed", (event) => {
    console.log("🎉 Pairing completed! QR code invalidated.", event.payload);

    // Mark QR as invalidated
    isQRInvalidated.value = true;

    // Show success notification using mainStore
    mainStore.addNotification({
      title: "Pairing Successful",
      message: "Device paired successfully! The QR code has been invalidated.",
      type: "success",
      autoClose: true,
    });
  });

  console.log("👂 Listening for pairing completion events");
});

onUnmounted(() => {
  // Clean up event listener
  if (unlistenFn) {
    unlistenFn();
    console.log("🔇 Stopped listening for pairing events");
  }
});
</script>

<style lang="scss"></style>
