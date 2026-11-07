// Import necessary dependencies
// import { useStorage } from "@vueuse/core";
import { type Deck } from "@base/types/deck";
import { defineStore } from "pinia"; // Import defineStore for Pinia
import { emitTo } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useMainStore } from "@base/stores/Main";


export interface DeckTheme {
  id: number;
  name: string;
  backgroundCss: string;
  components: Record<string, Record<string, string>>;
}

interface StatusResponse {
  status: number;
  message: string;
  data?: unknown;
}

type UpdateDeckSettingsResponse = [message: string, status: number, deck?: Deck | null];

interface SignedMessageClient {
  $sendSignedMessage: (message: unknown) => void | Promise<void>;
}



// Define the Pinia store
export const useDeckStore = defineStore("deckStore", {
  state: () => ({
    decks: [] as Deck[], // Default to an empty array for decks
    activeDeck: null as null | Deck, // Null if no deck is active
    defaultTheme: {
      id: 0,
      name: 'default',
      backgroundCss: 'background: radial-gradient(black 15%, transparent 16%) 0 0, radial-gradient(black 15%, transparent 16%) 8px 8px, radial-gradient(rgba(255, 255, 255, 0.1) 15%, transparent 20%) 0 1px, radial-gradient(rgba(255, 255, 255, 0.1) 15%, transparent 20%) 8px 9px; background-color: #282828; background-size: 16px 16px;',
      components: {
        rotaryButton: {
          knobCss: "background-color: #2d3748; box-shadow: 0 1px 3px 0 rgba(0, 0, 0, 0.1), 0 1px 2px 0 rgba(0, 0, 0, 0.06); &:hover { box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06); }",
          pointerCss: "background-color: #ffffff; border-radius: 9999px; box-shadow: 0 1px 2px 0 rgba(0, 0, 0, 0.05);",
          readyCss: "box-shadow: 0 0 15px rgba(0, 255, 255, 0.8), 0 0 30px rgba(0, 255, 255, 0.4); transition: box-shadow 0.2s ease-out;",

        },
        clickButton: {
          buttonCss: "background-color: #1a1a1af7; border: 1px solid #1a1a1a35; box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1); border-radius: 0.75rem; &:hover { box-shadow: 0 6px 12px rgba(0, 0, 0, 0.2); }",
          iconCss: "border-radius: 0.5rem; object-fit: cover; width: 100%; height: 100%;",
          selectedCss: "outline: 2px solid #3b82f6; outline-offset: 4px; border-radius: 0.5rem;"
        }
      }
    },
    themes: [] as DeckTheme[],
    systemVolume: {
      value: 0,
      isButtonChange: false,
      buttonId: null as string | null
    },
    isShift: false,
    isAlt: false,
    isTipIconInDeck: true,
    iconUrlBase: "" as string,
    iconRefreshKey: 0 as number,
  }),
  getters: {
    getActiveDeck(state) {
      return state.activeDeck;
    },
    getActiveDeckById(state) {
      return state.decks.find((deck) => deck.id === state.activeDeck?.id);
    },
    getAllDecks(state) {
      return state.decks;
    },
    getButtonById: (state) => (buttonId: number) => {
      if (state.activeDeck) {
        return state.activeDeck.buttons.find((button) => button.id === buttonId) || null;
      }
      return null;
    },
    getThemes(state) {
      return state.themes;
    },

    getActiveTheme(state) {
      if (!state.activeDeck?.theme) return state.defaultTheme;
      return state.activeDeck.theme;
    },
    getIsShift(state) {
      return state.isShift
    },
    getIsAlt(state) {
      return state.isAlt
    },
    getIsTipIconInDeck(state) {
      return state.isTipIconInDeck
    },
    getSystemVolume(state) {
      return state.systemVolume
    }
  }, 










  actions: {
    async setActiveDeck(id: number, runningFrom: 'desktop' | 'mobile' = 'desktop') {
        const foundDeck = this.decks.find((deck) => deck.id === id);
       
        if (foundDeck) {
            this.activeDeck = foundDeck;

            if(runningFrom === 'desktop') {
              emitTo('mobile_simulator', 'set_active_deck', { deck: this.activeDeck  });
            
              const result = await invoke('update_active_deck', {
                deckId: foundDeck.id
              });

              console.log(result)

            } else {
              const { $sendSignedMessage } = useNuxtApp() as unknown as SignedMessageClient
              $sendSignedMessage({
                action: "update_active_deck",
                payload: {
                  deck_id: foundDeck.id
                }
              });

            }
         
            
           
        } else {
            console.warn(`Deck with id ${id} not found.`);
            // this.activeDeck = null; // Reset to null if not found
       
        }

    },
    // Function to update the active deck
    async updateActiveDeck(deck: Deck) {
      // Ensure the deck exists in the state before updating
      this.activeDeck = deck
    },

    // Function to update a specific deck by ID
    async updateDeckById(id: number, updatedDeck: Deck) {
      const index = this.decks.findIndex(deck => deck.id === id)
      if (index !== -1) {
        this.decks[index] = updatedDeck
      }
    },

    async DeepUpdateDeckById(updatedDeck: Deck) {

      const index = this.decks.findIndex(deck => deck.id === updatedDeck.id)
      if (index !== -1) {
        this.decks[index] = updatedDeck
      }

      await invoke<[string, number]>("update_deck", {
        deckId: Number(updatedDeck.id),
        deckName: updatedDeck.name
      });
    },
    async addDeck(newDeck: Deck) {
        const exists = this.decks?.some((deck) => deck.id === newDeck.id);
        if (!exists) {
            this.decks.push(newDeck);
           
        } else {
            console.warn(`Deck with id ${newDeck.id} already exists.`);
        }

    },
    async createDeck(deckName: string, game: string = ''): Promise<boolean> {
      try {
        const newDeck: Deck = await invoke("create_new_deck", { deckName: deckName })
        console.log(newDeck)
        this.addDeck(newDeck)
        return true
      } catch (error) {
        console.error("Failed to create deck:", error)
        const { showFeatureLimitNotification } = useFeatureLimitNotification();
        showFeatureLimitNotification(error, 'saved_decks');
        return false
      }
    },

    setDecks(decks: Deck[]) {
      this.decks = decks
    },
    removeDeck(deckId: number) {
      console.log(invoke("remove_deck", {deckId})) ;

      this.decks = this.decks.filter((deck) => deck.id !== deckId);
      
      if (this.activeDeck?.id === deckId) {
          this.activeDeck = null; // Reset activeDeck if it's being removed
      }
  },

  toggleMaximizeButtonSpace() {
    if (!this.activeDeck) return;
    this.activeDeck.settings.maximize_button_space = !this.activeDeck.settings.maximize_button_space
    // alert(this.activeDeck.settings.maximize_button_space)
  },

  addTheme(theme: DeckTheme) {
    const exists = this.themes.some((t) => t.id === theme.id);
    if (!exists) {
      this.themes.push(theme);
    } else {
      console.warn(`Theme with id ${theme.id} already exists.`);
    }
  },
  
  async removeTheme(themeId: number) {
    this.themes = this.themes.filter((theme) => theme.id !== themeId);
    
    // If active deck is using this theme, reset its theme
    if (this.activeDeck?.settings?.active_theme === themeId) {
      this.activeDeck.settings.active_theme = null;
      this.activeDeck.theme = null;
    }
    
    console.log("Theme removed from store");
  },

  async setThemes(themes: DeckTheme[]) {
    this.themes = themes
  },
  async setActiveTheme(theme: DeckTheme) {
    if (!this.activeDeck) return;
    try {
      
      this.activeDeck.settings.active_theme = theme.id
      this.activeDeck.theme = theme
      
        
        const currentWindow = getCurrentWebviewWindow();
        if (currentWindow && currentWindow.label !== 'mobile_simulator') {
          const status = await invoke<UpdateDeckSettingsResponse>("update_deck_settings", {
            deckId: Number(this.activeDeck.id),
            deckSettings: this.activeDeck.settings
          });

          if (status[1] === 200) {
            console.log("Deck settings updated.");
          } else {
            console.log("Failed to update deck settings.");
          }

          await emitTo('mobile_simulator', 'set_active_theme', { 
            theme: this.activeDeck.theme
          });
        }
      
    } catch (error) {
      console.error("Error updating deck settings:", error);
    }
  },


  async downloadTheme(theme: DeckTheme) {
    
    const mainStore = useMainStore();
    const response = await invoke<StatusResponse>("download_theme", {
      deckTheme: theme
    });

    

    console.log(response)
    if(response.status === 200) {
      this.setActiveTheme(theme)
      this.addTheme(theme)
  
    
      mainStore.addNotification({
        type: "success",
        message: "Theme downloaded",
        timeout: 5000,
        autoClose: true
      });
    } else {
      mainStore.addNotification({
        type: "error",
        message: "Failed to download theme",
        timeout: 5000,
        autoClose: true
      });
    }
  },


  async uninstallTheme(themeId: number) {
    const mainStore = useMainStore();
    if(this.activeDeck?.theme?.id === themeId) {
      this.setActiveTheme(this.defaultTheme)
    }
    const response = await invoke<StatusResponse>("uninstall_theme", {
      themeId: themeId
    });

    if(response.status === 200) {
      this.removeTheme(themeId);
      mainStore.addNotification({
        type: "success",
        message: "Theme uninstalled",
        timeout: 5000,
        autoClose: true
      });
    } else {
      mainStore.addNotification({
        type: "error",
        message: "Failed to uninstall theme",
        timeout: 5000,
        autoClose: true
      });
    }
  },
  setIsShift(isShift: boolean) {
    this.isShift = isShift;
  },
  setIsAlt(isAlt: boolean) {
    this.isAlt = isAlt;
  },
  setIsTipIconInDeck(isTipIconInDeck: boolean) {
    this.isTipIconInDeck = isTipIconInDeck;
  },
  setSystemVolume(systemVolume: number, isButtonChange: boolean, buttonId?: string | null) {
    this.systemVolume = {
      value: systemVolume,
      isButtonChange: isButtonChange,
      buttonId: buttonId ?? null,
    };
  },
  setIconUrlBase(url: string) {
    this.iconUrlBase = url;
  },
  refreshIcons() {
    this.iconRefreshKey = Date.now();
  },
  },
  persist: {
    storage: sessionStorage,
  },
});
