<template lang="pug">
div(class="w-full h-auto grid grid-cols-4 gap-x-2 p-2 px-4 border-t border-themeBorder items-center bg-themeBackground/95 backdrop-blur-xl")
    NuxtLink(v-for="(item, index) in menu" :key="index" :to="item.to" class="nav-item flex flex-col items-center justify-center py-2 relative")
      div(:class="route.path === item.to ? 'bg-themePrimary' : 'bg-transparent'" class="absolute top-0 left-1/2 -translate-x-1/2 w-6 h-[2px] rounded-full transition-all duration-300")
      component(:is="item.icon" :class="route.path === item.to ? 'text-themePrimary' : 'text-themeText3'" class="h-[1.6rem] transition-colors duration-200")
      span(:class="route.path === item.to ? 'text-themePrimary' : 'text-themeText3'" class="text-[10px] font-medium mt-1 uppercase tracking-wider transition-colors duration-200") {{ item.title }}

</template>

<script setup lang="ts">
import MarketeqQrCodeScan2 from "~icons/marketeq/qr-code-scan-2?width=24px&height=24px";
import PepiconsPencilKeyboard from "~icons/pepicons-pencil/keyboard?width=24px&height=24px";
import IconoirHome from "~icons/iconoir/home?width=24px&height=24px";
import IconParkOutlineSettings from "~icons/icon-park-outline/setting-two?width=24px&height=24px";

const route = useRoute();

const menu = ref({
  home: {
    title: "Home",
    icon: IconoirHome,
    to: "/",
  },
  qr: {
    title: "QR Scan",
    icon: MarketeqQrCodeScan2,
    to: "/qr",
  },
  decks: {
    title: "Decks",
    icon: PepiconsPencilKeyboard,
    to: "/deck",
  },
  settings: {
    title: "Settings",
    icon: IconParkOutlineSettings,
    to: "/account",
  },
});
</script>

<style scoped>
.nav-item:active {
  transform: scale(0.95);
  transition: transform 0.1s;
}
</style>
