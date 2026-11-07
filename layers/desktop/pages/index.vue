<template lang="pug">
div(class="index-dashboard grid grid-cols-3 gap-x-4 lg:gap-x-6")
  //- Connected Devices
  div(class="index-card index-top-card index-paired-card flex flex-col p-5 pt-4 pb-4 gap-y-3 flex-grow scrollbar-visible")
      button(class="index-see-paired-btn index-see-paired-btn--floating" @click="isPairedModalOpen = true") {{$t('index.see_paired')}}
    
      div(class="index-paired-content flex flex-1 flex-col gap-y-5 overflow-y-auto")
        template(v-if="connectionStore.getPairedActive && connectionStore.getPairedActive.length > 0")
          div(class="flex w-full gap-x-2" v-for="(device, idx) in connectionStore.getPairedActive" :key="idx")
            BiPhone(class="w-6 h-6")
            span {{device.device}}
        template(v-else)
          div(class="index-paired-empty flex flex-1 flex-col items-center justify-center py-1")
            NuxtLink(to="/pair" class="index-icon-shell index-icon-shell--pair")
              IcBaselineQrcode(class="w-8 h-8 text-themeButton")
            NuxtLink(to="/pair" class="text-center text-[1.02rem] font-semibold mt-2 text-themeText") {{$t('index.pair_device')}}


  //- Premium Status
  div(class="index-card index-top-card index-status-card flex flex-col p-5 pt-4 gap-y-3")
    div(class="flex w-full justify-start items-center gap-x-3")
      Fa6SolidCrown(class="w-6 h-6 text-amber-400")
      p(class="text-[1.04rem] font-semibold text-themeText") {{$t('index.premium_status')}}
      div(class="index-soft-pill index-plan-pill ml-auto flex items-center justify-center rounded-full")
        span {{userData?.premium?.plan_name ? userData?.premium?.plan_name : 'Free Plan'}}

    p(class="index-status-copy text-themeText2")
      | Unlock advanced macros and unlimited flows.
    
    button(@click="manageSubscription" class="index-primary-btn index-status-btn mt-auto")
      span {{userData?.premium?.plan_name ? $t('index.manage_subscription') : $t('index.upgrade_account')}}


  //- System Status
  div(class="index-card index-top-card index-system-card flex flex-col p-5 pt-4 gap-y-3")
    div(class="flex w-full justify-start items-center gap-x-3")
      BiCpu(class="w-6 h-6 text-themeButton animate-pulse")
      p(class="text-[1.04rem] font-semibold text-themeText") {{$t('index.system_status')}}
      div(class="index-soft-pill index-system-pill ml-auto flex items-center justify-center rounded-full text-xs")
        span(class="flex items-center gap-x-1.5")
          span(class="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse")
          span {{$t('index.active')}}
    
    div(v-if="systemInfo" class="flex flex-col gap-y-2 text-[0.82rem] mt-auto select-none")
      div(class="flex justify-between items-center pb-0.5 border-b border-themeBorder/30")
        span(class="text-themeText2") {{$t('index.hostname')}}:
        span(class="font-semibold text-themeText truncate max-w-[130px]" :title="systemInfo.hostname") {{ systemInfo.hostname }}
      
      div(class="flex justify-between items-center pb-0.5 border-b border-themeBorder/30" v-if="serverInfo")
        span(class="text-themeText2") {{$t('index.server_url')}}:
        span(class="font-semibold text-themeText select-all truncate max-w-[140px]" :title="`http://${serverInfo.ip}:${serverInfo.port}`") http://{{ serverInfo.ip }}:{{ serverInfo.port }}
      
      div(class="flex justify-between items-center" v-if="webhookStats")
        span(class="text-themeText2") Webhooks:
        span(class="font-semibold text-themeText") {{ webhookStats.enabled }} / {{ webhookStats.total }} Active
    
    div(v-else class="flex items-center justify-center h-full text-xs text-themeText2 mt-auto pb-2")
      span {{$t('index.loading_system_info')}}

  //- Quick Access Widgets
  DashboardQuickAccessWidgets(class="col-span-3 mt-8")

  //- Recent News/Blog Posts
  section(class="index-news-shell col-span-3 mt-6")
    div(class="index-news-header flex w-full items-center gap-x-3")
      Fa6SolidRocket(class="index-news-heading-icon")
      p(class="text-[1.04rem] font-semibold text-themeText") Recent News & Updates
      div(class="index-news-pill ml-auto flex px-3 py-1 items-center justify-center text-xs font-semibold rounded-full")
        span New

    div(v-if="!pending && !error" class="index-news-grid")
      article(
        v-for="article in articles.slice(0, 2)"
        :key="article.id"
        class="index-news-feature-card group"
        @click="openArticle(article.slug)"
      )
        nuxt-img(
          :src="`https:${article.content.image}`"
          provider="storyblok"
          format="webp"
          width="1200"
          height="720"
          class="index-news-feature-image"
        )
        div(class="index-news-feature-overlay")
        div(class="index-news-feature-content")
          h3(class="index-news-feature-title") {{ article.content.title }}
          p(class="index-news-feature-subtitle") {{ article.content.description }}
        div(class="index-news-feature-meta")
          Fa6SolidPlay(class="w-3 h-3")
          span {{ article.published_at.split('T')[0] }}

    div(v-else-if="pending" class="index-news-state")
      div(class="spinner animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-themeButton")
    div(v-else-if="error" class="index-news-state flex flex-col items-center justify-center gap-2.5 py-6")
      p(class="text-xs text-themeText/70") Error loading articles, are you connected to the network?
      button(
        @click="refreshArticles()"
        class="px-3 py-1.5 bg-themeBackground hover:bg-themeBackground3 border border-themeBorder text-themeText text-xs font-semibold rounded-xl transition-colors cursor-pointer shadow-2xs"
      ) Retry

  //- Storage Info
  //- div(class="flex flex-col p-6 pt-4 rounded-xl bg-themeBackground2 gap-y-5 shadow-sm flex-grow")
  //-   div(class="flex w-full justify-start items-center gap-x-3 py-1 ")
  //-     BiUsbDrive(class="w-6 h-6 text-blue-400")
  //-     p(class="font-semibold ") {{$t('index.storage_info')}}
  //-   div(class="flex flex-col gap-y-3 bg-themeBackground3/50 p-3 rounded-lg  ")
  //-     div(class="flex items-center justify-between ")
  //-       span(class="text-sm font-medium opacity-80") {{$t('index.used_storage')}}:
  //-       span(class="text-sm font-semibold") {{ 22 }} MB <!-- storageStore.getUsedStorage() -->
  //-     div(class="w-full bg-gray-200 rounded-full h-2.5 dark:bg-gray-700")
  //-       div(class="bg-gradient-to-r from-blue-500 to-blue-600 h-2.5 rounded-full" style="width: 45%")
  //-   button(@click="storageStore.clearCache()" class=" text-white bg-red-500 hover:bg-red-600 px-3 py-2 rounded-md mt-3 transition-all duration-200 shadow-sm hover:shadow-md") {{$t('index.clear_cache')}}
  
  //- Status Togglers
  //- div(class="flex flex-col p-6 pt-4 rounded-xl bg-themeBackground2 gap-y-5 shadow-sm flex-grow")
  //-   div(class="flex w-full justify-start items-center gap-x-3 py-1 ")
  //-     BiShieldLock(class="w-6 h-6 text-green-400")
  //-     p(class="font-semibold ") Status Togglers
  //-   div(class="flex flex-col gap-y-3 bg-themeBackground3/50 p-3 rounded-lg")
  //-     div(class="flex items-center justify-between py-2 border-b border-gray-200 dark:border-gray-700")
  //-       div(class="flex items-center gap-x-2")
  //-         BiIncognito(class="w-5 h-5 text-purple-400")
  //-         span(class="text-sm font-medium") Private Mode
  //-       label(class="relative inline-flex items-center cursor-pointer")
  //-         input(type="checkbox" v-model="privateMode" class="sr-only peer")
  //-         div(class="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600")
      
  //-     div(class="flex items-center justify-between py-2 border-b border-gray-200 dark:border-gray-700")
  //-       div(class="flex items-center gap-x-2")
  //-         BiMoonStars(class="w-5 h-5 text-indigo-400")
  //-         span(class="text-sm font-medium") Do Not Disturb
  //-       label(class="relative inline-flex items-center cursor-pointer")
  //-         input(type="checkbox" v-model="doNotDisturb" class="sr-only peer")
  //-         div(class="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600")
      
  //-     div(class="flex items-center justify-between py-2")
  //-       div(class="flex items-center gap-x-2")
  //-         BiGlobe(class="w-5 h-5 text-blue-400")
  //-         span(class="text-sm font-medium") Offline Mode
  //-       label(class="relative inline-flex items-center cursor-pointer")
  //-         input(type="checkbox" v-model="offlineMode" class="sr-only peer")
  //-         div(class="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600")

  //- Theme Selector
  //- div(class="flex flex-col p-6 pt-4 rounded-xl bg-themeBackground2 gap-y-5 shadow-sm h-max")
  //-   div(class="flex w-full justify-start items-center gap-x-3 py-1")
  //-     BiPalette(class="w-6 h-6 text-purple-400")
  //-     p(class="font-semibold") {{$t('index.current_theme')}}
  //-   div(class="flex flex-col gap-y-3 bg-themeBackground3/50 p-3 rounded-lg")
  //-     select(class="p-2 rounded-md bg-themeBackground3 text-themeText w-full" v-model="selectedTheme" @change="changeTheme")
  //-       option(v-for="theme in themes" :key="theme.value" :value="theme.value") {{ theme.name }}
  
  //- Smart Actions
  //- div(class="flex flex-col p-6 pt-4 rounded-xl bg-themeBackground2 gap-y-5 shadow-sm h-max")
  //-   div(class="flex w-full justify-start items-center gap-x-3")
  //-     BiLightningChargeFill(class="w-6 h-6 text-amber-400")
  //-     p(class="font-semibold") Smart Actions
  //-   div(class="flex flex-col gap-y-3 bg-themeBackground3/50 p-3 rounded-lg")
  //-     div(class="grid grid-cols-2 gap-3")
  //-       button(class="flex items-center justify-center gap-x-2 bg-gradient-to-r from-blue-500 to-blue-600 hover:from-blue-600 hover:to-blue-700 text-white px-3 py-2 rounded-md transition-all duration-200 shadow-sm hover:shadow-md")
  //-         //- BiPlayCircle(class="w-4 h-4")
  //-         span {{$t('obs_actions.obs_stream_start')}}
  //-       button(class="flex items-center justify-center gap-x-2 bg-gradient-to-r from-red-500 to-red-600 hover:from-red-600 hover:to-red-700 text-white px-3 py-2 rounded-md transition-all duration-200 shadow-sm hover:shadow-md")
  //-         //- BiStopCircle(class="w-4 h-4")
  //-         span {{$t('obs_actions.obs_stream_stop')}}
  //-       button(class="flex items-center justify-center gap-x-2 bg-gradient-to-r from-green-500 to-green-600 hover:from-green-600 hover:to-green-700 text-white px-3 py-2 rounded-md transition-all duration-200 shadow-sm hover:shadow-md")
  //-         //- BiRecordCircle(class="w-4 h-4")
  //-         span {{$t('obs_actions.obs_recording_start')}}
  //-       button(class="flex items-center justify-center gap-x-2 bg-gradient-to-r from-purple-500 to-purple-600 hover:from-purple-600 hover:to-purple-700 text-white px-3 py-2 rounded-md transition-all duration-200 shadow-sm hover:shadow-md")
  //-         //- BiMicMute(class="w-4 h-4")
  //-         span {{$t('obs_actions.obs_mic_mute_toggle')}}
      
      
      
  ModalPaired(:isOpen="isPairedModalOpen" @close="isPairedModalOpen = false")
  ModalInstruction(:isOpen="desktopUtils.getIsShowInstruction && !mainStore.getUpdate" qrCodeUrl="download_client_qr.png" @updatePreference="handlePreferenceUpdate")

//- button( @click="sendMessage()") send Message
</template>

<script setup lang="ts">
import { onMounted } from "vue";

import BiCpu from "~icons/bi/cpu?width=16px&height=16px";
import IcBaselineQrcode from "~icons/ic/baseline-qrcode?width=24px&height=24px";
import BiPhone from "~icons/bi/phone?width=16px&height=16px";
import Fa6SolidCrown from "~icons/fa6-solid/crown?width=648px&height=576px";
import Fa6SolidRocket from "~icons/fa6-solid/rocket?width=16px&height=16px";
import Fa6SolidPlay from "~icons/fa6-solid/play?width=16px&height=16px";

import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-shell";
import { listen } from "@tauri-apps/api/event";

import { useConnectionStore } from "@/stores/Connection";
import { useDesktopUtils } from "../../desktop/stores/desktopUtils";
import { useMainStore } from "@/stores/Main";
import { useAuthStore } from "@/stores/Auth";
// import { getUserDataFromJwt } from "../../../layers/base/composables/getUserDataFromJwt";

const authStore = useAuthStore();

const userData = computed(() => {
  return getUserDataFromJwt(authStore.getAccessToken?.token);
});

const desktopUtils = useDesktopUtils();
const mainStore = useMainStore();

const systemInfo = ref<any>(null);
const serverInfo = ref<any>(null);
const webhookStats = ref<any>(null);

const refreshDashboardData = async () => {
  try {
    const [sys, srv, stats] = await Promise.all([
      invoke("get_system_info_universal_cmd"),
      invoke("get_server_info_new"),
      invoke("get_webhook_statistics"),
    ]);
    systemInfo.value = sys;
    serverInfo.value = srv;
    webhookStats.value = stats;
  } catch (err) {
    console.error("Failed to load server/system stats:", err);
  }

  if (error.value || !articles.value?.length) {
    await refreshArticles();
  }
};

// onMounted(async () => {
//   await mainStore.$tauri.start();
// });
const isPairedModalOpen = ref(false);

const connectionStore = useConnectionStore();

await invoke("get_app_config").then((c) => {
  // console.log(c);
  if (desktopUtils.getIsShowInstruction === null)
    desktopUtils.setIsShowinstruction(c.data.show_instruction);
});

const handlePreferenceUpdate = async (dontShowAgain: boolean) => {
  const new_show_instruction = !dontShowAgain;

  if (
    dontShowAgain !== null &&
    new_show_instruction !== desktopUtils.getIsShowInstruction
  ) {
    const res = (await invoke("update_app_config", {
      newShowInstruction: new_show_instruction,
    })) as { status: number; message: string };

    console.log(res.message);
    desktopUtils.setIsShowinstruction(false);
  } else {
    // just close
    desktopUtils.setIsShowinstruction(false);
  }
};

const manageSubscription = async () => {
  await open(`https://vortideck.com/account`);
};

const perPage = 3;
const page = ref(1);
const storyblokApi = useStoryblokApi();

const { data: articlesData, pending, error, refresh: refreshArticles } = await useAsyncData(
  "articles",
  async () => {
    const { data } = await storyblokApi.get("cdn/stories", {
      version: "published",
      starts_with: "blog",
      is_startpage: false,
      per_page: perPage,
      resolve_relations: "article.author,article.category",
      page: page.value,
    });

    return data.stories;
  },
  {
    server: true,
    immediate: true,
  }
);

const articles = computed(() => (articlesData.value ? toRaw(articlesData.value) : []));

const { isOnline } = useNetworkStatus();

watch(isOnline, (online) => {
  if (online) {
    refreshDashboardData();
  }
});

onMounted(() => {
  refreshDashboardData();
});

const openArticle = (slug: string) => {
  open(`https://vortideck.com/blog/article/${slug}`);
};

</script>

<style lang="scss">
.index-dashboard .index-card {
  border-radius: 1rem;
  border: 1px solid hsl(var(--border2));
  background: hsl(var(--background2));
  box-shadow: 0 10px 30px -22px hsl(var(--text) / 0.2);
}

.index-dashboard .index-icon-shell {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 4rem;
  height: 4rem;
  border-radius: 1rem;
  border: 1px solid hsl(var(--border2));
  background: hsl(var(--background3));
}

.index-dashboard .index-top-card {
  min-height: 10.35rem;
  border: none;
}

.index-dashboard .index-top-card .index-icon-shell {
  width: 3.5rem;
  height: 3.5rem;
  border-radius: 0.9rem;
}

.index-dashboard .index-icon-shell--pair {
  position: relative;
  width: 3.45rem;
  height: 3.45rem;
  aspect-ratio: 1 / 1;
  border: none;
  border-radius: 50% !important;
  background: radial-gradient(circle at 35% 35%, hsl(var(--button) / 0.06) 0%, hsl(var(--button) / 0.12) 100%);
  box-shadow: inset 0 1px 0 hsl(var(--background) / 0.85);
  overflow: visible;
  isolation: isolate;
}

.index-dashboard .index-icon-shell--pair::before,
.index-dashboard .index-icon-shell--pair::after {
  content: "";
  position: absolute;
  inset: -2px;
  border-radius: 50%;
  background: radial-gradient(
    circle,
    hsl(var(--button) / 0.38) 0%,
    hsl(var(--button) / 0.21) 42%,
    hsl(var(--button) / 0.08) 68%,
    hsl(var(--button) / 0) 100%
  );
  opacity: 0;
  pointer-events: none;
  z-index: -1;
}

.index-dashboard .index-icon-shell--pair::before {
  animation: pair-wave 4.4s cubic-bezier(0.22, 0.68, 0.24, 1) infinite;
}

.index-dashboard .index-icon-shell--pair::after {
  animation: pair-wave 4.4s cubic-bezier(0.22, 0.68, 0.24, 1) 2.2s infinite;
}

@keyframes pair-wave {
  0% {
    transform: scale(0.9);
    opacity: 0;
  }
  4% {
    opacity: 0.5;
    transform: scale(1.02);
  }
  12% {
    opacity: 0;
    transform: scale(1.55);
  }
  100% {
    transform: scale(1.55);
    opacity: 0;
  }
}

.index-dashboard .index-soft-pill {
  background: hsl(var(--background3));
  color: hsl(var(--text2));
  border: 1px solid hsl(var(--border2));
}

.index-dashboard .index-primary-btn {
  background: linear-gradient(90deg, hsl(var(--button)), hsl(var(--button) / 0.82));
  color: #fff;
  border-radius: 0.6rem;
  padding: 0.62rem 1rem;
  font-weight: 600;
  box-shadow: 0 8px 16px -10px hsl(var(--button) / 0.6);
  transition: all 0.2s ease;
  display: flex;
  align-items: center;
  justify-content: center;
}

.index-dashboard .index-primary-btn:hover {
  filter: brightness(1.06);
  transform: translateY(-1px);
}

.index-dashboard .index-error-state {
  width: 100%;
  justify-content: center;
  border: 1px solid hsl(var(--error) / 0.25);
  background: hsl(var(--errorBg));
  border-radius: 0.55rem;
  padding: 0.65rem 0.75rem;
  color: hsl(var(--error));
}

.index-dashboard .index-news-pill {
  background: hsl(var(--button) / 0.12);
  color: hsl(var(--button));
}

.index-dashboard .index-news-shell {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
}

.index-dashboard .index-news-header {
  padding: 0 0.35rem;
}

.index-dashboard .index-news-heading-icon {
  width: 1.55rem;
  height: 1.55rem;
  padding: 0.34rem;
  color: hsl(var(--button));
  border: 1px solid hsl(var(--button) / 0.2);
  border-radius: 0.5rem;
  background: hsl(var(--button) / 0.1);
}

.index-dashboard .index-news-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 1rem;
}

.index-dashboard .index-news-feature-card {
  position: relative;
  min-height: 12.9rem;
  border-radius: 1rem;
  overflow: hidden;
  border: 1px solid hsl(var(--border2) / 0.48);
  background: #0b1426;
  cursor: pointer;
  isolation: isolate;
  transition: transform 0.2s ease, box-shadow 0.2s ease, border-color 0.2s ease;
}

.index-dashboard .index-news-feature-card:hover {
  transform: translateY(-2px);
  border-color: hsl(var(--editor-accent, 20 96% 55%) / 0.36);
  box-shadow: 0 12px 30px -22px rgba(7, 18, 35, 0.9);
}

.index-dashboard .index-news-feature-image {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
  filter: saturate(1.03) contrast(1.02);
  transition: transform 0.25s ease;
}

.index-dashboard .index-news-feature-card:hover .index-news-feature-image {
  transform: scale(1.02);
}

.index-dashboard .index-news-feature-overlay {
  position: absolute;
  inset: 0;
  background: linear-gradient(
    180deg,
    rgba(3, 8, 20, 0.08) 0%,
    rgba(3, 8, 20, 0.36) 44%,
    rgba(3, 8, 20, 0.9) 100%
  );
}

.index-dashboard .index-news-feature-content {
  position: absolute;
  left: 1.2rem;
  right: 1.2rem;
  bottom: 1.05rem;
  z-index: 2;
}

.index-dashboard .index-news-feature-title {
  color: #ffffff;
  font-size: 1.58rem;
  line-height: 1.12;
  font-weight: 700;
  margin-bottom: 0.18rem;
  text-wrap: balance;
  text-shadow: 0 3px 12px rgba(0, 0, 0, 0.38);
}

.index-dashboard .index-news-feature-subtitle {
  color: rgba(236, 242, 252, 0.9);
  font-size: 0.8rem;
  line-height: 1.24;
  max-width: 94%;
}

.index-dashboard .index-news-feature-meta {
  position: absolute;
  right: 1rem;
  bottom: 1rem;
  z-index: 3;
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  border-radius: 999px;
  background: rgba(5, 10, 22, 0.76);
  color: rgba(240, 246, 255, 0.9);
  border: 1px solid rgba(224, 234, 255, 0.22);
  padding: 0.22rem 0.55rem;
  font-size: 0.62rem;
  font-weight: 600;
}

.index-dashboard .index-news-state {
  min-height: 12rem;
  display: flex;
  align-items: center;
  justify-content: center;
}

@media (min-width: 768px) {
  .index-dashboard .index-news-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 1024px) {
  .index-dashboard .index-news-feature-title {
    font-size: 1.3rem;
  }
}

@media (max-width: 640px) {
  .index-dashboard .index-news-feature-card {
    min-height: 11.5rem;
  }

  .index-dashboard .index-news-feature-title {
    font-size: 1.08rem;
  }

  .index-dashboard .index-news-feature-subtitle {
    font-size: 0.74rem;
  }
}

.index-dashboard .index-status-card {
  justify-content: flex-start;
}

.index-dashboard .index-plan-pill {
  font-size: 0.72rem;
  font-weight: 600;
  padding: 0.22rem 0.62rem;
  background: hsl(var(--button) / 0.1);
  color: hsl(var(--button));
  border-color: hsl(var(--button) / 0.2);
}

.index-dashboard .index-status-copy {
  font-size: 0.9rem;
  line-height: 1.36;
  margin-top: auto;
}

.index-dashboard .index-status-btn {
  border-radius: 0.58rem;
  padding-top: 0.42rem;
  padding-bottom: 0.42rem;
  font-size: 0.95rem;
}

.index-dashboard .index-extension-card {
  justify-content: flex-start;
}

.index-dashboard .index-extension-status {
  border-radius: 0.58rem;
  border: none;
  min-height: 2.05rem;
  padding: 0.34rem 0.68rem;
  text-align: center;
  display: flex;
  align-items: center;
  justify-content: center;
}

.index-dashboard .index-extension-status--warn {
  background: hsl(var(--error) / 0.08);
  color: hsl(var(--error));
}

.index-dashboard .index-extension-status--ok {
  border-color: hsl(var(--success) / 0.3);
  background: hsl(var(--successBg));
  color: hsl(var(--success));
}

.index-dashboard .index-extension-coming-soon {
  border-radius: 0.58rem;
  min-height: 2.05rem;
  padding: 0.34rem 0.9rem;
  font-weight: 600;
  font-size: 0.88rem;
  color: hsl(var(--text3));
  background: hsl(var(--background3));
  border: none;
  display: flex;
  align-items: center;
  justify-content: center;
}

.index-dashboard .index-system-card {
  justify-content: flex-start;
}

.index-dashboard .index-system-pill {
  font-size: 0.72rem;
  font-weight: 600;
  padding: 0.22rem 0.62rem;
  background: hsl(var(--success) / 0.1);
  color: hsl(var(--success));
  border-color: hsl(var(--success) / 0.2);
  border-width: 1px;
}

.index-dashboard .index-see-paired-btn {
  color: hsl(var(--button));
  font-size: 0.8rem;
  font-weight: 600;
  line-height: 1;
  padding: 0.22rem 0.5rem;
  border-radius: 999px;
  transition: color 0.2s ease, background 0.2s ease;
}

.index-dashboard .index-see-paired-btn:hover {
  color: hsl(var(--button) / 0.85);
  background: hsl(var(--button) / 0.1);
}

.index-dashboard .index-paired-card {
  position: relative;
}

.index-dashboard .index-see-paired-btn--floating {
  position: absolute;
  top: 0.75rem;
  right: 0.9rem;
  z-index: 2;
}

.index-dashboard .index-paired-content {
  min-height: 0;
}

.index-dashboard .index-paired-empty {
  margin-top: 0.35rem;
}
</style>
