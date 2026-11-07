import { invoke } from "@tauri-apps/api/core";
import { useDeckStore } from "@base/stores/Deck";
import { emitTo } from "@tauri-apps/api/event";
import type { ActionReference } from "@base/types/actionReference";

export const useSetButtonAction = async (
  buttonId: number,
  actionReference: ActionReference,
  label?: string,
) => {
  const deckStore = useDeckStore();
  try {
    // References keep IDs transport-neutral. Rust owns conversion to the
    // database action and validates numeric IDs exactly once.
    const result = await invoke("set_button_action", {
      buttonId,
      actionReference,
      label: label ?? null,
    });

    const decks = await invoke<import("@base/types/deck").Deck[]>("get_all_decks");

    deckStore.setDecks(decks);

    await emitTo("main", "set_active_deck", { deck: deckStore.getActiveDeck });
    console.log("Button action updated successfully", result);
  } catch (error) {
    console.error("Error updating button action:", error);
    throw error;
  }
};
