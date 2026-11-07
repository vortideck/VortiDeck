<template lang="pug">
BaseModal(:is-open="isOpen" shell-class="scrollbar-visible" @close="handleClose")
  div(class="vd-modal-card relative w-full max-w-[38rem] h-[38rem] max-h-[88vh] flex flex-col overflow-hidden")
    //- Header
    div(class="flex items-center justify-between px-6 pt-5 pb-4 border-b border-themeBorder2/80 bg-themeBackground2/40 shrink-0")
      div(class="flex items-center gap-3")
        div(class="w-10 h-10 rounded-xl bg-themeButton/10 border border-themeButton/25 flex items-center justify-center text-themeButton shrink-0 shadow-2xs")
          MaterialSymbolsLightScreenshotMonitorOutline(class="w-6 h-6")
        div(class="flex flex-col")
          div(class="flex items-center gap-2")
            h3(class="text-lg font-bold text-themeText tracking-tight") {{$t('modals.manage_decks.title')}}
            span(class="text-[11px] font-bold px-2 py-0.5 rounded-full bg-themeBackground3 text-themeText2 border border-themeBorder2/60") {{ deckStore.getAllDecks.length }}
          p(class="text-xs text-themeText3") {{$t('modals.manage_decks.subtitle')}}
      button(@click="handleClose" class="vd-modal-close-btn" :title="$t('modals.manage_decks.close')")
        span(class="sr-only") {{$t('modals.manage_decks.close')}}
        svg(class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")

    //- Toolbar (Search & Quick Add Toggle)
    div(class="px-6 pt-4 pb-2 flex items-center gap-2 shrink-0")
      div(class="relative flex-1 flex items-center")
        PixelarticonsSearch(class="absolute left-3 w-4 h-4 text-themeText3 pointer-events-none")
        input(
          v-model="searchQuery"
          :placeholder="$t('modals.manage_decks.search_placeholder')"
          class="vd-modal-input w-full pl-9 pr-8 py-2 text-xs"
        )
        button(
          v-if="searchQuery"
          @click="searchQuery = ''"
          class="absolute right-2.5 p-0.5 text-themeText3 hover:text-themeText rounded transition-colors cursor-pointer"
        )
          BiX(class="w-4 h-4")

      button(
        type="button"
        @click="toggleCreateDrawer"
        :class="isCreatingInline ? 'bg-themeButton text-white border-themeButton' : 'bg-themeBackground3 text-themeText hover:bg-themeHover border-themeBorder2/80'"
        class="flex items-center gap-1.5 px-3 py-2 text-xs font-bold rounded-xl border transition-all cursor-pointer shrink-0 shadow-2xs"
      )
        Fa6SolidPlus(class="w-3 h-3")
        span {{$t('modals.manage_decks.new_deck')}}

    //- Inline New Deck Creator Drawer
    Transition(name="fade")
      div(v-if="isCreatingInline" class="px-6 py-2 shrink-0")
        div(class="p-3 rounded-xl bg-themeBackground3/70 border border-themeButton/40 shadow-xs space-y-2.5")
          div(class="flex items-center justify-between")
            span(class="text-xs font-bold text-themeText") {{$t('modals.manage_decks.create_deck')}}
            span(class="text-[10px] text-themeText3 font-mono") Press Enter to create
          
          div(class="flex items-center gap-2")
            input(
              ref="newDeckInputRef"
              v-model="newDeckName"
              @keyup.enter="handleCreateDeck"
              @keyup.esc="cancelCreateInline"
              :placeholder="$t('modals.manage_decks.deck_name_placeholder')"
              class="vd-modal-input flex-1 px-3 py-1.5 text-xs font-medium"
            )

            button(
              type="button"
              @click="handleCreateDeck"
              :disabled="!newDeckName.trim() || isSubmitting"
              class="vd-modal-primary-btn px-3.5 py-1.5 text-xs font-bold disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer shrink-0"
            )
              span(v-if="!isSubmitting") {{$t('modals.manage_decks.create_deck')}}
              span(v-else) ...
            button(
              type="button"
              @click="cancelCreateInline"
              class="px-2.5 py-1.5 text-xs text-themeText3 hover:text-themeText rounded-lg hover:bg-themeBackground transition-colors cursor-pointer shrink-0"
            ) {{$t('modals.manage_decks.cancel')}}

    //- Body / Deck List
    div(class="p-6 pt-2 flex-1 min-h-0 overflow-y-auto")
      div(class="flex flex-col gap-y-2.5")
        //- Deck Card
        div(
          v-for="deck in filteredDecks"
          :key="deck.id"
          :class="isDeckActive(deck.id) ? 'bg-themeButton/[0.05] border-themeButton/70 ring-1 ring-themeButton/30 shadow-xs' : 'bg-themeBackground2/60 hover:bg-themeBackground2 border-themeBorder2/70 hover:border-themeBorder2 shadow-2xs'"
          class="group relative flex flex-col p-3 rounded-xl border transition-all duration-200 gap-y-2"
        )
          //- Top Row: Icon, Name & Actions
          div(class="flex items-center justify-between w-full")
            div(class="flex items-center gap-3 min-w-0 flex-1")
              div(
                :class="isDeckActive(deck.id) ? 'bg-themeButton/15 text-themeButton border-themeButton/30' : 'bg-themeBackground3 text-themeText2 border-themeBorder2/60 group-hover:text-themeText'"
                class="w-9 h-9 rounded-xl flex items-center justify-center border shrink-0 transition-colors"
              )
                MaterialSymbolsLightScreenshotMonitorOutline(class="w-5 h-5")

              div(class="flex flex-col min-w-0 flex-1 mr-2")
                //- Name row (view or inline edit)
                div(v-if="editingDeckId === deck.id" class="flex items-center gap-1.5 w-full py-0.5")
                  input(
                    :ref="setEditInput"
                    v-model="editDeckName"
                    @keyup.enter="saveDeckName(deck)"
                    @keyup.esc="cancelEditing"
                    class="vd-modal-input text-xs font-semibold px-2.5 py-1 flex-1 min-w-0"
                  )
                  button(
                    type="button"
                    @click="saveDeckName(deck)"
                    class="p-1 rounded-lg bg-themeSuccess/20 text-themeSuccess hover:bg-themeSuccess/30 border border-themeSuccess/40 transition-colors cursor-pointer shrink-0"
                    :title="$t('modals.manage_decks.save')"
                  )
                    BiCheck(class="w-4 h-4")
                  button(
                    type="button"
                    @click="cancelEditing"
                    class="p-1 rounded-lg bg-themeBackground3 text-themeText3 hover:text-themeText border border-themeBorder2/80 transition-colors cursor-pointer shrink-0"
                    :title="$t('modals.manage_decks.cancel')"
                  )
                    BiX(class="w-4 h-4")

                div(v-else class="flex items-center gap-2")
                  p(
                    @dblclick="startEditing(deck)"
                    class="font-bold text-xs text-themeText truncate tracking-tight select-none cursor-pointer hover:text-themeButton transition-colors"
                    :title="deck.name"
                  ) {{ deck.name }}

            //- Action Buttons
            div(class="flex items-center gap-1.5 shrink-0 ml-2")
              //- Delete Confirmation State
              template(v-if="deletingDeckId === deck.id")
                div(class="flex items-center gap-1.5 bg-themeError/10 border border-themeError/30 px-2 py-1 rounded-lg")
                  span(class="text-[11px] font-semibold text-themeError") {{$t('modals.manage_decks.confirm_delete')}}
                  button(
                    type="button"
                    @click="confirmDelete(deck.id)"
                    class="px-2 py-0.5 text-[11px] font-bold rounded bg-themeError text-white hover:brightness-110 transition-all cursor-pointer"
                  ) {{$t('modals.manage_decks.delete')}}
                  button(
                    type="button"
                    @click="deletingDeckId = null"
                    class="px-1.5 py-0.5 text-[11px] font-medium rounded text-themeText2 hover:text-themeText transition-colors cursor-pointer"
                  ) {{$t('modals.manage_decks.cancel')}}

              //- Normal Actions State
              template(v-else)
                //- Set Active Button (if not currently active)
                button(
                  v-if="!isDeckActive(deck.id)"
                  type="button"
                  @click="setActive(deck.id)"
                  class="px-2.5 py-1 text-[11px] font-bold rounded-lg border border-themeBorder2/80 bg-themeBackground hover:bg-themeButton hover:text-white hover:border-themeButton text-themeText2 transition-all cursor-pointer shadow-2xs"
                ) {{$t('modals.manage_decks.set_active')}}

                //- Rename Button
                button(
                  v-if="editingDeckId !== deck.id"
                  type="button"
                  @click="startEditing(deck)"
                  class="p-1.5 rounded-lg text-themeText3 hover:text-themeText hover:bg-themeBackground3 border border-transparent hover:border-themeBorder2/60 transition-all cursor-pointer"
                  :title="$t('modals.manage_decks.rename')"
                )
                  BiPencil(class="w-3.5 h-3.5")

                //- Delete Button
                button(
                  type="button"
                  @click="promptDelete(deck.id)"
                  :disabled="deckStore.getAllDecks.length <= 1"
                  :class="deckStore.getAllDecks.length <= 1 ? 'opacity-30 cursor-not-allowed' : 'text-themeText3 hover:text-themeError hover:bg-themeError/10 border-transparent hover:border-themeError/20 cursor-pointer'"
                  class="p-1.5 rounded-lg border transition-all"
                  :title="deckStore.getAllDecks.length <= 1 ? $t('modals.manage_decks.cannot_delete_last') : $t('modals.manage_decks.delete')"
                )
                  IcOutlineDelete(class="w-4 h-4")

          //- Bottom Sub-Row: Badges & App Auto-activation
          div(class="flex items-center justify-between gap-2 pt-1.5 border-t border-themeBorder2/40 flex-wrap")
            //- Left badges
            div(class="flex items-center gap-1.5 flex-wrap")
              //- Active Badge
              span(
                v-if="isDeckActive(deck.id)"
                class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-bold bg-themeConnectedBg text-themeConnected border border-themeConnected/20"
              )
                span(class="w-1.5 h-1.5 rounded-full bg-themeConnected animate-pulse")
                span {{$t('modals.manage_decks.active')}}

              //- Buttons count badge
              span(class="inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-[10px] font-medium bg-themeBackground3/80 text-themeText3 border border-themeBorder2/40")
                span {{ getDeckButtonsCount(deck) }} {{$t('modals.manage_decks.buttons')}}

              //- Grid size badge if present
              span(
                v-if="deck.settings?.grid"
                class="inline-flex items-center px-1.5 py-0.5 rounded-md text-[10px] font-mono text-themeText3 bg-themeBackground3/60 border border-themeBorder2/30"
              ) {{ deck.settings.grid }}

            //- Right: App/Window Auto-activation (Grayed out / Coming Soon)
            div(class="flex items-center gap-1.5 ml-auto opacity-50 select-none cursor-not-allowed" :title="$t('modals.manage_decks.auto_activate_coming_soon')")
              svg(class="w-3.5 h-3.5 text-themeText3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.75")
                path(stroke-linecap="round" stroke-linejoin="round" d="M9 17V7m0 10a2 2 0 01-2 2H5a2 2 0 01-2-2V7a2 2 0 012-2h2a2 2 0 012 2m0 10a2 2 0 002 2h2a2 2 0 002-2M9 7a2 2 0 012-2h2a2 2 0 012 2m0 10V7m0 10a2 2 0 002 2h2a2 2 0 002-2V7a2 2 0 00-2-2h-2a2 2 0 00-2 2")
              span(class="text-[10px] text-themeText3 font-medium hidden sm:inline-block") {{$t('modals.manage_decks.auto_activate_on')}}
              span(class="text-[10px] font-semibold px-2 py-0.5 rounded-lg bg-themeBackground3/60 text-themeText3 border border-themeBorder2/40 cursor-not-allowed") {{$t('modals.manage_decks.auto_activate_coming_soon')}}

        //- Empty State
        div(v-if="filteredDecks.length === 0" class="flex flex-col items-center justify-center py-12 text-center h-full")
          div(class="w-12 h-12 rounded-2xl bg-themeBackground3 flex items-center justify-center text-themeText3 mb-2.5 border border-themeBorder2/60")
            MaterialSymbolsLightScreenshotMonitorOutline(class="w-6 h-6")
          p(class="text-xs font-bold text-themeText mb-1") {{ searchQuery ? $t('modals.manage_decks.no_decks_found') : $t('modals.manage_decks.no_decks_yet') }}
          p(v-if="searchQuery" class="text-[11px] text-themeText3") Try adjusting your search query
          button(
            v-if="!searchQuery"
            type="button"
            @click="toggleCreateDrawer"
            class="vd-modal-primary-btn mt-3 px-3.5 py-1.5 text-xs font-bold cursor-pointer"
          ) {{$t('modals.manage_decks.create_deck')}}

    //- Footer
    div(class="vd-modal-footer px-6 py-3.5 flex justify-between items-center rounded-b-2xl border-t border-themeBorder2/70 bg-themeBackground2/70 shrink-0")
      div(class="flex items-center gap-2 text-xs text-themeText3")
        span {{ deckStore.getAllDecks.length }} {{$t('modals.manage_decks.decks_total')}}
        span(v-if="deckStore.getActiveDeck") •
        span(v-if="deckStore.getActiveDeck" class="text-themeText2 truncate max-w-[14rem]")
          | {{$t('modals.manage_decks.active_deck')}}: 
          strong(class="text-themeText font-semibold") {{ deckStore.getActiveDeck.name }}
      button(
        type="button"
        @click="handleClose"
        class="px-4 py-1.5 text-xs font-bold rounded-xl border border-themeBorder2/80 bg-themeBackground hover:bg-themeHover text-themeText transition-colors cursor-pointer shadow-2xs"
      ) {{$t('modals.manage_decks.close')}}
</template>

<script setup lang="ts">
import BaseModal from "./BaseModal.vue";
import { ref, computed, nextTick, watch, onMounted } from "vue";
import MaterialSymbolsLightScreenshotMonitorOutline from "~icons/material-symbols-light/screenshot-monitor-outline";
import IcOutlineDelete from "~icons/ic/outline-delete";
import PixelarticonsSearch from "~icons/pixelarticons/search";
import Fa6SolidPlus from "~icons/fa6-solid/plus";
import BiPencil from "~icons/bi/pencil";
import BiCheck from "~icons/bi/check";
import BiX from "~icons/bi/x";
import { useDeckStore } from "@/stores/Deck";
import { invoke } from "@tauri-apps/api/core";
import type { Deck } from "@/types/deck";

const props = defineProps<{
  isOpen: boolean;
}>();

const emit = defineEmits(["close"]);
const deckStore = useDeckStore();

// Running visible windows fetched dynamically via Tauri
const runningWindows = ref<{ id: string; name: string }[]>([]);

const loadRunningWindows = async () => {
  try {
    const windows: any[] = await invoke("get_visible_windows");
    if (Array.isArray(windows)) {
      const seen = new Set<string>();
      const list: { id: string; name: string }[] = [];
      for (const w of windows) {
        if (!w.process_name || w.process_name === "Unknown") continue;
        const key = w.process_name.toLowerCase();
        if (!seen.has(key)) {
          seen.add(key);
          const label = w.title && w.title !== w.process_name
            ? `${w.process_name} — ${w.title.slice(0, 30)}${w.title.length > 30 ? '...' : ''}`
            : w.process_name;
          list.push({ id: w.process_name, name: label });
        }
      }
      runningWindows.value = list;
    }
  } catch (e) {
    // web fallback or ignored
  }
};

watch(
  () => props.isOpen,
  (open) => {
    if (open) {
      loadRunningWindows();
    }
  },
  { immediate: true }
);

onMounted(() => {
  if (props.isOpen) {
    loadRunningWindows();
  }
});

// Search State
const searchQuery = ref("");

// Inline Create Deck State
const isCreatingInline = ref(false);
const newDeckName = ref("");
const newDeckGame = ref("");
const isSubmitting = ref(false);
const newDeckInputRef = ref<HTMLInputElement | null>(null);

// Editing State
const editingDeckId = ref<number | null>(null);
const editDeckName = ref("");
let currentEditInputEl: HTMLInputElement | null = null;

const setEditInput = (el: any) => {
  if (el && typeof el.focus === "function") {
    currentEditInputEl = el;
    el.focus();
    el.select?.();
  }
};

// Deleting State (inline confirmation)
const deletingDeckId = ref<number | null>(null);

// Filtered Decks list (searches deck name, bound process, or window title)
const filteredDecks = computed(() => {
  const all = deckStore.getAllDecks || [];
  if (!searchQuery.value.trim()) return all;
  const q = searchQuery.value.toLowerCase().trim();
  return all.filter((deck) => {
    if (deck.name && deck.name.toLowerCase().includes(q)) return true;
    if (deck.game) {
      const g = deck.game.toLowerCase();
      if (g.includes(q)) return true;
      const win = runningWindows.value.find((w) => w.id.toLowerCase() === g || w.name.toLowerCase().includes(q));
      if (win) return true;
    }
    return false;
  });
});

const isDeckActive = (deckId: number) => {
  return deckStore.getActiveDeck?.id === deckId;
};

const getDeckButtonsCount = (deck: Deck) => {
  if (deck.buttons && Array.isArray(deck.buttons)) {
    return deck.buttons.length;
  }
  return 0;
};

const setActive = async (deckId: number) => {
  await deckStore.setActiveDeck(deckId);
};

const onDeckGameChange = async (deck: Deck, gameId: string) => {
  deck.game = gameId;
  await deckStore.DeepUpdateDeckById(deck);
};

const handleGameSelect = (deck: Deck, event: Event) => {
  const target = event.target as HTMLSelectElement | null;
  if (target) {
    onDeckGameChange(deck, target.value);
  }
};

const toggleCreateDrawer = async () => {
  isCreatingInline.value = !isCreatingInline.value;
  if (isCreatingInline.value) {
    newDeckName.value = "";
    newDeckGame.value = "";
    await nextTick();
    if (newDeckInputRef.value && typeof newDeckInputRef.value.focus === "function") {
      newDeckInputRef.value.focus();
    }
  }
};

const cancelCreateInline = () => {
  isCreatingInline.value = false;
  newDeckName.value = "";
  newDeckGame.value = "";
};

const handleCreateDeck = async () => {
  const trimmed = newDeckName.value.trim();
  if (!trimmed || isSubmitting.value) return;

  isSubmitting.value = true;
  try {
    const success = await deckStore.createDeck(trimmed, newDeckGame.value);
    if (success) {
      newDeckName.value = "";
      newDeckGame.value = "";
      isCreatingInline.value = false;
    }
  } finally {
    isSubmitting.value = false;
  }
};

const startEditing = async (deck: Deck) => {
  editingDeckId.value = deck.id;
  editDeckName.value = deck.name;
  deletingDeckId.value = null;
  await nextTick();
  if (currentEditInputEl && typeof currentEditInputEl.focus === "function") {
    currentEditInputEl.focus();
    currentEditInputEl.select?.();
  }
};

const cancelEditing = () => {
  editingDeckId.value = null;
  editDeckName.value = "";
};

const saveDeckName = async (deck: Deck) => {
  const trimmed = editDeckName.value.trim();
  if (trimmed && trimmed !== deck.name) {
    const updated = { ...deck, name: trimmed };
    await deckStore.DeepUpdateDeckById(updated);
  }
  editingDeckId.value = null;
};

const promptDelete = (deckId: number) => {
  if (deckStore.getAllDecks.length <= 1) return;
  deletingDeckId.value = deckId;
  editingDeckId.value = null;
};

const confirmDelete = async (deckId: number) => {
  await deckStore.removeDeck(deckId);
  deletingDeckId.value = null;
};

const handleClose = () => {
  isCreatingInline.value = false;
  editingDeckId.value = null;
  deletingDeckId.value = null;
  searchQuery.value = "";
  newDeckName.value = "";
  newDeckGame.value = "";
  emit("close");
};
</script>

<style lang="scss">
@use "./modalTheme.scss" as *;
</style>


