<template lang="pug">
div(class="min-h-screen relative overflow-hidden")
  div(class="relative min-h-screen pb-28")
    //- Header
    div(class="flex items-center justify-between px-5 pt-5 pb-3")
      button(@click="navigateTo('/account')" class="w-9 h-9 flex items-center justify-center rounded-xl bg-themeBackground3/80 border border-themeBorder")
        svg(class="w-4 h-4 text-themeText2" fill="none" viewBox="0 0 24 24" stroke="currentColor")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7")
      h1(class="text-lg font-semibold text-themeText") Connected Devices
      div(class="w-9")

    div(class="px-5 pt-2")
      //- Connection Status
      div(class="mb-5")
        h3(class="text-xs font-bold text-themePrimary tracking-wider uppercase mb-3") Status
        div(class="bg-themeBackground3/80 rounded-2xl border border-themeBorder overflow-hidden p-4")
          div(class="flex items-center justify-between")
            div(class="flex items-center gap-3")
              div(class="w-10 h-10 rounded-xl flex items-center justify-center" :class="isPaired ? 'bg-themeConnected/10 border border-themeConnected/20' : 'bg-themeHint border border-themeBorder'")
                svg(class="w-5 h-5" :class="isPaired ? 'text-themeConnected' : 'text-themeText3'" fill="none" viewBox="0 0 24 24" stroke="currentColor")
                  path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z")
              div
                p(class="text-sm font-medium text-themeText") {{ isPaired ? 'Connected' : 'Disconnected' }}
                p(class="text-xs text-themeText3 mt-0.5") {{ isPaired ? 'Desktop is paired and active' : 'No desktop connected' }}
            //- Mode badge
            span(v-if="isPaired" class="text-[11px] font-medium px-2.5 py-1 rounded-full" :class="connectionStore.getConnectionMode === 'relay' ? 'bg-themeSecondary/10 text-themeSecondary border border-themeSecondary/30' : 'bg-themePrimary/10 text-themePrimary border border-themePrimary/30'") {{ connectionStore.getConnectionMode === 'relay' ? 'Relay' : 'Direct' }}

      //- Active Desktop Card
      div(v-if="isPaired" class="mb-5")
        h3(class="text-xs font-bold text-themePrimary tracking-wider uppercase mb-3") Active Desktop
        div(class="bg-themeBackground3/80 rounded-2xl border border-themeBorder overflow-hidden")
          //- Desktop hero
          div(class="p-5 flex items-center gap-4")
            div(class="w-14 h-14 rounded-2xl bg-themePrimary/10 border border-themePrimary/20 flex items-center justify-center flex-shrink-0")
              svg(class="w-7 h-7 text-themePrimary" fill="none" viewBox="0 0 24 24" stroke="currentColor")
                path(stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z")
            div(class="flex-1 min-w-0")
              p(class="text-base font-semibold text-themeText truncate") {{ serverInfo?.name || 'Desktop' }}
              p(v-if="serverInfo?.type" class="text-xs text-themeText3 mt-0.5") {{ serverInfo.type }}
              div(class="flex items-center gap-2 mt-1.5")
                span(class="text-[10px] font-medium px-2 py-0.5 rounded-full bg-themeConnected/10 text-themeConnected border border-themeConnected/20") Connected
                span(v-if="connectedIp" class="text-[10px] font-mono text-themeText3") {{ connectedIp }}

          //- Connection details
          div(class="border-t border-themeBorder/40")
            div(v-for="item in connectionDetails" :key="item.label" class="px-4 py-3 flex items-center justify-between border-b border-themeBorder/20 last:border-b-0")
              span(class="text-xs text-themeText3") {{ item.label }}
              span(class="text-xs font-mono text-themeText2 text-right truncate max-w-[60%]") {{ item.value }}

      //- No device connected
      div(v-else class="mb-5")
        div(class="bg-themeBackground3/80 rounded-2xl border border-themeBorder overflow-hidden p-8 text-center")
          div(class="w-14 h-14 mx-auto mb-3 rounded-full bg-themeHint flex items-center justify-center")
            svg(class="w-7 h-7 text-themeText3" fill="none" viewBox="0 0 24 24" stroke="currentColor")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M18.364 5.636a9 9 0 010 12.728m-2.829-2.829a5 5 0 000-7.07m-4.243 9.9a9 9 0 01-4.243-1.757M9.879 16.12a3 3 0 01-1.415-2.12")
          p(class="text-sm text-themeText3") No desktop is currently connected
          p(class="text-xs text-themeText3 mt-1") Open VortiDeck on your desktop and pair via QR code

      //- Connection strategy
      div(class="mb-5")
        h3(class="text-xs font-bold text-themePrimary tracking-wider uppercase mb-3") Connection Strategy
        div(class="bg-themeBackground3/80 rounded-2xl border border-themeBorder overflow-hidden p-4")
          p(class="text-sm font-medium text-themeText") Preferred route
          p(class="text-xs text-themeText3 mt-1 mb-3") Automatic prefers LAN, then Tailscale, and falls back to the VortiDeck relay.
          div(class="grid grid-cols-3 gap-2")
            button(
              v-for="option in connectionPreferences"
              :key="option.value"
              @click="connectionStore.setConnectionPreference(option.value)"
              :class="connectionStore.getConnectionPreference === option.value ? 'border-themePrimary bg-themePrimary/10 text-themePrimary' : 'border-themeBorder bg-themeBackground text-themeText2'"
              class="min-w-0 px-2 py-3 rounded-xl border transition-all"
            )
              span(class="block text-xs font-semibold") {{ option.label }}
              span(class="block text-[10px] opacity-70 mt-0.5") {{ option.hint }}
          p(v-if="connectionStore.getConnectionPreference !== 'automatic'" class="text-[11px] text-themeConnecting mt-3") This override applies on the next desktop selection or full reconnect.

      //- Open Deck button
      button(
        @click="navigateTo('/deck')"
        class="w-full py-3.5 bg-themeButton text-themeText rounded-xl font-semibold text-sm hover:opacity-90 transition-opacity flex items-center justify-center gap-2"
      )
        svg(class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z")
        span Open Deck

</template>

<script setup lang="ts">
import { computed } from "vue";
import { useConnectionStore, type ConnectionPreference } from "@/stores/Connection";

definePageMeta({
  middleware: "auth",
});

const connectionStore = useConnectionStore();

const connectionPreferences: Array<{ value: ConnectionPreference; label: string; hint: string }> = [
  { value: 'automatic', label: 'Automatic', hint: 'Recommended' },
  { value: 'direct', label: 'Direct only', hint: 'No relay' },
  { value: 'relay', label: 'Relay only', hint: 'Cloud route' },
];

const isPaired = computed(() => connectionStore.getStatus === "paired");

// Desktop info comes from serverInfo (sent via server_info WebSocket message)
const serverInfo = computed(() => {
  const info = connectionStore.getServerInfo;
  return info && Object.keys(info).length > 0 ? info : null;
});

const connectedIp = computed(() => {
  const url = connectionStore.getWsUrl;
  if (!url) return null;
  try {
    return new URL(url).hostname;
  } catch {
    return null;
  }
});

const connectionDetails = computed(() => {
  const items: { label: string; value: string }[] = [];
  const mode = connectionStore.getConnectionMode;
  if (mode) items.push({ label: 'Mode', value: mode === 'relay' ? 'Relay' : 'Direct' });
  const transport = connectionStore.getActiveConnectionTransport;
  if (transport) {
    const labels = { lan: 'Local network', tailscale: 'Tailscale', relay: 'VortiDeck relay' } as const;
    items.push({ label: 'Active route', value: labels[transport] });
  }
  const latencyMs = connectionStore.getActiveConnectionLatencyMs;
  if (latencyMs !== null) items.push({ label: 'Direct handshake', value: `${latencyMs} ms` });
  items.push({
    label: 'Relay service',
    value: connectionStore.getRelayFallbackAvailable ? 'Ready' : 'Not available',
  });
  items.push({
    label: 'Preference',
    value: connectionPreferences.find(option => option.value === connectionStore.getConnectionPreference)?.label || 'Automatic',
  });
  const desktopId = connectionStore.getActiveDesktopId;
  if (desktopId) items.push({ label: 'Desktop ID', value: desktopId });
  const sessionId = connectionStore.getRelaySessionId;
  if (sessionId) items.push({ label: 'Session ID', value: sessionId });
  return items;
});
</script>
