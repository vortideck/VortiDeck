import { useDeckStore } from "@/stores/Deck";
import { invoke } from "@tauri-apps/api/core";


import {type Deck} from "@/types/deck"
export default defineNuxtPlugin(async (nuxtApp) => {

 
// To stop listening later



    const deckStore = useDeckStore()

    const deck = await invoke("get_active_deck");
    // const activeDeck = await invoke("get_active_deck_id");

    deckStore.updateActiveDeck(deck)
    console.log(deckStore.getAllDecks)
    // console.log(activeDeck)

    
    // if (deckStore.getActiveDeck === null) {
    //     const firstDeckId = deckStore.getAllDecks[0]?.id; // Safely access the first deck's ID
    //     console.log("decks:" + deckStore.getAllDecks)
    //     if (firstDeckId !== undefined) {
    //       deckStore.setActiveDeck(firstDeckId);
    //     }
    //   }



    // now available on `nuxtApp.$injected`
    // nuxtApp.provide("injected", () => "my injected function");
    // // You can alternatively use this format, which comes with automatic type support
  });
  





