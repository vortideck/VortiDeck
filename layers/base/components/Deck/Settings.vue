<template lang="pug">
  transition(name="slide" @enter="onEnter" @leave="onLeave" :css="false")
    div(v-if="isDeckSettingsOpen" 
        class="absolute top-0 left-0 w-full h-full z-[45] flex flex-col items-start settings-panel py-2"
        :class="isLandscape ? 'landscape-mode' : 'portrait-mode'"
        :style="`${deckStore.getActiveTheme.backgroundCss}`")
      
      .settings-header.w-full.flex.justify-between.items-center.px-6.py-4.border-b.border-opacity-30
        h2.text-white.font-bold.text-xl.text-shadow-sm Deck Settings
        //- button.settings-close-btn(@click="$emit('closeDeckSettings')")
        //-   svg.w-6.h-6(xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor")
        //-     path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")
      
      .settings-body.w-full.flex-1.overflow-auto.px-6.py-4
        .setting-group.mb-6.flex.flex-col.gap-y-4
          h3.text-white.text-lg.mb-3.text-shadow-sm Layout Options
          
          .setting-item.flex.gap-x-4.items-center.p-3.rounded-lg.hover-bg-change
            .toggle-wrapper.relative.flex
              input.toggle-checkbox.opacity-0.absolute.h-0.w-0(
                type="checkbox"
                v-model="deckStore.getActiveDeck.settings.maximize_button_space"
                id="fillPageCheckbox"
              )
              label.toggle-label.bg-gray-300.cursor-pointer.relative.rounded-full.transition-colors.duration-200.ease-in-out.inline-block.h-6.w-12(
                for="fillPageCheckbox"
                :class="{'bg-green-400': deckStore.getActiveDeck.settings.maximize_button_space}"
              )
                span.toggle-button.absolute.top-0.left-0.bg-white.border-2.border-gray-300.rounded-full.transition-transform.duration-200.ease-in-out.h-6.w-6(
                  :class="{'transform translate-x-6 border-green-400': deckStore.getActiveDeck.settings.maximize_button_space}"
                )
                
            .setting-info
              label.text-white.font-medium(for="fillPageCheckbox") Maximize Button Space
              p.text-gray-200.text-sm.mt-1 Expands buttons to fill the available space on screen


          .setting-item.flex.gap-x-4.items-center.p-3.rounded-lg.hover-bg-change
            .toggle-wrapper.relative.flex
              input.toggle-checkbox.opacity-0.absolute.h-0.w-0(
                type="checkbox"
                v-model="deckStore.isTipIconInDeck"
                id="tipIconInDeckCheckbox"
              )
              label.toggle-label.bg-gray-300.cursor-pointer.relative.rounded-full.transition-colors.duration-200.ease-in-out.inline-block.h-6.w-12(
                for="tipIconInDeckCheckbox"
                :class="{'bg-green-400': deckStore.getIsTipIconInDeck}"
              )
                span.toggle-button.absolute.top-0.left-0.bg-white.border-2.border-gray-300.rounded-full.transition-transform.duration-200.ease-in-out.h-6.w-6(
                  :class="{'transform translate-x-6 border-green-400': deckStore.getIsTipIconInDeck}"
                )
                
            .setting-info
              label.text-white.font-medium(for="tipIconInDeckCheckbox") Tip Icon in Deck
              p.text-gray-200.text-sm.mt-1 Shows a tip icon in the deck
  </template>

<script setup lang="ts">
import { useDeckStore } from "@base/stores/Deck";

const deckStore = useDeckStore();

const props = defineProps({
  isDeckSettingsOpen: {
    type: Boolean,
    required: true,
  },
  isLandscape: {
    type: Boolean,
    required: true,
  },
});

const emit = defineEmits(["closeDeckSettings"]);

// Transition handlers
const onEnter = (el: HTMLElement, done: () => void) => {
  el.style.transform = "translateX(-100%)";
  el.style.opacity = "0";

  // Force reflow
  el.offsetHeight;

  // Add transition
  el.style.transition = "all 0.3s ease-out";

  // Start animation
  requestAnimationFrame(() => {
    el.style.transform = "translateX(0)";
    el.style.opacity = "1";

    el.addEventListener(
      "transitionend",
      () => {
        done();
      },
      { once: true }
    );
  });
};

const onLeave = (el: HTMLElement, done: () => void) => {
  el.style.transform = "translateX(0)";
  el.style.opacity = "1";

  // Force reflow
  el.offsetHeight;

  // Add transition
  el.style.transition = "all 0.3s ease-in";

  // Start animation
  requestAnimationFrame(() => {
    el.style.transform = "translateX(-100%)";
    el.style.opacity = "0";

    el.addEventListener(
      "transitionend",
      () => {
        done();
      },
      { once: true }
    );
  });
};
</script>

<style lang="scss">
.settings-panel {
  color: white;

  &.landscape-mode {
    padding-inline: 12vh;
  }

  &.portrait-mode {
    padding: 9vh 0 0 0;
  }
}

.settings-header {
  border-color: rgba(255, 255, 255, 0.2);
}

.settings-close-btn {
  color: #f1c40f;
  height: 36px;
  width: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  background: rgba(39, 174, 96, 0.2);
  border: 1px solid #27ae60;
  transition: all 0.2s;

  &:hover {
    background: rgba(39, 174, 96, 0.4);
    box-shadow: 0 0 10px rgba(46, 204, 113, 0.5);
  }
}

.setting-item {
  background: rgba(19, 78, 94, 0.3);
  border: 1px solid rgba(39, 174, 96, 0.3);
  transition: all 0.2s ease;

  &:hover {
    background: rgba(19, 78, 94, 0.5);
    box-shadow: 0 0 10px rgba(46, 204, 113, 0.2);
  }
}

.toggle-label {
  &::after {
    content: "";
    position: absolute;
    top: 2px;
    left: 2px;
    width: 20px;
    height: 20px;
    background-color: white;
    border-radius: 50%;
    transition: 0.3s;
  }
}

.toggle-checkbox:checked + .toggle-label {
  background-color: #27ae60;

  &::after {
    left: calc(100% - 2px);
    transform: translateX(-100%);
  }
}

.text-shadow-sm {
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
}

.hover-bg-change {
  &:hover {
    background: rgba(39, 174, 96, 0.2);
  }
}
</style>
