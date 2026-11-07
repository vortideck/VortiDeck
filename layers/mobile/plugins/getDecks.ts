// import { useDeckStore } from "@/stores/Deck";
import { useConnectionStore } from "@/stores/Connection";
import { computed, watch } from "vue";
import { storeToRefs } from 'pinia'

// import {type Deck} from "@/types/deck"
export default defineNuxtPlugin({
  dependsOn: ['socket'],
  
  async setup() {

    

    const route = useRoute();
    const connectionStore = useConnectionStore()



    const { $socket, $sendSignedMessage } = useNuxtApp()

    const get_deck_message = {
      action: "get_current_deck"
    }

    const get_all_decks_message = {
      action: "get_all_decks"
    }

    // Get initial status and send message if already paired
    const { getStatus } = storeToRefs(connectionStore)

    if (getStatus.value === "paired" && $socket.value?.readyState === WebSocket.OPEN) {
      $sendSignedMessage(get_deck_message);
      $sendSignedMessage(get_all_decks_message);
    }


    // Watch both connection status and socket readyState
    watch([getStatus, () => $socket.value?.readyState], ([newStatus, socketState]) => {
      if (newStatus === "paired" && socketState === WebSocket.OPEN) {
        $sendSignedMessage(get_deck_message);
        $sendSignedMessage(get_all_decks_message);
      }
    })
   


      // Messages are sent via WebSocket to retrieve decks from server
      // Implementation complete via get_deck_message and get_all_decks_message


  }
});
  





