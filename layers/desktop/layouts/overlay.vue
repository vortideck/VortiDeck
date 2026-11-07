<template lang="pug">
    div
        WindowsTitlebar(class="z-[10001]")
          
        div(id="simulator" style="" class="h-[calc(100vh-30px)] flex flex-row bg-themeBackground justify-center items-start w-full mt-[30px]  select-none")
            //- div#overlay(class="absolute overlaying top-0 left-0 w-full h-screen bg-black opacity-40 z-10 hidden-overlay")
          
    
            div(class=" w-full  h-[calc(100vh-30px)] overflow-hidden " ) 
    
                slot()
    
                
    
    div(class="cursor cursor--small")
        
    </template>

<script setup lang="ts">
import IconFb from "~icons/ic/baseline-facebook";
import IconPhone from "~icons/ic/baseline-phone";
import IconEmail from "~icons/material-symbols/alternate-email";
import IconParkOutlineConfig from "~icons/icon-park-outline/config?width=48px&height=48px";

import { onBeforeMount } from "vue";
import { useUserStore } from "@/stores/User";
import { useMainStore } from "@/stores/Main";
const mainStore = useMainStore();

const userStore = useUserStore();
const router = useRouter();
const route = useRoute();

const props = defineProps({
  isLandscape: {
    type: Boolean,
    required: false,
  },
});
const sendEmail = () => {
  document.location = "mailto:alexi.pawelec@vortideck.com";
};

onMounted(() => {});
</script>

<style lang="scss" scoped>
#main-app {
  background: radial-gradient(black 15%, transparent 16%) 0 0,
    radial-gradient(black 15%, transparent 16%) 8px 8px,
    radial-gradient(rgba(255, 255, 255, 0.1) 15%, transparent 20%) 0 1px,
    radial-gradient(rgba(255, 255, 255, 0.1) 15%, transparent 20%) 8px 9px;
  background-color: #282828;
  background-size: 16px 16px;
}
</style>
<style lang="sass" scoped>





.app-container

    z-index: 10
    display: flex
    flex-direction: column
    justify-content: center



.panel
    margin-top: 60px
    text-align: center
    margin: 0 auto
    scroll-snap-align: center

    width: 90%
    p.title

        font-style: normal
        font-size: 5vh
        line-height: 0%
        align-self: center
        letter-spacing: 0.18em





.cursor
  position: fixed
  left: 0
  top: 0
  pointer-events: none

.cursor--small
  width: 5px
  height: 5px
  left: -2.5px
  top: -2.5px
  border-radius: 50%
  z-index: 11000
  background: var(--color-text)

.cursor--canvas
  width: 100vw
  height: 100vh
  z-index: 12000

.hidden-overlay
  visibility: hidden
  opacity: 0

.visible-overlay
  visibility: visible
  opacity: 0.4
</style>
