<template lang="pug">
div(class=" bg-themeBackground  max-w-xl mx-auto lg:max-w-7xl  w-full ")
  //- Improved Header
  div(class=" z-10 bg-themeBackground/90 backdrop-blur-md border-b border-themeBackground3/50 ")
    div(class="container mx-auto  py-4 max-w-6xl")
      div(class="flex items-center justify-between")
        div
          h1(class="text-xl font-semibold text-themeText") {{ $t('pages.account.title') }}
          p(class="text-themeText3 text-sm mt-0.5") {{ $t('pages.account.subtitle') || 'Manage your account settings and preferences' }}
        button(@click="logout" class="px-4 py-2 bg-red-500/90 hover:bg-red-500 text-white rounded-lg transition-all duration-200 flex items-center gap-2 text-sm font-medium shadow-sm hover:shadow-md")
          PixelarticonsLogout(class="w-4 h-4")
          span {{ $t('pages.account.logout') }}

  div(class="container  max-w-xl mx-auto lg:max-w-7xl ")

    div(class="flex flex-col lg:flex-row gap-6")
      //- Enhanced Profile Card
      div(class="lg:w-1/3 w-full")
        div(class="bg-gradient-to-br from-themeBackground2 to-themeBackground2/80 rounded-xl border border-themeBackground3/60 p-5 sticky -top-2 shadow-sm")
          div(class="text-center ")
            div(class="relative inline-block mb-4")
              div(class="w-24 h-24 rounded-full flex items-center justify-center text-2xl font-semibold text-white overflow-hidden")
                NuxtImg(v-if="userData?.avatar" :src="userData.avatar.startsWith('http') ? userData.avatar : `${config.public.API_URL}${userData.avatar}`" class="w-full h-full object-cover")
                span(v-else-if="userData?.username") {{ userData.username.charAt(0).toUpperCase() }}
                PixelarticonsUser(v-else class="w-8 h-8")
              div(class="absolute -bottom-1 -right-1 w-5 h-5 bg-green-500 rounded-full border-3 border-white shadow-sm" v-if="userData?.lastLogin")
            
            h2(class="text-lg font-semibold text-themeText mb-1") {{ userData?.username || $t('pages.account.anonymous') }}
            p(class="text-themeText3 text-sm mb-4") {{ userData?.email || $t('pages.account.noEmail') }}
            
            //- Enhanced Quick Stats
            div(class="gap-y-3 gap-x-4 pt-4 border-t border-themeBackground3/40 flex flex-row lg:flex-col w-full justify-center items-center")
              div(class="bg-themeBackground/50 rounded-lg p-2 border border-themeBackground3/30 w-1/2 lg:w-full")
                div(class="text-xs text-themeText3 mb-1 font-medium") Subscription Plan
                div(class="font-semibold text-themeText flex items-center justify-center gap-2")
                  div(class="w-2 h-2 rounded-full" :class="userData?.premium ? 'bg-green-500' : 'bg-gray-400'")
                  span {{ userData?.premium ? 'Premium' : 'Free' }}
              div(class="bg-themeBackground/50 rounded-lg p-2 border border-themeBackground3/30 w-1/2 lg:w-full")
                div(class="text-xs text-themeText3 mb-1 font-medium") Last Active
                div(class="font-semibold text-themeText text-sm") {{ userData?.lastLogin ? formatTimeAgo(userData.lastLogin) : 'Never' }}

      //- Enhanced Main Content
      div(class="flex-1 lg:w-3/4 w-full flex flex-col space-y-6")
        //- Account Settings Card
        div(class="bg-gradient-to-br from-themeBackground2 to-themeBackground2/80 rounded-xl border border-themeBackground3/60 p-6 shadow-sm")
          div(class="flex items-center gap-3 mb-5")
            div(class="w-10 h-10 bg-themePrimary/10 rounded-lg flex items-center justify-center border border-themePrimary/20")
              PixelarticonsUser(class="w-5 h-5 text-themePrimary")
            div
              h3(class="text-lg font-semibold text-themeText") {{ $t('pages.account.accountSettings') }}
              p(class="text-themeText3 text-sm") Update your personal information
          
          div(class="space-y-4")
            div(class="flex flex-col md:flex-row gap-4")
              div(class="flex-1" v-if="userData")
                label(for="name" class="block text-sm font-medium text-themeText mb-2") {{ $t('pages.account.username') }}
                input(
                  id="name"
                  type="text"
                  v-model="userData.name"
                  class="w-full px-4 py-2.5 bg-themeBackground border border-themeBackground3/60 rounded-lg focus:outline-none focus:ring-2 focus:ring-themePrimary/50 focus:border-themePrimary text-themeText transition-all duration-200"
                  :placeholder="$t('pages.account.usernamePlaceholder') || 'Enter your username'"
                )

              div(class="flex-1" v-if="userData")
                label(for="email" class="block text-sm font-medium text-themeText mb-2") {{ $t('pages.account.email') }}
                input(
                  id="email"
                  type="email"
                  v-model="userData.email"
                  class="w-full px-4 py-2.5 bg-themeBackground border border-themeBackground3/60 rounded-lg focus:outline-none focus:ring-2 focus:ring-themePrimary/50 focus:border-themePrimary text-themeText transition-all duration-200"
                  :placeholder="$t('pages.account.emailPlaceholder') || 'Enter your email'"
                )
            
            div(class="flex justify-end pt-2")
              button(@click="updateUserData" class="px-6 py-2.5 bg-themePrimary hover:bg-themePrimaryHover text-white rounded-lg transition-all duration-200 flex items-center gap-2 text-sm font-medium disabled:opacity-50 disabled:cursor-not-allowed shadow-sm hover:shadow-md" :disabled="isUpdating") 
                div(v-if="isUpdating" class="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin")
                span {{ isUpdating ? $t('pages.account.updating') : $t('pages.account.updateProfile') }}

        //- Enhanced Subscription & Activity Flex
        div(class="flex flex-col md:flex-row gap-6")
          //- Subscription Card
          div(class="flex-1")
            div(class="bg-gradient-to-br from-themeBackground2 to-themeBackground2/80 rounded-xl border border-themeBackground3/60 p-6 shadow-sm h-full")
              div(class="flex items-center gap-3 mb-4")
                div(class="w-10 h-10 bg-purple-500/10 rounded-lg flex items-center justify-center border border-purple-500/20")
                  div(class="w-5 h-5 bg-gradient-to-br from-purple-500 to-purple-600 rounded-md")
                div
                  h3(class="text-lg font-semibold text-themeText") {{ $t('pages.account.subscription') }}
                  p(class="text-themeText3 text-sm") Manage your plan
              
              div(class="space-y-4")
                div(class="bg-themeBackground/50 rounded-lg p-4 border border-themeBackground3/30")
                  div(class="flex items-center justify-between mb-3")
                    div(class="flex items-center gap-3")
                      div(class="w-3 h-3 rounded-full" :class="userData?.premium ? 'bg-green-500' : 'bg-gray-400'")
                      div
                        div(class="font-semibold text-themeText") {{ userData?.premium?.plan_name ? userData?.premium?.plan_name.split(' ')[1] : $t('pages.account.free') }}
                        div(class="text-sm text-themeText3") {{ userData?.premium ? 'Premium Plan' : 'Free Plan' }}
                    
                    div(v-if="!userData?.premium")
                      button(class="px-4 py-1.5 bg-purple-500 hover:bg-purple-600 text-white rounded-md transition-all duration-200 text-sm font-medium shadow-sm hover:shadow-md")
                        | {{ $t('pages.account.upgrade') || 'Upgrade' }}
                
                div(v-if="userData?.premium?.current_period_end" class="text-sm text-themeText3 bg-themeBackground/50 rounded-lg p-3 border border-themeBackground3/30")
                  div(class="flex items-center gap-2 mb-1")
                    div(class="w-2 h-2 bg-green-500 rounded-full")
                    span(class="font-medium") Valid until:
                  div(class="text-themeText font-medium") {{ new Date(userData?.premium?.current_period_end).toLocaleDateString() }}

          //- Activity Card
          div(class="flex-1")
            div(class="bg-gradient-to-br from-themeBackground2 to-themeBackground2/80 rounded-xl border border-themeBackground3/60 p-6 shadow-sm h-full")
              div(class="flex items-center gap-3 mb-4")
                div(class="w-10 h-10 bg-blue-500/10 rounded-lg flex items-center justify-center border border-blue-500/20")
                  div(class="w-5 h-5 bg-gradient-to-br from-blue-500 to-blue-600 rounded-md")
                div
                  h3(class="text-lg font-semibold text-themeText") {{ $t('pages.account.activity') || 'Activity' }}
                  p(class="text-themeText3 text-sm") Recent activity
              
              div(class="space-y-4")
                div(class="bg-themeBackground/50 rounded-lg p-4 border border-themeBackground3/30")
                  div(class="flex items-center justify-between")
                    div(class="flex items-center gap-3")
                      div(class="w-8 h-8 bg-green-500/10 rounded-lg flex items-center justify-center border border-green-500/20")
                        div(class="w-3 h-3 bg-green-500 rounded-full animate-pulse")
                      div
                        div(class="font-semibold text-themeText") {{ $t('pages.account.lastLogin') }}
                        div(class="text-sm text-themeText3") {{ userData?.lastLogin ? new Date(userData.lastLogin).toLocaleString() : $t('pages.account.unknown') }}
                    
                    div(class="text-sm text-themeText3 font-medium") 
                      span(v-if="userData?.lastLogin") {{ formatTimeAgo(userData.lastLogin) }}
                      span(v-else) Never

</template>

<script setup lang="ts">
import { useAuthStore } from "@/stores/Auth";
import { useMainStore } from "@/stores/Main";
import { invoke } from "@tauri-apps/api/core";
import { useDesktopUtils } from "@desktop/stores/desktopUtils";
import { useConnectionStore } from "@/stores/Connection";
import PixelarticonsUser from "~icons/pixelarticons/user?width=24px&height=24px";
import PixelarticonsLogout from "~icons/pixelarticons/logout?width=24px&height=24px";
import IcBaselineDevices from "~icons/ic/baseline-devices?width=24px&height=24px";
import IcOutlineDevicesOther from "~icons/ic/outline-devices-other?width=24px&height=24px";
import IcBaselineDelete from "~icons/ic/baseline-delete?width=24px&height=24px";

const mainStore = useMainStore();
const authStore = useAuthStore();
const desktopUtils = useDesktopUtils();
const connectionStore = useConnectionStore();

const userData = computed(() => {
  return getUserDataFromJwt(authStore.getAccessToken?.token);
});

const config = useRuntimeConfig();
const router = useRouter();
const { t } = useI18n();

const isUpdating = ref(false);

// Simple time ago formatter
const formatTimeAgo = (dateString: string) => {
  const date = new Date(dateString);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

  if (diffDays < 1) {
    return "Today";
  } else if (diffDays < 7) {
    return `${diffDays} day${diffDays > 1 ? "s" : ""} ago`;
  } else if (diffDays < 30) {
    const weeks = Math.floor(diffDays / 7);
    return `${weeks} week${weeks > 1 ? "s" : ""} ago`;
  } else {
    return date.toLocaleDateString();
  }
};

const updateUserData = async () => {
  isUpdating.value = true;
  try {
    const result = await useFetchInterceptor(
      `${config.public.API_URL}/api/v1/accounts/update`,
      {
        method: "PUT",
        body: JSON.stringify(userData.value),
        headers: {
          "Content-Type": "application/json",
          "x-client-Type": "native",
          Authorization: `Token ${authStore.getAccessToken?.token}`,
        },
      }
    );

    if (result.data) {
      authStore.setUser({
        ...authStore.getUser,
        ...userData.value,
      });

      mainStore.addNotification({
        title: t("pages.account.profileUpdated") || "Profile Updated",
        message:
          t("pages.account.profileUpdatedSuccess") ||
          "Your profile has been updated successfully.",
        type: "success",
        autoClose: true,
      });
    }
  } catch (error) {
    mainStore.addNotification({
      title: t("pages.account.updateFailed") || "Update Failed",
      message:
        t("pages.account.updateFailedMessage") ||
        "Failed to update your profile. Please try again.",
      type: "error",
      autoClose: true,
    });
  } finally {
    isUpdating.value = false;
  }
};

const disconnectDevice = async (deviceId: string) => {
  try {
    await connectionStore.disconnectDevice(deviceId);
    mainStore.addNotification({
      title: t("pages.account.deviceDisconnected"),
      message: t("pages.account.deviceDisconnectedSuccess"),
      type: "success",
      autoClose: true,
    });
  } catch (error) {
    mainStore.addNotification({
      title: t("pages.account.disconnectFailed"),
      message: t("pages.account.disconnectFailedMessage"),
      type: "error",
      autoClose: true,
    });
  }
};

const testProtectedRoute = async () => {
  const result = await useFetchInterceptor(
    `${config.public.API_URL}/api/v1/accounts/protected`,
    {
      method: "GET",

      headers: {
        "Content-Type": "application/json",
        "x-client-Type": "native",
        accept: "application/json",
        Authorization: `Token ${authStore.getAccessToken?.token}`,
      },
    }
  );
  console.log(result);
  // console.log(tokensStore.getAccessToken?.token);
};

const logout = async () => {
  const { $posthog } = useNuxtApp();
  $posthog?.capture("user_logged_out");

  authStore.logout();

  await invoke("logout_user");

  mainStore.addNotification({
    title: t("pages.account.loggedOut"),
    message: t("pages.account.loggedOutSuccess"),
    type: "logout",
    autoClose: true,
  });
  await navigateTo("/");
};

definePageMeta({
  middleware: "auth",
  layout: "default",
});
</script>

<style lang="scss"></style>
