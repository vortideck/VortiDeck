<template lang="pug">
Transition(name="consent-banner")
  div(v-if="showBanner" class="fixed bottom-0 left-0 right-0 z-[9999] p-4")
    div(class="max-w-2xl mx-auto bg-gray-900/95 backdrop-blur-md border border-gray-700/50 rounded-xl px-5 py-4 shadow-2xl flex flex-col sm:flex-row items-start sm:items-center gap-4")
      p(class="text-sm text-gray-300 flex-1")
        | Help improve VortiDeck by sending anonymous usage analytics. No personal data is collected. You can change this anytime in
        span(class="text-blue-400 ml-1") Settings → Privacy
      div(class="flex gap-2 shrink-0")
        button(@click="handleReject" class="px-4 py-2 text-sm text-gray-400 hover:text-white border border-gray-600 hover:border-gray-500 rounded-lg transition-colors") No thanks
        button(@click="handleAccept" class="px-4 py-2 text-sm text-white bg-blue-600 hover:bg-blue-500 rounded-lg transition-colors font-medium") Accept
</template>

<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";

const showBanner = ref(false);

onMounted(async () => {
  try {
    const config = await invoke<{ data: { analytics_consent_given: boolean } }>("get_app_config");
    // Only show banner if the user has never made an explicit choice
    if (!config?.data?.analytics_consent_given) {
      showBanner.value = true;
    }
  } catch {
    showBanner.value = true;
  }
});

async function handleAccept() {
  showBanner.value = false;
  try {
    await invoke("update_app_config", {
      newAnalyticsEnabled: true,
      newAnalyticsConsentGiven: true,
    });
  } catch (e) {
    console.error("Failed to enable analytics:", e);
  }
}

async function handleReject() {
  showBanner.value = false;
  try {
    await invoke("update_app_config", {
      newAnalyticsEnabled: false,
      newAnalyticsConsentGiven: true,
    });
  } catch (e) {
    console.error("Failed to save analytics preference:", e);
  }
}
</script>

<style scoped>
.consent-banner-enter-active {
  transition: transform 0.4s cubic-bezier(0.16, 1, 0.3, 1), opacity 0.3s ease;
}
.consent-banner-leave-active {
  transition: transform 0.3s cubic-bezier(0.7, 0, 0.84, 0), opacity 0.2s ease;
}
.consent-banner-enter-from {
  transform: translateY(100%);
  opacity: 0;
}
.consent-banner-leave-to {
  transform: translateY(100%);
  opacity: 0;
}
</style>
