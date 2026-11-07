<template lang="pug">
div(class="min-h-screen relative overflow-hidden")
  //- Content
  div(class="relative min-h-screen pb-28")
    //- Header
    div(class="flex items-center justify-between px-5 pt-5 pb-3")
      button(@click="navigateTo('/')" class="w-9 h-9 flex items-center justify-center rounded-xl bg-themeBackground3/80 border border-themeBorder")
        svg(class="w-4 h-4 text-themeText2" fill="none" viewBox="0 0 24 24" stroke="currentColor")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7")
      h1(class="text-lg font-semibold text-themeText") Profile
      button(@click="navigateTo('/account/personal')" class="w-9 h-9 flex items-center justify-center rounded-xl bg-themeBackground3/80 border border-themeBorder")
        svg(class="w-4 h-4 text-themeText2" fill="none" viewBox="0 0 24 24" stroke="currentColor")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z")

    //- Profile Section
    div(v-if="userInfo" class="flex flex-col items-center px-5 pt-2 pb-6")
      //- Avatar with primary ring
      div(class="relative mb-4")
        div(class="w-24 h-24 rounded-full bg-gradient-to-br from-themePrimary to-themeSecondary p-[3px]")
          div(class="w-full h-full rounded-full bg-themeBackground p-[3px]")
            div(class="w-full h-full rounded-full bg-themeHint overflow-hidden flex items-center justify-center")
              img(v-if="avatarUrl" :src="avatarUrl" alt="Profile" class="w-full h-full object-cover")
              span(v-else class="text-2xl font-bold text-themePrimary") {{ userInfo.name?.[0]?.toUpperCase() || userInfo.email?.[0]?.toUpperCase() }}
        //- Online indicator
        div(class="absolute bottom-1 right-1 w-4 h-4 bg-themeConnected rounded-full border-[3px] border-themeBackground")

      //- Name
      h2(class="text-xl font-bold text-themeText mb-1") {{ userInfo.name || 'User' }}

      //- Premium badge (from JWT userData.premium field)
      div(v-if="userInfo.premium" class="flex items-center gap-2 mb-1")
        div(class="px-2.5 py-0.5 rounded border border-themePrimary/50 bg-themePrimary/10")
          span(class="text-[11px] font-bold text-themePrimary tracking-wider uppercase") {{ userInfo.premium.plan_name || 'Premium' }}

      //- Email
      p(class="text-themeText3 text-sm") {{ userInfo.email }}

      //- Edit Profile button
      button(@click="navigateTo('/account/personal')" class="mt-4 w-48 py-2.5 rounded-xl border border-themePrimary/40 bg-themePrimary/5 text-themePrimary font-medium text-sm hover:bg-themePrimary/10 transition-all")
        | Edit Profile

    //- Loading State
    div(v-else-if="isLoading" class="flex flex-col items-center px-5 pt-2 pb-6")
      div(class="animate-pulse flex flex-col items-center")
        div(class="w-24 h-24 bg-themeHint rounded-full mb-4")
        div(class="h-5 bg-themeHint rounded-lg w-32 mb-2")
        div(class="h-4 bg-themeHint rounded-lg w-48")

    //- Hardware Status Section
    div(class="px-5 mb-5")
      h3(class="text-xs font-bold text-themePrimary tracking-wider uppercase mb-3") Hardware Status
      div(class="bg-themeBackground3/80 rounded-2xl border border-themeBorder overflow-hidden")
        button(@click="navigateTo('/account/devices')" class="w-full p-4 flex items-center gap-3 group")
          div(class="w-11 h-11 rounded-xl bg-themePrimary/10 border border-themePrimary/20 flex items-center justify-center")
            svg(class="w-5 h-5 text-themePrimary" fill="none" viewBox="0 0 24 24" stroke="currentColor")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z")
          div(class="flex-1 text-left")
            h4(class="font-medium text-themeText text-sm") VortiDeck
            p(class="text-xs text-themeText3") Manage connected devices
          div(class="flex items-center gap-2")
            div(class="flex items-center gap-1.5 px-2 py-1 rounded-lg bg-themeConnected/10")
              div(class="w-1.5 h-1.5 bg-themeConnected rounded-full animate-pulse")
              span(class="text-[11px] text-themeConnected font-medium") Connected
          svg(class="w-4 h-4 text-themeText3 ml-1" fill="none" viewBox="0 0 24 24" stroke="currentColor")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7")

    //- Account Settings Section
    div(class="px-5 mb-5")
      h3(class="text-xs font-bold text-themePrimary tracking-wider uppercase mb-3") Account Settings
      div(class="bg-themeBackground3/80 rounded-2xl border border-themeBorder overflow-hidden divide-y divide-themeBorder/40")
        //- Personal Information
        button(@click="navigateTo('/account/personal')" class="w-full p-4 flex items-center gap-3 group hover:bg-themeHover transition-colors")
          div(class="w-9 h-9 rounded-lg bg-themeHint flex items-center justify-center")
            svg(class="w-[18px] h-[18px] text-themeText2" fill="none" viewBox="0 0 24 24" stroke="currentColor")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z")
          span(class="flex-1 text-left text-sm font-medium text-themeText") Personal Information
          svg(class="w-4 h-4 text-themeText3" fill="none" viewBox="0 0 24 24" stroke="currentColor")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7")

        //- Security & Privacy
        button(@click="navigateTo('/account/security')" class="w-full p-4 flex items-center gap-3 group hover:bg-themeHover transition-colors")
          div(class="w-9 h-9 rounded-lg bg-themeHint flex items-center justify-center")
            svg(class="w-[18px] h-[18px] text-themeText2" fill="none" viewBox="0 0 24 24" stroke="currentColor")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z")
          span(class="flex-1 text-left text-sm font-medium text-themeText") Security & Privacy
          svg(class="w-4 h-4 text-themeText3" fill="none" viewBox="0 0 24 24" stroke="currentColor")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7")

        //- Billing & Subscription
        button(@click="navigateTo('/account/billing')" class="w-full p-4 flex items-center gap-3 group hover:bg-themeHover transition-colors")
          div(class="w-9 h-9 rounded-lg bg-themeHint flex items-center justify-center")
            svg(class="w-[18px] h-[18px] text-themeText2" fill="none" viewBox="0 0 24 24" stroke="currentColor")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 10h18M7 15h1m4 0h1m-7 4h12a3 3 0 003-3V8a3 3 0 00-3-3H6a3 3 0 00-3 3v8a3 3 0 003 3z")
          span(class="flex-1 text-left text-sm font-medium text-themeText") Billing & Subscription
          svg(class="w-4 h-4 text-themeText3" fill="none" viewBox="0 0 24 24" stroke="currentColor")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7")

        //- Notifications
        button(@click="navigateTo('/account/notifications')" class="w-full p-4 flex items-center gap-3 group hover:bg-themeHover transition-colors")
          div(class="w-9 h-9 rounded-lg bg-themeHint flex items-center justify-center")
            svg(class="w-[18px] h-[18px] text-themeText2" fill="none" viewBox="0 0 24 24" stroke="currentColor")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9")
          span(class="flex-1 text-left text-sm font-medium text-themeText") Notifications
          svg(class="w-4 h-4 text-themeText3" fill="none" viewBox="0 0 24 24" stroke="currentColor")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7")

    //- Appearance Section
    div(class="px-5 mb-5")
      h3(class="text-xs font-bold text-themePrimary tracking-wider uppercase mb-3") Appearance
      div(class="bg-themeBackground3/80 rounded-2xl border border-themeBorder overflow-hidden p-4")
        div(class="flex items-center gap-3 mb-3")
          div(class="w-9 h-9 rounded-lg bg-themeHint flex items-center justify-center")
            svg(class="w-[18px] h-[18px] text-themeText2" fill="none" viewBox="0 0 24 24" stroke="currentColor")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z")
          span(class="text-sm font-medium text-themeText") Theme
        div(class="grid grid-cols-3 gap-2")
          button(
            v-for="opt in themeOptions"
            :key="opt.value"
            @click="setThemeMode(opt.value)"
            :class="themeMode === opt.value ? 'border-themePrimary bg-themePrimary/10 text-themePrimary' : 'border-themeBorder bg-themeBackground text-themeText2'"
            class="flex flex-col items-center gap-1.5 py-3 rounded-xl border transition-all text-xs font-medium"
          )
            component(:is="opt.icon")
            span {{ opt.label }}

    //- Logout Button
    div(class="px-5 mb-6")
      button(@click="handleLogout" :disabled="isLoggingOut" class="w-full py-3.5 rounded-2xl bg-themeBackground3/80 border border-themeBorder flex items-center justify-center gap-2 hover:bg-themeError/10 hover:border-themeError/30 transition-all disabled:opacity-50")
        svg(v-if="isLoggingOut" class="w-4 h-4 animate-spin text-themeError" fill="none" viewBox="0 0 24 24")
          circle(class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4")
          path(class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z")
        svg(v-else class="w-4 h-4 text-themeError" fill="none" viewBox="0 0 24 24" stroke="currentColor")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1")
        span(class="text-themeError font-medium text-sm") {{ isLoggingOut ? 'Logging out...' : 'Log Out' }}

    //- Error Message
    div(v-if="errorMessage" class="px-5")
      div(class="p-3 bg-themeErrorBg border border-themeError/20 rounded-xl flex items-center gap-2")
        svg(class="w-4 h-4 text-themeError flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z")
        p(class="text-themeError text-sm") {{ errorMessage }}

</template>

<script setup lang="ts">
import { ref, onMounted, computed, h } from "vue";
import { useOAuthLogin } from "@/composables/useOAuthLogin";
import { useTheme } from "~/composables/useTheme";

definePageMeta({
  middleware: "auth",
});

const { getUserInfo, logout } = useOAuthLogin();
const { themeMode, setThemeMode } = useTheme();
const config = useRuntimeConfig();

const userInfo = ref<any>(null);
const isLoading = ref(true);
const isLoggingOut = ref(false);
const errorMessage = ref("");

const SunIcon = () => h('svg', { class: 'w-5 h-5', fill: 'none', viewBox: '0 0 24 24', stroke: 'currentColor' }, [
  h('path', { 'stroke-linecap': 'round', 'stroke-linejoin': 'round', 'stroke-width': '1.5', d: 'M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z' })
]);
const MoonIcon = () => h('svg', { class: 'w-5 h-5', fill: 'none', viewBox: '0 0 24 24', stroke: 'currentColor' }, [
  h('path', { 'stroke-linecap': 'round', 'stroke-linejoin': 'round', 'stroke-width': '1.5', d: 'M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z' })
]);
const SystemIcon = () => h('svg', { class: 'w-5 h-5', fill: 'none', viewBox: '0 0 24 24', stroke: 'currentColor' }, [
  h('path', { 'stroke-linecap': 'round', 'stroke-linejoin': 'round', 'stroke-width': '1.5', d: 'M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z' })
]);

const themeOptions = [
  { value: 'system' as const, label: 'System', icon: SystemIcon },
  { value: 'light' as const, label: 'Light', icon: SunIcon },
  { value: 'dark' as const, label: 'Dark', icon: MoonIcon },
];

onMounted(async () => {
  try {
    userInfo.value = await getUserInfo();
    isLoading.value = false;
  } catch (error) {
    console.error("Failed to load user info:", error);
    errorMessage.value = "Failed to load account information";
    isLoading.value = false;
  }
});

const avatarUrl = computed(() => {
  return userInfo.value?.avatar
    ? (userInfo.value.avatar.startsWith('http') ? userInfo.value.avatar : `${config.public.API_URL}${userInfo.value.avatar}`)
    : null;
});

const handleLogout = async () => {
  try {
    isLoggingOut.value = true;
    await logout();
    navigateTo("/login");
  } catch (error) {
    console.error("Logout failed:", error);
    errorMessage.value = "Failed to logout. Please try again.";
    isLoggingOut.value = false;
  }
};
</script>
