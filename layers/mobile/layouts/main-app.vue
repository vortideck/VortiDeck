<template lang="pug">
    div(id="main-app" class="bg-themeBackground min-h-screen w-full overflow-x-hidden")
        div#overlay(class="absolute overlaying top-0 left-0 w-full h-screen bg-black opacity-40 z-10 hidden-overlay")
        NavBottom(class="fixed bottom-0 z-50")

        div(class="w-full min-h-screen pb-[4.5rem]")
          <slot />
    div(class="cursor cursor--small")
    
    </template>

<script setup lang="ts">
import IconFb from "~icons/ic/baseline-facebook";
import IconPhone from "~icons/ic/baseline-phone";
import IconEmail from "~icons/material-symbols/alternate-email";
import { onBeforeMount } from "vue";
import { useUserStore } from "@/stores/User";
import { useMainStore } from "@/stores/Main";
const mainStore = useMainStore();

const userStore = useUserStore();
const router = useRouter();
const route = useRoute();

const sendEmail = () => {
  document.location = "mailto:alexi.pawelec@vortideck.com";
};

onMounted(() => {});

useHead({});
</script>

<style lang="sass">

html, body
  overscroll-behavior-y: auto /* Allow natural behavior */
  overflow-x: hidden /* Prevent horizontal overflow only */
:root
  touch-action: pan-y /* Allow vertical scrolling */
  height: 100%

body
    min-height: 100%




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
