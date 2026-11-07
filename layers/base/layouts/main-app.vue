<template lang="pug">
div(:class="mainStore.getTheme === 'dark' ? 'theme-default-dark' : 'theme-default-light'" class="")

    WindowsTitlebar
    div(id="main-app" style="" class="min-h-screen flex flex-row bg-themeBackground justify-center items-start w-full   ")
        div#overlay(class="absolute overlaying top-0 left-0 w-full h-screen bg-black opacity-40 z-10 hidden-overlay")
        
        div(  class=" flex flex-col gap-y-4 lg:gap-y-8 w-full  h-auto  mb-[9.1dvh] p-2 lg:px-20  lg:ml-[23rem] lg:mt-[4rem] " ) 
          //- mt-[9.1dvh]
          //- //min-h-[calc(100dvh-18.2dvh)]
          Navbar()
          <slot class="" />
          <!-- Footer(v-if="route.path!=='/login' && route.path!=='/signup'" class="mt-12") -->
    div(class="cursor cursor--small")
    
</template>

<script setup lang="ts">
import IconFb from "~icons/ic/baseline-facebook";
import IconPhone from "~icons/ic/baseline-phone";
import IconEmail from "~icons/material-symbols/alternate-email";
import { onBeforeMount } from "vue";
import { useUserStore } from "@base/stores/User";
import { useMainStore } from "@base/stores/Main";
const mainStore = useMainStore();

const userStore = useUserStore();
const router = useRouter();
const route = useRoute();

const sendEmail = () => {
  document.location = "mailto:alexi.pawelec@vortideck.com";
};

onMounted(() => {});
</script>

<style lang="sass">



*
    scroll-snap-type: y mandatory

body
    overflow-x: hidden



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
