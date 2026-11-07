<template lang="pug">
BaseModal(:is-open="isOpen" @close="$emit('close')")
  div(class="vd-modal-card relative w-full max-w-[30rem]")
        // Header
        div(class="flex items-center gap-x-3 p-6 border-b border-themeBorder2")
          MaterialSymbolsLightScreenshotMonitorOutline(class="w-7 h-7 text-themeText")

          h3(class="text-2xl font-semibold text-themeText") {{$t('modals.create_deck.title')}}
          button(@click="$emit('close')" class="vd-modal-close-btn ml-auto")
            span(class="sr-only") {{$t('modals.create_deck.close')}}
            svg(class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")

        // Body
        div(class="p-6 space-y-6")
          div(class="flex flex-col gap-y-6")
            div(class="space-y-2")
              label(for="deck-name" class="block text-sm font-medium text-themeText") {{$t('modals.create_deck.deck_name')}}
              div(class="flex items-center gap-x-3")
                input#deck-name(
                  v-model="newDeck.name" 
                  placeholder="Enter deck name"
                  class="vd-modal-input flex-1 px-3 py-2"
                  @change="validateDeck"
                )

            div(class="space-y-2")
              label(for="game-select" class="block text-sm font-medium text-themeText") {{$t('modals.create_deck.auto_activate_on_game')}}
              select#game-select(
                v-model="newDeck.game" 
                class="vd-modal-select w-full px-3 py-2"
                @change="validateDeck"
              )
                option(value="" disabled selected) {{$t('modals.create_deck.select_game')}}

                option(value="valorant") Valorant
                option(value="csgo") CS:GO
                option(value="apex") Apex Legends
                option(value="fortnite") Fortnite

            div(class="space-y-2")
              label(class="block text-sm font-medium text-themeText") {{$t('modals.create_deck.deck_settings')}}
              div(class="flex items-center gap-x-2 text-sm text-themeText")
                input#default-deck(
                  type="checkbox" 
                  v-model="newDeck.isDefault" 
                  class="h-4 w-4 text-themeText focus:ring-2 focus:ring-blue-500/50 border-themeBorder2 rounded bg-themeBackground3"
                )
                label(for="default-deck") {{$t('modals.create_deck.set_as_active_deck')}}

        // Footer
        div(class="vd-modal-footer px-6 py-4 flex justify-between items-center rounded-b-lg")
          span(class="text-sm text-themeText") {{isValidDeck ? $t('modals.create_deck.ready_to_create') : $t('modals.create_deck.please_fill_required_fields')}}
          button(
            @click="createDeck" 
            :disabled="!isValidDeck"
            class="vd-modal-primary-btn px-4 py-2 disabled:opacity-50 disabled:cursor-not-allowed"
          ) {{$t('modals.create_deck.create_deck')}}
</template>

<script setup lang="ts">
import BaseModal from "./BaseModal.vue";
import MaterialSymbolsLightScreenshotMonitorOutline from "~icons/material-symbols-light/screenshot-monitor-outline";

import { ref, computed } from "vue";
import { useDeckStore } from "@/stores/Deck";

const deckStore = useDeckStore();

const props = defineProps({
  isOpen: {
    type: Boolean,
    required: true,
  },
});

const emit = defineEmits(["close"]);

const newDeck = ref({
  name: "",
  game: "",
});

const isValidDeck = computed(() => {
  return newDeck.value.name.trim() !== "";
});

const validateDeck = () => {
  // Additional validation logic if needed
};

const createDeck = async () => {
  if (isValidDeck.value) {
    const success = await deckStore.createDeck(newDeck.value.name, newDeck.value.game);
    if (success) {
      emit("close");
    }
  }
};
</script>

<style lang="scss">
@use "./modalTheme.scss" as *;
</style>
