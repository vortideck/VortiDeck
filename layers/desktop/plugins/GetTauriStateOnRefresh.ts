
import { invoke } from "@tauri-apps/api/core";
import { useConnectionStore } from "@/stores/Connection";
import { useDesktopUtils } from "../stores/desktopUtils";
import { useDeckStore } from "@/stores/Deck";

import type { PeerInfo } from "@/types/Peer";
import type { Deck } from "@base/types/deck";
import { useStateBridgeSync } from "../composables/useStateBridgeSync";

export default defineNuxtPlugin(async () => {
  const connectionStore = useConnectionStore();
  const desktopUtils = useDesktopUtils();
  const route = useRoute();
  const deckStore = useDeckStore();
  const { refreshVolatileState } = useStateBridgeSync();

  watch(
    () => route.fullPath,
    () => {
      desktopUtils.closeSidebar();
    }
  );


  async function retrieveDownloadedThemes() {
    const response = await invoke("get_themes")
    if(response.status === 200) {
      return response.data
    }
    
  }

  async function retrieveDecks(): Promise<Deck[]> {
    const decks = await invoke("get_all_decks");
    

    return decks;
  }

  async function retrieveActiveDeck(): Promise<Deck> {
    const deck = await invoke("get_active_deck");
    return deck;
  }

  async function retrievePeers(): Promise<PeerInfo[] | null> {
    try {
      // Safely invoke the Tauri command
      const peers = await invoke<PeerInfo[]>("retrieve_peers");
      const { activePeers: active_peers } = await refreshVolatileState();
      console.log("retrieved active peers:", active_peers);
      console.log("retrieved peers:", peers); // Log the result
      connectionStore.setPeers(peers);
      connectionStore.setPairedActive(active_peers);
      if(connectionStore.getPairedActive.length>=1) {
        connectionStore.setStatus("paired");
      }
      desktopUtils.closeSidebar();

      return peers; // Return the list of services
    } catch (error) {
      console.error("Error discovering services:", error);
      // alert("Error discovering services");
      return null; // Return null in case of error
    }
  }


  await retrievePeers();
  const decks = await retrieveDecks();
  const activeDeck = await retrieveActiveDeck();
  const themes = await retrieveDownloadedThemes()



  deckStore.setDecks(decks);
  deckStore.updateActiveDeck(activeDeck);
  deckStore.setThemes(themes)
});
