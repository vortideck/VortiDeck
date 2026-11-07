<template lang="pug">
div(v-if="recentDecks.length > 0")
  div(class="flex items-center justify-between mb-3")
    div(class="flex items-center gap-2")
      svg(class="w-5 h-5 text-themePrimary" fill="none" viewBox="0 0 24 24" stroke="currentColor")
        path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z")
      h2(class="font-bold text-lg text-themeText") Recent Decks
    button(v-if="allDecks.length > MAX_RECENT" @click="showAll = !showAll" class="text-xs text-themePrimary font-medium") {{ showAll ? 'Show Less' : 'View All' }}

  div(class="flex gap-3 overflow-x-auto pb-2 -mx-5 px-5 scrollbar-hide")
    button(
      v-for="deck in displayedDecks"
      :key="deck.id"
      @click="openDeck(deck.id)"
      class="flex-shrink-0 w-32 bg-themeBackground3/80 rounded-xl border border-themeBorder p-3 text-left transition-all active:scale-95"
      :class="activeDeckId === deck.id ? 'border-themePrimary/50 bg-themePrimary/5' : ''"
    )
      div(class="w-12 h-12 rounded-xl mb-2.5 flex items-center justify-center overflow-hidden" :class="activeDeckId === deck.id ? 'bg-themePrimary/15 border border-themePrimary/30' : 'bg-themeHint border border-themeBorder'")
        img(v-if="getDeckIcon(deck)" :src="getDeckIcon(deck)" class="w-7 h-7 object-contain")
        svg(v-else class="w-6 h-6 text-themeText3" fill="none" viewBox="0 0 24 24" stroke="currentColor")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z")
      p(class="text-sm font-semibold text-themeText truncate") {{ deck.name }}
      p(class="text-[10px] text-themeText3 uppercase tracking-wider mt-0.5") {{ deck.buttons.length }} actions
</template>

<script setup lang="ts">
import { ref, computed } from "vue";
import { useDeckStore } from "@/stores/Deck";

const router = useRouter();
const deckStore = useDeckStore();

const showAll = ref(false);
const RECENT_DECKS_KEY = "vortideck_recent_decks";
const MAX_RECENT = 5;

const allDecks = computed(() => deckStore.getAllDecks || []);
const activeDeckId = computed(() => deckStore.getActiveDeck?.id ?? null);

const recentDeckIds = computed<number[]>(() => {
  try {
    const stored = localStorage.getItem(RECENT_DECKS_KEY);
    return stored ? JSON.parse(stored) : [];
  } catch {
    return [];
  }
});

const recentDecks = computed(() => {
  const decks = allDecks.value;
  if (decks.length === 0) return [];
  const ids = recentDeckIds.value;
  const sorted = [...decks].sort((a, b) => {
    const ai = ids.indexOf(a.id);
    const bi = ids.indexOf(b.id);
    if (ai !== -1 && bi !== -1) return ai - bi;
    if (ai !== -1) return -1;
    if (bi !== -1) return 1;
    return 0;
  });
  return sorted.slice(0, MAX_RECENT);
});

const displayedDecks = computed(() =>
  showAll.value ? allDecks.value : recentDecks.value
);

const getDeckIcon = (deck: any): string | undefined => {
  const btn = deck.buttons?.find((b: any) => b.icon);
  return btn?.icon || undefined;
};

const trackRecentDeck = (deckId: number) => {
  try {
    const ids = recentDeckIds.value.filter((id: number) => id !== deckId);
    ids.unshift(deckId);
    localStorage.setItem(
      RECENT_DECKS_KEY,
      JSON.stringify(ids.slice(0, MAX_RECENT))
    );
  } catch {
    /* non-critical */
  }
};

const openDeck = (deckId: number) => {
  trackRecentDeck(deckId);
  deckStore.setActiveDeck(deckId, "mobile");
  router.push("/deck");
};
</script>

<style scoped>
.scrollbar-hide {
  -ms-overflow-style: none;
  scrollbar-width: none;
}
.scrollbar-hide::-webkit-scrollbar {
  display: none;
}
</style>
