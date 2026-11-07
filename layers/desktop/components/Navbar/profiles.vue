<template lang="pug">
div.profile-switcher(
  class="relative z-[110] select-none"
  :class="(isProfileSearch || isProfileSearchClicked) ? 'profile-switcher--open' : ''"
)
  div.profile-trigger(class="flex items-center gap-x-3 w-full")
    MaterialSymbolsLightScreenshotMonitorOutline(class="profile-trigger-icon")
    div(class="truncate")
      span(class="profile-prefix") {{$t('navigation.profiles.active')}}:
      span(class="profile-name") {{ deckStore.getActiveDeck?.name || "No active deck" }}
    HugeiconsArrowDown01(
      class="profile-chevron"
      :class="(isProfileSearch || isProfileSearchClicked) ? 'profile-chevron--open' : ''"
    )

  div#navbarProfiles(v-if="isProfileSearch || isProfileSearchClicked" @click.stop class="profile-dropdown")
    div(class="profile-search-wrap")
      PixelarticonsSearch(class="profile-search-icon")
      input(
        v-model="searchPrompt"
        placeholder="Search decks"
        class="profile-search-input"
      )

    div(class="profile-list")
      div(
        v-for="deck in decksComputed"
        :key="deck.id"
        class="profile-row"
        @click="deckStore.setActiveDeck(deck.id)"
      )
        MaterialSymbolsLightScreenshotMonitorOutline(class="profile-row-icon")
        p(class="profile-row-name") {{ deck.name }}
        button(class="profile-delete-btn" @click.stop="deckStore.removeDeck(deck.id)")
          IcOutlineDelete(class="w-4.5 h-4.5")

      p(v-if="decksComputed.length === 0" class="profile-empty") No decks found

    div(class="profile-footer")
      button(class="profile-footer-btn" @click.stop="isManageDecks = !isManageDecks")
        span {{$t('navigation.profiles.manage_decks')}}

      button(class="profile-footer-btn profile-footer-btn--primary" @click.stop="isCreatingDeck = !isCreatingDeck")
        span {{$t('navigation.profiles.new_deck')}}

  ModalManageDecks(:isOpen="isManageDecks" @close="isManageDecks = false")
  ModalCreateDeck(:isOpen="isCreatingDeck" @close="isCreatingDeck = false")
</template>

<script setup lang="ts">
import IcOutlineDelete from "~icons/ic/outline-delete?width=24px&height=24px";
import PixelarticonsSearch from "~icons/pixelarticons/search?width=24px&height=24px";
import HugeiconsArrowDown01 from "~icons/hugeicons/arrow-down-01?width=24px&height=24px";
import MaterialSymbolsLightScreenshotMonitorOutline from "~icons/material-symbols-light/screenshot-monitor-outline?width=24px&height=24px";
import { useDeckStore } from "@/stores/Deck";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

const deckStore = useDeckStore();
const isManageDecks = ref(false);
const isCreatingDeck = ref(false);
const searchPrompt = ref("");
const decksComputed = computed(() => {
  // also sort by prompt
  let decks = deckStore.getAllDecks.filter(
    (deck) => deck.id !== deckStore.getActiveDeck?.id
  );
  if (searchPrompt.value) {
    decks = decks.filter((deck) =>
      deck.name.toLowerCase().includes(searchPrompt.value.toLowerCase())
    );
  }
  return decks;
});

defineProps({
  isProfileSearch: Boolean,
  isProfileSearchClicked: Boolean,
});

await listen("update_active_deck", async () => {
  const decks = await invoke("get_all_decks");
  if (Array.isArray(decks)) {
    deckStore.setDecks(decks);
  }
});
</script>

<style lang="scss" scoped>
.profile-switcher {
  width: 20rem;
  min-width: 20rem;
  max-width: 20rem;
}

.profile-switcher--open::after {
  content: "";
  position: absolute;
  top: 100%;
  left: 0;
  width: 100%;
  height: 0.45rem;
}

.profile-trigger {
  height: 2.55rem;
  border-radius: 0.62rem;
  border: 1px solid hsl(var(--border2));
  background: hsl(var(--background2));
  padding: 0 0.72rem;
  cursor: pointer;
  transition: border-color 0.2s ease, box-shadow 0.2s ease, background 0.2s ease;
}

.profile-trigger:hover {
  border-color: hsl(var(--border2) / 0.8);
  background: hsl(var(--hover));
}

.profile-trigger-icon {
  width: 1.1rem;
  height: 1.1rem;
  color: hsl(var(--text2));
  flex-shrink: 0;
}

.profile-prefix {
  font-size: 0.74rem;
  color: hsl(var(--text3));
  margin-right: 0.25rem;
}

.profile-name {
  font-size: 0.93rem;
  font-weight: 700;
  color: hsl(var(--text));
}

.profile-chevron {
  width: 1rem;
  height: 1rem;
  color: hsl(var(--text3));
  margin-left: auto;
  flex-shrink: 0;
  transition: transform 0.2s ease;
}

.profile-chevron--open {
  transform: rotate(180deg);
}

.profile-dropdown {
  position: absolute;
  top: calc(100% + 0.22rem);
  left: 0;
  width: 100%;
  border-radius: 0.62rem;
  border: 1px solid hsl(var(--border2));
  background: hsl(var(--background2));
  box-shadow: 0 14px 28px hsl(var(--text) / 0.14);
  padding: 0.65rem 0.55rem 0.5rem;
}

.profile-search-wrap {
  height: 2.55rem;
  border: 1px solid hsl(var(--border2));
  border-radius: 0.55rem;
  background: hsl(var(--background));
  display: flex;
  align-items: center;
  gap: 0.55rem;
  padding: 0 0.62rem;
}

.profile-search-icon {
  width: 1.05rem;
  height: 1.05rem;
  color: hsl(var(--text3));
}

.profile-search-input {
  width: 100%;
  font-size: 0.85rem;
  color: hsl(var(--text));
  background: transparent;
  outline: none;
}

.profile-list {
  margin-top: 0.6rem;
  max-height: 11.2rem;
  overflow-y: auto;
  padding-right: 0.15rem;
}

.profile-row {
  display: flex;
  align-items: center;
  gap: 0.58rem;
  border-radius: 0.52rem;
  padding: 0.48rem 0.52rem;
  transition: background 0.2s ease;
  cursor: pointer;
}

.profile-row:hover {
  background: hsl(var(--hover));
}

.profile-row-icon {
  width: 0.95rem;
  height: 0.95rem;
  color: hsl(var(--text2));
  flex-shrink: 0;
}

.profile-row-name {
  font-size: 0.82rem;
  color: hsl(var(--text));
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.profile-delete-btn {
  margin-left: auto;
  width: 1.9rem;
  height: 1.9rem;
  border-radius: 0.42rem;
  border: none;
  background: transparent;
  color: hsl(var(--text3));
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transition: background 0.2s ease, color 0.2s ease;
}

.profile-delete-btn:hover {
  background: hsl(var(--error) / 0.1);
  color: hsl(var(--error));
}

.profile-empty {
  padding: 0.7rem 0.55rem;
  font-size: 0.8rem;
  color: hsl(var(--text3));
}

.profile-footer {
  display: flex;
  align-items: center;
  gap: 0.55rem;
  border-top: 1px solid hsl(var(--border2));
  padding: 0.68rem 0.2rem 0.1rem;
  margin-top: 0.35rem;
}

.profile-footer-btn {
  flex: 1 1 0;
  height: 2.45rem;
  border-radius: 0.5rem;
  border: 1px solid hsl(var(--border2));
  background: hsl(var(--background));
  color: hsl(var(--text));
  font-size: 0.76rem;
  font-weight: 700;
  transition: background 0.2s ease, color 0.2s ease, border-color 0.2s ease;
}

.profile-footer-btn:hover {
  background: hsl(var(--hover));
  border-color: hsl(var(--border2) / 0.8);
}

.profile-footer-btn--primary {
  background: hsl(var(--text));
  border-color: hsl(var(--text));
  color: hsl(var(--background));
}

.profile-footer-btn--primary:hover {
  background: hsl(var(--text) / 0.85);
  border-color: hsl(var(--text) / 0.85);
}

.profile-list::-webkit-scrollbar {
  width: 0.36rem;
}

.profile-list::-webkit-scrollbar-thumb {
  background: hsl(var(--border2));
  border-radius: 999px;
}
</style>
