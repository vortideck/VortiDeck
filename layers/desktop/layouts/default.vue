<template lang="pug">
div(class="select-none bg-themeBackground text-themeText")
    WindowsTitlebar(class="z-[10001]")
      
    div(id="main-app" style="" class="h-[calc(100%-30px)] flex flex-row bg-themeBackground justify-start items-start w-full mt-[30px] text-themeText")
        //- div#overlay(class="absolute overlaying top-0 left-0 w-full h-screen bg-black opacity-40 z-10 hidden-overlay")
      
        Sidebar(class="" )
        Navbar(
          class=" "
          v-if="route.path!=='/applications'"
          :title="pageMeta.title? pageMeta.title : ''"
          :subtitle="pageMeta.description? pageMeta.description : ''"
        )
        button(
          v-if="canGoBack"
          type="button"
          class="fixed left-[1.5rem] top-[2.5rem] z-[9999] inline-flex h-10 w-10 items-center justify-center text-themeText focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-themePrimary"
          aria-label="Go back"
          @click="handleBack"
        )
          SolarArrowLeftLinear(class="h-6 w-6 pointer-events-none" aria-hidden="true")
        button(
          v-else
          type="button"
          class="fixed left-[1.5rem] top-[2.5rem] z-[9999] inline-flex h-10 w-10 items-center justify-center text-themeText focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-themePrimary 2xl:hidden"
          :aria-expanded="desktopUtils.getIsSidebar"
          aria-label="Open navigation"
          @click="desktopUtils.toggleSidebar()"
        )
          GgMenuLeft(class="h-6 w-6 pointer-events-none" aria-hidden="true")
        div(:class="scrollShellClasses")
          div(:class="contentShellClasses")
            //- mt-[9.1dvh]
            //- //min-h-[calc(100dvh-18.2dvh)]
            Transition(name="fade")
              div(v-if="desktopUtils.getIsSidebar" @click="desktopUtils.toggleSidebar()" tabindex="-1" class="fixed 2xl:hidden top-0 flex items-center bg-black/55 justify-center left-0 right-0 z-50 w-full p-4 overflow-x-hidden overflow-y-auto md:inset-0 h-screen md:h-full")
            slot
            <!-- Footer(v-if="route.path!=='/login' && route.path!=='/signup'" class="mt-12") -->

    //- div(class="w-full fixed bottom-0 left-0 h-[2.3rem] bg-gray-200 flex px-8 items-center border-t gap-x-6")
    //-   div(class="flex w-[17rem] gap-x-2 items-center")
    //-     p Network:
    //-     select(class="w-full focus:outline-none" v-model="selectedNetworkCard")
    //-       option(v-for="network in NetworkCards" :value="network.displayName") {{network.displayName}} | {{network.ip}}
    //-   div(class="flex max-w-[8rem] gap-x-2 items-center")
    //-     p Port:
    //-     input(class="focus:outline-none px-2 w-full")
    div(class="cursor cursor--small")
    
</template>

<script setup lang="ts">
import SolarArrowLeftLinear from "~icons/solar/arrow-left-linear?width=24px&height=24px";
import GgMenuLeft from "~icons/gg/menu-left?width=24px&height=24px";
import IconFb from "~icons/ic/baseline-facebook";
import IconPhone from "~icons/ic/baseline-phone";
import IconEmail from "~icons/material-symbols/alternate-email";
import { onBeforeMount } from "vue";
import { useUserStore } from "@/stores/User";
import { useDesktopUtils } from "@desktop/stores/desktopUtils";
import { useMainStore } from "@/stores/Main";

import Multiselect from "vue-multiselect";
import "vue-multiselect/dist/vue-multiselect.css";

const mainStore = useMainStore();
const pageMeta = computed(() => {
  return route.meta;
});
const contentPaddingPreset = computed(() => {
  const preset = route.meta?.contentPaddingPreset;
  return preset === "wide" || preset === "none" ? preset : "compact";
});
const scrollShellClasses = computed(() => [
  "layout-scroll-shell scrollbar-visible box-border 2xl:pl-[24rem] w-screen min-h-[calc(100vh-30px-4.2rem)] h-[calc(100vh-30px-4.2rem)] mt-[4.2rem]",
  route.meta?.layoutScroll === false ? "overflow-hidden" : "overflow-x-hidden overflow-y-auto",
]);
const contentShellClasses = computed(() => [
  "layout-content-shell box-border flex w-full flex-col gap-y-4 lg:gap-y-8",
  route.meta?.layoutScroll === false ? "h-full min-h-0" : "min-h-full",
  contentPaddingPreset.value === "none"
      ? "p-2 px-0"
      : contentPaddingPreset.value === "wide"
        ? "pt-5 pb-2 pl-20 pr-[calc(5rem-var(--layout-scrollbar-width))]"
        : "pt-5 pb-2 pl-6 pr-[calc(1.5rem-var(--layout-scrollbar-width))] lg:pl-8 lg:pr-[calc(2rem-var(--layout-scrollbar-width))]",
]);
const desktopUtils = useDesktopUtils();
const userStore = useUserStore();
const router = useRouter();
const route = useRoute();

const canGoBack = computed(() => {
  return route.path !== "/";
});

const handleBack = () => {
  if (typeof window !== "undefined" && window.history.length > 1) {
    router.back();
    return;
  }
  router.push("/");
};

const NetworkCards = ref([
  {
    displayName: "wlan3",
    ip: "127.0.0.1",
    port: 9001,
  },
  {
    displayName: "wlan3",
    ip: "127.0.0.1",
    port: 9001,
  },
]);
const selectedNetworkCard = ref(NetworkCards.value[0]);
const sendEmail = () => {
  document.location = "mailto:alexi.pawelec@vortideck.com";
};

onMounted(() => {});
</script>

<style lang="scss">
@use "../components/Modal/modalTheme.scss" as *;

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.5s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

.layout-scroll-shell {
  --layout-scrollbar-width: 8px;
  scrollbar-gutter: stable;

  &::-webkit-scrollbar {
    display: block;
    width: var(--layout-scrollbar-width);
    height: 12px;
  }
}

.custom-multiselect {
  ::-webkit-scrollbar {
    display: block; /* Chrome, Safari and Opera */
  }

  font-size: 0.85rem;

  // Override vue-multiselect default styles with theme-aware colors
  :deep(.multiselect) {
    background: hsl(var(--background2));
    border: 1px solid hsl(var(--border2));
    border-radius: 0.375rem;
    color: hsl(var(--text));
  }

  :deep(.multiselect__tags) {
    background: hsl(var(--background2));
    border: 1px solid hsl(var(--border2));
    border-radius: 0.375rem;
    padding: 8px 12px;
    min-height: 40px;
  }

  :deep(.multiselect__single) {
    background: transparent;
    color: hsl(var(--text));
    padding: 0;
    margin: 0;
    line-height: 1.5;
  }

  :deep(.multiselect__input) {
    background: transparent;
    color: hsl(var(--text));
    border: none;
    padding: 0;
    margin: 0;
  }

  :deep(.multiselect__input::placeholder) {
    color: hsl(var(--text3));
  }

  :deep(.multiselect__placeholder) {
    color: hsl(var(--text3));
    padding: 0;
    margin: 0;
  }

  :deep(.multiselect__select) {
    background: transparent;
    border-left: 1px solid hsl(var(--border2));
    border-radius: 0 0.375rem 0.375rem 0;
    width: 40px;
    height: 38px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  :deep(.multiselect__select::before) {
    border-color: hsl(var(--text2)) transparent transparent;
    border-style: solid;
    border-width: 5px 5px 0 5px;
    content: "";
    height: 0;
    width: 0;
    position: relative;
    top: 0;
    margin: 0;
    transition: transform 0.2s ease;
    background: transparent;
  }

  // Fix arrow rotation - only rotate the arrow, not background
  :deep(.multiselect--active .multiselect__select::before) {
    transform: rotate(180deg);
  }

  :deep(.multiselect__content-wrapper) {
    background: hsl(var(--background2));
    border: 1px solid hsl(var(--border2));
    border-top: none;
    border-radius: 0 0 0.375rem 0.375rem;
    box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
    max-height: 200px;
    overflow-y: auto;
  }

  :deep(.multiselect__content) {
    background: hsl(var(--background2));
  }

  :deep(.multiselect__option) {
    background: hsl(var(--background2));
    color: hsl(var(--text));
    padding: 8px 12px;
    line-height: 1.5;
    transition: all 0.2s ease;

    &:hover {
      background: hsl(var(--hover));
      color: hsl(var(--text));
    }

    &.multiselect__option--selected {
      background: hsl(var(--primary));
      color: white;
      font-weight: 600;

      &:hover {
        background: hsl(var(--primary));
        color: white;
      }
    }

    &.multiselect__option--highlight {
      background: hsl(var(--background3));
      color: hsl(var(--text));

      &.multiselect__option--selected {
        background: hsl(var(--primary));
        color: white;
      }
    }
  }

  :deep(.multiselect__option--disabled) {
    background: hsl(var(--background2));
    color: hsl(var(--text3));
    opacity: 0.5;
    cursor: not-allowed;
  }

  :deep(.multiselect__spinner) {
    background: hsl(var(--background2));

    &::before,
    &::after {
      border-color: hsl(var(--primary)) transparent transparent;
    }
  }

  // Focus states
  :deep(.multiselect--active) {
    .multiselect__tags {
      border-color: hsl(var(--primary));
      box-shadow: 0 0 0 1px hsl(var(--primary) / 0.2);
    }
  }

  // Disabled state
  :deep(.multiselect--disabled) {
    .multiselect__tags {
      background: hsl(var(--background));
      opacity: 0.6;
      cursor: not-allowed;
    }

    .multiselect__single,
    .multiselect__input {
      color: hsl(var(--text3));
    }
  }
}
</style>
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
