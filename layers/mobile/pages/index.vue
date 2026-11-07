<template lang="pug">
div(class="min-h-screen")
  //- Content
  div(class="relative flex flex-col gap-5 max-w-lg mx-auto px-5 pt-4 pb-4")
    //- Status header
    div(class="flex justify-between items-center")
      span(class="font-bold text-xl text-themeText") Home
      div(class="flex items-center gap-2")
        div(v-if="connectionStore.getStatus === 'connecting'" class="w-2 h-2 bg-themeConnecting animate-pulse rounded-full")
        div(:class="statusBadgeClass" class="px-3 py-1 text-xs font-semibold rounded-full flex items-center gap-1.5")
          div(:class="statusDotClass" class="w-1.5 h-1.5 rounded-full")
          span.uppercase {{ connectionStore.getStatus }}

    //- Durable workflow approvals
    div(v-if="connectionStore.getStatus === 'paired' && pendingApprovals.length" class="bg-themeBackground3/80 rounded-2xl border border-amber-400/30 overflow-hidden")
      div(class="px-5 py-4 border-b border-themeBorder/50 flex items-center justify-between")
        div
          h3(class="text-themeText font-bold") Approval required
          p(class="text-themeText3 text-xs mt-0.5") {{ pendingApprovals.length }} workflow request{{ pendingApprovals.length === 1 ? '' : 's' }} waiting
      div(class="divide-y divide-themeBorder/50")
        article(v-for="approval in pendingApprovals" :key="approval.id" class="p-5")
          div(class="flex items-start justify-between gap-3")
            div(class="min-w-0")
              h4(class="text-themeText font-semibold") {{ approval.payload?.title || 'Workflow approval' }}
              p(class="text-themeText2 text-sm mt-1 whitespace-pre-wrap") {{ approval.payload?.message || 'A workflow is waiting for your decision.' }}
            span(class="text-[10px] uppercase font-bold px-2 py-1 rounded-full bg-amber-400/10 text-amber-300") {{ approval.payload?.severity || 'normal' }}
          p(v-if="approval.dueAt" class="text-themeText3 text-xs mt-3") Expires {{ new Date(approval.dueAt).toLocaleString() }}
          textarea(v-model="approvalComments[approval.id]" maxlength="2000" rows="2" placeholder="Optional comment" class="mt-3 w-full rounded-xl border border-themeBorder bg-themeBackground px-3 py-2 text-sm text-themeText")
          div(class="grid grid-cols-2 gap-3 mt-3")
            button(@click="decideApproval(approval.id, 'denied')" :disabled="approvalSubmitting.has(approval.id)" class="px-4 py-2.5 rounded-xl border border-red-400/30 text-red-300 text-sm font-semibold disabled:opacity-40") Deny
            button(@click="decideApproval(approval.id, 'approved')" :disabled="approvalSubmitting.has(approval.id)" class="px-4 py-2.5 rounded-xl bg-themeButton text-themeText text-sm font-semibold disabled:opacity-40") Approve

    //- Connect card
    div(class="bg-themeBackground3/80 rounded-2xl border border-themeBorder overflow-hidden p-5")
      div(class="flex items-center gap-4")
        div(class="flex-1")
          h3(class="text-themeText font-bold text-lg mb-1") Connect
          p(class="text-themeText2 text-sm leading-relaxed") Establish a new connection to your deck hardware.
          button(@click="showConnectionOptions = true" class="mt-3 px-5 py-2.5 bg-themeButton text-themeText text-sm font-semibold rounded-xl hover:opacity-90 transition-opacity") Connect Now
        div(class="w-20 h-20 rounded-xl overflow-hidden relative bg-themePrimary/10 border border-themePrimary/20 flex-shrink-0 flex items-center justify-center")
          svg(class="w-9 h-9 text-themePrimary/50" fill="none" viewBox="0 0 24 24" stroke="currentColor")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z")

    //- Sync card (when paired)
    div(v-if="connectionStore.getStatus === 'paired'" class="bg-themeBackground3/80 rounded-2xl border border-themeBorder overflow-hidden p-5")
      div(class="flex items-center gap-4")
        div(class="flex-1")
          h3(class="text-themeText font-bold text-lg mb-1") Sync Deck
          p(class="text-themeText2 text-sm leading-relaxed") Pull the latest deck layout from your connected desktop.
          button(@click="sendMessage()" class="mt-3 px-5 py-2.5 bg-themeButton text-themeText text-sm font-semibold rounded-xl hover:opacity-90 transition-opacity") Sync Now
        div(class="w-20 h-20 rounded-xl overflow-hidden relative bg-themePrimary/10 border border-themePrimary/20 flex-shrink-0 flex items-center justify-center")
          svg(class="w-9 h-9 text-themePrimary/50" fill="none" viewBox="0 0 24 24" stroke="currentColor")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15")

    //- Recent Decks
    RecentDecks(v-if="connectionStore.getStatus === 'paired'" class="mt-1")

    //- Getting Started
    div(class="mt-1")
      div(class="flex items-center gap-2 mb-4")
        svg(class="w-5 h-5 text-themePrimary" fill="none" viewBox="0 0 24 24" stroke="currentColor")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 3v4M3 5h4M6 17v4m-2-2h4m5-16l2.286 6.857L21 12l-5.714 2.143L13 21l-2.286-6.857L5 12l5.714-2.143L13 3z")
        h2(class="font-bold text-lg text-themeText") Getting Started

      div(class="flex gap-3 overflow-x-auto pb-2 -mx-5 px-5 scrollbar-hide")
        div(class="flex-shrink-0 w-40 bg-themeBackground3/80 rounded-xl p-4 border border-themeBorder")
          div(class="w-8 h-8 bg-themePrimary/10 rounded-full flex items-center justify-center text-themePrimary font-bold text-sm mb-3 border border-themePrimary/20") 1
          svg(class="w-6 h-6 text-themeText2 mb-2" fill="none" viewBox="0 0 24 24" stroke="currentColor")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4")
          p(class="text-themeText2 text-xs leading-relaxed") Download the VortiDeck desktop client on your PC.

        div(class="flex-shrink-0 w-40 bg-themeBackground3/80 rounded-xl p-4 border border-themeBorder")
          div(class="w-8 h-8 bg-themePrimary/10 rounded-full flex items-center justify-center text-themePrimary font-bold text-sm mb-3 border border-themePrimary/20") 2
          svg(class="w-6 h-6 text-themeText2 mb-2" fill="none" viewBox="0 0 24 24" stroke="currentColor")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.141 0M1.394 9.393c5.857-5.858 15.355-5.858 21.213 0")
          p(class="text-themeText2 text-xs leading-relaxed") Ensure both devices are on the same Wi-Fi network.

        div(class="flex-shrink-0 w-40 bg-themeBackground3/80 rounded-xl p-4 border border-themeBorder")
          div(class="w-8 h-8 bg-themePrimary/10 rounded-full flex items-center justify-center text-themePrimary font-bold text-sm mb-3 border border-themePrimary/20") 3
          svg(class="w-6 h-6 text-themeText2 mb-2" fill="none" viewBox="0 0 24 24" stroke="currentColor")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 4v1m6 11h2m-6 0h-2v4m0-11v3m0 0h.01M12 12h4.01M16 20h4M4 12h4m12 0h.01M5 8h2a1 1 0 001-1V5a1 1 0 00-1-1H5a1 1 0 00-1 1v2a1 1 0 001 1zm12 0h2a1 1 0 001-1V5a1 1 0 00-1-1h-2a1 1 0 00-1 1v2a1 1 0 001 1zM5 20h2a1 1 0 001-1v-2a1 1 0 00-1-1H5a1 1 0 00-1 1v2a1 1 0 001 1z")
          p(class="text-themeText2 text-xs leading-relaxed") Scan the QR code shown on your desktop app.

        div(class="flex-shrink-0 w-40 bg-themeBackground3/80 rounded-xl p-4 border border-themeBorder")
          div(class="w-8 h-8 bg-themePrimary/10 rounded-full flex items-center justify-center text-themePrimary font-bold text-sm mb-3 border border-themePrimary/20") 4
          svg(class="w-6 h-6 text-themeText2 mb-2" fill="none" viewBox="0 0 24 24" stroke="currentColor")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z")
          p(class="text-themeText2 text-xs leading-relaxed") Open a deck and start controlling your PC!

    //- Help & Support
    div(class="mt-1")
      div(class="flex items-center gap-2 mb-4")
        svg(class="w-5 h-5 text-themePrimary" fill="none" viewBox="0 0 24 24" stroke="currentColor")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z")
        h2(class="font-bold text-lg text-themeText") Help & Support

      div(class="space-y-2")
        div(class="bg-themeBackground3/80 rounded-xl border border-themeBorder overflow-hidden")
          button(@click="toggleHelp('connection')" class="w-full flex items-center justify-between px-4 py-3.5")
            span(class="text-sm font-semibold text-themeText2 uppercase tracking-wider") Connection Issues
            svg(:class="{'rotate-180': expandedHelp.has('connection')}" class="w-4 h-4 text-themeText3 transition-transform duration-200" fill="none" viewBox="0 0 24 24" stroke="currentColor")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7")
          div(v-if="expandedHelp.has('connection')" class="px-4 pb-4 border-t border-themeBorder/50")
            ul(class="text-themeText2 text-sm space-y-2 mt-3 list-inside list-disc")
              li Set network type to private on your desktop
              li Ensure same network connection
              li Check if desktop app is running
              li Try restarting both apps
              li Disable antivirus temporarily

        div(class="bg-themeBackground3/80 rounded-xl border border-themeBorder overflow-hidden")
          button(@click="toggleHelp('controls')" class="w-full flex items-center justify-between px-4 py-3.5")
            span(class="text-sm font-semibold text-themeText2 uppercase tracking-wider") Controls Not Working
            svg(:class="{'rotate-180': expandedHelp.has('controls')}" class="w-4 h-4 text-themeText3 transition-transform duration-200" fill="none" viewBox="0 0 24 24" stroke="currentColor")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7")
          div(v-if="expandedHelp.has('controls')" class="px-4 pb-4 border-t border-themeBorder/50")
            ul(class="text-themeText2 text-sm space-y-2 mt-3 list-inside list-disc")
              li Verify "paired" status
              li Check deck configuration
              li Review app permissions

    //- Connection Options Modal
    div(v-if="showConnectionOptions" class="fixed inset-0 bg-black/90 backdrop-blur-sm z-50 flex items-center justify-center p-6" @click.self="showConnectionOptions = false")
      div(class="w-full max-w-md bg-themeBackground3 rounded-2xl border border-themeBorder overflow-hidden")
        div(class="p-6 border-b border-themeBorder/50")
          h2(class="text-xl font-bold text-themeText mb-1") Connect to Desktop
          p(class="text-themeText2 text-sm") Choose how to connect

        div(class="p-5 space-y-3")
          button(@click="connectLocal" class="w-full p-5 bg-themePrimary/5 border border-themePrimary/20 rounded-xl hover:border-themePrimary/40 transition-all")
            div(class="flex items-center gap-4")
              div(class="w-11 h-11 bg-themePrimary/10 rounded-xl flex items-center justify-center border border-themePrimary/20")
                svg(class="w-5 h-5 text-themePrimary" fill="none" viewBox="0 0 24 24" stroke="currentColor")
                  path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 18h.01M8 21h8a2 2 0 002-2V5a2 2 0 00-2-2H8a2 2 0 00-2 2v14a2 2 0 002 2z")
              div(class="flex-1 text-left")
                h3(class="font-semibold text-themeText") Local Network
                p(class="text-sm text-themeText2") Same WiFi (QR Code)

          button(@click="connectRelay" :disabled="!isLoggedIn" class="w-full p-5 bg-themePrimary/5 border border-themePrimary/20 rounded-xl hover:border-themePrimary/40 transition-all disabled:opacity-40")
            div(class="flex items-center gap-4")
              div(class="w-11 h-11 bg-themePrimary/10 rounded-xl flex items-center justify-center border border-themePrimary/20")
                svg(class="w-5 h-5 text-themePrimary" fill="none" viewBox="0 0 24 24" stroke="currentColor")
                  path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3.055 11H5a2 2 0 012 2v1a2 2 0 002 2 2 2 0 012 2v2.945M8 3.935V5.5A2.5 2.5 0 0010.5 8h.5a2 2 0 012 2 2 2 0 104 0 2 2 0 012-2h1.064M15 20.488V18a2 2 0 012-2h3.064M21 12a9 9 0 11-18 0 9 9 0 0118 0z")
              div(class="flex-1 text-left")
                h3(class="font-semibold text-themeText") Remote Relay
                p(class="text-sm text-themeText2") {{ isLoggedIn ? 'Connect from anywhere' : 'Login required' }}

          div(v-if="!isLoggedIn" class="p-3 bg-themeConnectingBg border border-themeConnecting/20 rounded-xl")
            p(class="text-themeConnecting text-sm text-center mb-2") Login to use remote relay
            button(@click="router.push('/login')" class="w-full px-4 py-2.5 bg-themeButton text-themeText rounded-xl font-medium text-sm") Login

        div(class="p-4 border-t border-themeBorder/50")
          button(@click="showConnectionOptions = false" class="w-full px-4 py-2.5 text-themeText2 hover:text-themeText transition-colors text-sm") Cancel

    //- Desktop Picker Modal (Relay)
    div(v-if="showDesktopPicker" class="fixed inset-0 bg-black/90 backdrop-blur-sm z-50 flex items-center justify-center p-6" @click.self="showDesktopPicker = false")
      div(class="w-full max-w-md h-[80vh] bg-themeBackground3 rounded-2xl border border-themeBorder overflow-hidden flex flex-col")
        div(class="p-6 border-b border-themeBorder/50")
          h2(class="text-xl font-bold text-themeText mb-1") Select Desktop
          p(class="text-themeText2 text-sm") Choose which desktop to control via relay

        div(class="flex-1 overflow-hidden")
          DesktopPicker(:desktops="relayDesktops" :loading="loadingDesktops" @select="handleDesktopSelected" @refresh="loadDesktops")

        div(class="p-4 border-t border-themeBorder/50")
          button(@click="showDesktopPicker = false" class="w-full px-4 py-2.5 text-themeText2 hover:text-themeText transition-colors text-sm") Cancel

    //- Local Desktop Picker Modal
    div(v-if="showLocalDesktopPicker" class="fixed inset-0 bg-black/90 backdrop-blur-sm z-50 flex items-center justify-center p-6" @click.self="showLocalDesktopPicker = false")
      div(class="w-full max-w-md h-[80vh] bg-themeBackground3 rounded-2xl border border-themeBorder overflow-hidden flex flex-col")
        div(class="p-6 border-b border-themeBorder/50")
          h2(class="text-xl font-bold text-themeText mb-1") Local Desktops
          p(class="text-themeText2 text-sm") Discovered on your network

        div(class="flex-1 overflow-hidden")
          DesktopPicker(:desktops="localDesktops" :loading="loadingLocalDesktops" @select="handleLocalDesktopSelected" @refresh="loadLocalDesktops" localMode)

        div(class="p-4 space-y-3 border-t border-themeBorder/50")
          button(@click="scanQRCode" class="w-full px-4 py-2.5 bg-themeButton text-themeText rounded-xl hover:opacity-90 transition-opacity text-sm font-medium") Scan QR Code Instead
          button(@click="showLocalDesktopPicker = false" class="w-full px-4 py-2.5 text-themeText2 hover:text-themeText transition-colors text-sm") Cancel
</template>

<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { useConnectionStore } from "@/stores/Connection";
import { useApprovalsStore } from "@/stores/Approvals";
import { useRelayDesktops } from "~/composables/useRelayDesktops";
import { useLocalDesktops } from "~/composables/useLocalDesktops";
import { vibrate } from "@tauri-apps/plugin-haptics";
import { keepScreenOn } from "tauri-plugin-keep-screen-on-api";
import { useAuthStore } from "@/stores/Auth";

const { $socket, $sendSignedMessage } = useNuxtApp();
const router = useRouter();
const connectionStore = useConnectionStore();
const approvalsStore = useApprovalsStore();
const pendingApprovals = computed(() => approvalsStore.pending);
const approvalComments = ref<Record<string, string>>({});
const approvalSubmitting = ref(new Set<string>());

const decideApproval = async (wakeId: string, decision: "approved" | "denied") => {
  if (approvalSubmitting.value.has(wakeId)) return;
  approvalSubmitting.value = new Set([...approvalSubmitting.value, wakeId]);
  try {
    await $sendSignedMessage({
      action: "workflow_approval_decision",
      protocolVersion: 1,
      wakeId,
      decision,
      comment: approvalComments.value[wakeId]?.trim() || null,
    });
  } finally {
    const next = new Set(approvalSubmitting.value);
    next.delete(wakeId);
    approvalSubmitting.value = next;
  }
};

watch(
  () => connectionStore.getStatus,
  (status) => {
    if (status === "paired") {
      void $sendSignedMessage({ action: "list_workflow_approvals", protocolVersion: 1 });
    } else {
      approvalsStore.replace([]);
    }
  },
  { immediate: true },
);
const { getRelayDesktops, selectDesktop } = useRelayDesktops();
const { getLocalDesktops, connectToLocalDesktop } = useLocalDesktops();
const authStore = useAuthStore();

const showConnectionOptions = ref(false);
const showDesktopPicker = ref(false);
const showLocalDesktopPicker = ref(false);
const relayDesktops = ref([]);
const localDesktops = ref([]);
const loadingDesktops = ref(false);
const loadingLocalDesktops = ref(false);
const expandedHelp = ref(new Set<string>());

const isLoggedIn = computed(() => authStore.isLoggedIn);

const statusBadgeClass = computed(() => {
  const s = connectionStore.getStatus;
  if (s === 'paired') return 'bg-themePairedBg text-themePaired border border-themePaired/30';
  if (s === 'connecting') return 'bg-themeConnectingBg text-themeConnecting border border-themeConnecting/30';
  return 'bg-themeDisconnectedBg text-themeDisconnected border border-themeDisconnected/30';
});

const statusDotClass = computed(() => {
  const s = connectionStore.getStatus;
  if (s === 'paired') return 'bg-themePaired';
  if (s === 'connecting') return 'bg-themeConnecting animate-pulse';
  return 'bg-themeDisconnected';
});

const toggleHelp = (key: string) => {
  const s = expandedHelp.value;
  if (s.has(key)) s.delete(key);
  else s.add(key);
  expandedHelp.value = new Set(s);
};

keepScreenOn(false);

const connectLocal = async () => {
  showConnectionOptions.value = false;
  showLocalDesktopPicker.value = true;
  await loadLocalDesktops();
};

const connectRelay = async () => {
  showConnectionOptions.value = false;
  showDesktopPicker.value = true;
  await loadDesktops();
};

const loadDesktops = async () => {
  try {
    loadingDesktops.value = true;
    const { desktops } = await getRelayDesktops();
    relayDesktops.value = desktops;
  } catch (error) {
    console.error("Failed to load desktops:", error);
    alert("Failed to load desktops. Please try again.");
  } finally {
    loadingDesktops.value = false;
  }
};

const loadLocalDesktops = async () => {
  try {
    loadingLocalDesktops.value = true;
    const desktops = await getLocalDesktops();
    localDesktops.value = desktops;

    if (desktops.length === 0) {
      alert(
        "No local desktops found. Make sure the desktop app is running and you're on the same WiFi network."
      );
    }
  } catch (error) {
    console.error("Failed to discover local desktops:", error);
    alert("Failed to discover local desktops. Please try again.");
  } finally {
    loadingLocalDesktops.value = false;
  }
};

const handleDesktopSelected = async (desktop: any) => {
  try {
    showDesktopPicker.value = false;
    connectionStore.setStatus("connecting");
    await selectDesktop(desktop.desktop_id);
    router.push("/deck");
  } catch (error) {
    console.error("Failed to connect to desktop:", error);
    alert("Failed to connect to desktop. Please try again.");
    connectionStore.setStatus("disconnected");
  }
};

const handleLocalDesktopSelected = async (desktop: any) => {
  try {
    showLocalDesktopPicker.value = false;
    console.log("Selected local desktop:", desktop);

    try {
      const { load } = await import("@tauri-apps/plugin-store");
      const store = await load("store.json", { autoSave: true, defaults: {} });
      const allKeys = await store.keys();
      const secretKeys = allKeys.filter((key) => key.startsWith("e2ee_shared_secret_"));

      if (secretKeys.length > 0) {
        console.log("Found saved E2EE secret(s)! Attempting to connect...");
        connectionStore.setStatus("connecting");
        await connectToLocalDesktop(desktop);
        router.push("/deck");
        return;
      } else {
        console.log("No saved E2EE secrets found - need to pair via QR");
      }
    } catch (error) {
      console.error("Failed to check for saved secrets:", error);
    }

    console.log("Navigating to QR scanner for pairing");
    router.push("/qr");
  } catch (error) {
    console.error("Failed to connect to local desktop:", error);
    alert("Failed to connect to local desktop. Please try again.");
    connectionStore.setStatus("disconnected");
  }
};

const scanQRCode = () => {
  showLocalDesktopPicker.value = false;
  router.push("/qr");
};

const sendMessage = async () => {
  console.log("Sending message");
  await vibrate(2);

  if ($socket.value) {
    const get_deck_message = {
      action: "get_current_deck",
    };

    $sendSignedMessage(get_deck_message);
    alert("Updating Deck");
  } else {
    alert("WebSocket is not open, cannot send message.");
  }
};
</script>

<style scoped>
.scrollbar-hide {
  -ms-overflow-style: none;
  scrollbar-width: none;
}
.scrollbar-hide::-webkit-scrollbar {
  display: none;
}
</style>
