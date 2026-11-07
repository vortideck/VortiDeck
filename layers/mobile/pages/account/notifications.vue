<template lang="pug">
div(class="min-h-screen relative overflow-hidden")
  div(class="relative min-h-screen pb-28")
    //- Header
    div(class="flex items-center justify-between px-5 pt-5 pb-3")
      button(@click="navigateTo('/account')" class="w-9 h-9 flex items-center justify-center rounded-xl bg-themeBackground3/80 border border-themeBorder")
        svg(class="w-4 h-4 text-themeText2" fill="none" viewBox="0 0 24 24" stroke="currentColor")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7")
      h1(class="text-lg font-semibold text-themeText") Notifications
      div(class="w-9")

    div(class="px-5 pt-2")
      //- Push Notifications
      div(class="mb-5")
        h3(class="text-xs font-bold text-themePrimary tracking-wider uppercase mb-3") Push Notifications
        div(class="bg-themeBackground3/80 rounded-2xl border border-themeBorder overflow-hidden divide-y divide-themeBorder/40")
          //- Connection alerts
          div(class="p-4 flex items-center justify-between")
            div(class="flex-1 pr-4")
              p(class="text-sm font-medium text-themeText") Connection Alerts
              p(class="text-xs text-themeText3 mt-0.5") Get notified when a device connects or disconnects
            button(
              @click="toggleSetting('connectionAlerts')"
              :class="settings.connectionAlerts ? 'bg-themePrimary' : 'bg-themeHint'"
              class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors duration-200 flex-shrink-0"
            )
              span(
                :class="settings.connectionAlerts ? 'translate-x-6' : 'translate-x-1'"
                class="inline-block h-4 w-4 transform rounded-full bg-white shadow-sm transition-transform duration-200"
              )

          //- Pairing requests
          div(class="p-4 flex items-center justify-between")
            div(class="flex-1 pr-4")
              p(class="text-sm font-medium text-themeText") Pairing Requests
              p(class="text-xs text-themeText3 mt-0.5") Alert when a new device wants to pair
            button(
              @click="toggleSetting('pairingRequests')"
              :class="settings.pairingRequests ? 'bg-themePrimary' : 'bg-themeHint'"
              class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors duration-200 flex-shrink-0"
            )
              span(
                :class="settings.pairingRequests ? 'translate-x-6' : 'translate-x-1'"
                class="inline-block h-4 w-4 transform rounded-full bg-white shadow-sm transition-transform duration-200"
              )

          //- Update alerts
          div(class="p-4 flex items-center justify-between")
            div(class="flex-1 pr-4")
              p(class="text-sm font-medium text-themeText") App Updates
              p(class="text-xs text-themeText3 mt-0.5") Notify when a new version is available
            button(
              @click="toggleSetting('appUpdates')"
              :class="settings.appUpdates ? 'bg-themePrimary' : 'bg-themeHint'"
              class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors duration-200 flex-shrink-0"
            )
              span(
                :class="settings.appUpdates ? 'translate-x-6' : 'translate-x-1'"
                class="inline-block h-4 w-4 transform rounded-full bg-white shadow-sm transition-transform duration-200"
              )

      //- Sound & Haptics
      div(class="mb-5")
        h3(class="text-xs font-bold text-themePrimary tracking-wider uppercase mb-3") Sound & Haptics
        div(class="bg-themeBackground3/80 rounded-2xl border border-themeBorder overflow-hidden divide-y divide-themeBorder/40")
          //- Sound
          div(class="p-4 flex items-center justify-between")
            div(class="flex-1 pr-4")
              p(class="text-sm font-medium text-themeText") Button Sounds
              p(class="text-xs text-themeText3 mt-0.5") Play sound when pressing deck buttons
            button(
              @click="toggleSetting('buttonSounds')"
              :class="settings.buttonSounds ? 'bg-themePrimary' : 'bg-themeHint'"
              class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors duration-200 flex-shrink-0"
            )
              span(
                :class="settings.buttonSounds ? 'translate-x-6' : 'translate-x-1'"
                class="inline-block h-4 w-4 transform rounded-full bg-white shadow-sm transition-transform duration-200"
              )

          //- Haptics
          div(class="p-4 flex items-center justify-between")
            div(class="flex-1 pr-4")
              p(class="text-sm font-medium text-themeText") Haptic Feedback
              p(class="text-xs text-themeText3 mt-0.5") Vibrate on button press
            button(
              @click="toggleSetting('hapticFeedback')"
              :class="settings.hapticFeedback ? 'bg-themePrimary' : 'bg-themeHint'"
              class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors duration-200 flex-shrink-0"
            )
              span(
                :class="settings.hapticFeedback ? 'translate-x-6' : 'translate-x-1'"
                class="inline-block h-4 w-4 transform rounded-full bg-white shadow-sm transition-transform duration-200"
              )

      //- Saved indicator
      div(v-if="showSaved" class="flex items-center justify-center gap-2 py-2")
        svg(class="w-4 h-4 text-themeConnected" fill="none" viewBox="0 0 24 24" stroke="currentColor")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7")
        span(class="text-xs text-themeConnected font-medium") Settings saved

</template>

<script setup lang="ts">
import { ref, reactive, watch } from "vue";

definePageMeta({
  middleware: "auth",
});

const STORAGE_KEY = "vortideck_notification_settings";

const settings = reactive({
  connectionAlerts: true,
  pairingRequests: true,
  appUpdates: true,
  buttonSounds: false,
  hapticFeedback: true,
});

const showSaved = ref(false);
let saveTimeout: ReturnType<typeof setTimeout> | null = null;

// Load saved settings
if (import.meta.client) {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved) {
      Object.assign(settings, JSON.parse(saved));
    }
  } catch {
    // Use defaults
  }
}

const toggleSetting = (key: keyof typeof settings) => {
  settings[key] = !settings[key];
  saveSettings();
};

const saveSettings = () => {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ ...settings }));
    showSaved.value = true;

    if (saveTimeout) clearTimeout(saveTimeout);
    saveTimeout = setTimeout(() => {
      showSaved.value = false;
    }, 2000);
  } catch {
    // Storage might be unavailable
  }
};
</script>
