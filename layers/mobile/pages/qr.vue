<template lang="pug">
div(class="flex flex-col w-full h-full justify-between")
  div(class="w-full h-[10vh] bg-black bg-opacity-50 justify-center items-center flex")
    p(class="text-white font-2xl") Scan Qr Code to Pair
  div(class="w-full h-[20vh] bg-black bg-opacity-50 justify-center items-center flex ")
    button(@click="exit" class=" px-4 py-2 text-black bg-white shadow-lg border-[1px] border-gray-200 rounded-xl text-2xl") Cancel
</template>

<script setup lang="ts">
import {
  scan,
  Format,
  cancel,
  requestPermissions,
  openAppSettings,
  checkPermissions,
} from "@tauri-apps/plugin-barcode-scanner";
import DeviceDetector from "device-detector-js";
import { useConnectionStore } from "@/stores/Connection";
import { useDeckStore } from "@/stores/Deck";
import { load } from "@tauri-apps/plugin-store";
import { keepScreenOn } from "tauri-plugin-keep-screen-on-api";
import { useConnectWs } from "../composables/useConnectWs";
import {
  connectionCandidateUrl,
  findReachableConnectionCandidate,
  normalizeConnectionCandidates,
} from "../utils/connectionCandidates";

const store = await load("store.json", { autoSave: false });

const deviceDetector = new DeviceDetector();
const connectionStore = useConnectionStore();
const deckStore = useDeckStore();

const scanningInProgress = ref(false);
keepScreenOn;
const router = useRouter();
definePageMeta({
  layout: "qr",
});

const exit = () => {
  cancel();
  router.push("/");
};
keepScreenOn(false);
// Function to establish a WebSocket connection
const connectToWebsocket = async (
  wsUrl: string,
  key: string,
  pairingToken?: string,
  desktopPublicKey?: string,
  desktopId?: string  // NEW: For relay mode
) => {
  // Create new WebSocket connection
  const { socket, isWebSocketConnected, connectWebSocket } = await useConnectWs(
    wsUrl,
    key,
    pairingToken,
    desktopPublicKey,
    desktopId
  );
  await connectWebSocket();
  const { $socket } = useNuxtApp();

  $socket.value = socket.value;
};

// Function to start scanning
const startScan = async () => {
  try {
    scanningInProgress.value = true;
    const result = await scan({
      windowed: true,
      formats: [Format.QRCode],
    });

    if (result) {
      scanningInProgress.value = false;

      const parsedData = JSON.parse(result.content);
      const { key, ws, pairing_token, desktop_public_key, desktop_id } = parsedData;
      const candidates = normalizeConnectionCandidates(parsedData.connection_candidates);
      const reachableCandidate = await findReachableConnectionCandidate(candidates);
      const selectedWebSocketUrl = reachableCandidate
        ? connectionCandidateUrl(reachableCandidate.candidate)
        : ws;

      if (key && selectedWebSocketUrl) {
        // 🔐 Save desktop public key and desktop_id for future reconnections
        if (desktop_id && desktop_public_key) {
          const storageKey = `desktop_public_key_${desktop_id}`;
          localStorage.setItem(storageKey, desktop_public_key);
          console.log(`🔐 Saved desktop public key for future relay connections: ${desktop_id}`);

          // 🔐 CRITICAL: Save desktop_id for token reconnections with E2EE
          // When mobile restarts and reconnects with saved token, it needs to know
          // which desktop_id to use to load the saved shared secret
          try {
            const { load } = await import('@tauri-apps/plugin-store');
            const store = await load('store.json', { autoSave: false });
            await store.set('last_paired_desktop_id', desktop_id);
            await store.save();
            console.log(`💾 Saved desktop_id to Tauri store for E2EE reconnection: ${desktop_id}`);
          } catch (error) {
            console.error('❌ Failed to save desktop_id to Tauri store:', error);
          }
        }

        // 🔐 E2EE: Pass pairing_token and desktop_public_key if available
        // 📡 Relay: Pass desktop_id if using relay mode
        await connectToWebsocket(selectedWebSocketUrl, key, pairing_token, desktop_public_key, desktop_id);
        connectionStore.setActiveConnectionTransport(reachableCandidate?.candidate.transport || 'lan');
        connectionStore.setActiveConnectionLatencyMs(reachableCandidate?.latencyMs ?? null);
        connectionStore.setRelayFallbackAvailable(false);

        if (pairing_token && desktop_public_key) {
          console.log("🔐 QR code includes E2EE parameters - secure pairing initiated");
        }

        if (desktop_id) {
          console.log("📡 QR code includes desktop_id - relay mode enabled");
        }
      }

      cancel();
      router.push("/");
    }
  } catch (error) {
    console.error("Error scanning:", error);
    scanningInProgress.value = false;
  }
};

// Function to stop scanning
const stopScan = async () => {
  try {
    scanningInProgress.value = false;
    await cancel();
  } catch (error) {
    console.error("Error stopping scan:", error);
  }
};

onMounted(async () => {
  await requestPermissions();
  const permision = await checkPermissions();
  if (permision !== "granted") {
    alert("permission not granted allow camera permission in settings");
    openAppSettings();
  }
  await startScan();
});
</script>

<style lang="scss"></style>
