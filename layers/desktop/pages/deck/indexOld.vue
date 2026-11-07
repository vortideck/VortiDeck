<template lang="pug">

div(class="flex justify-start flex-col gap-x-8 gap-y-8 h-full pb-6 ")
 
             
  

  div(class="rounded-lg flex flex-col gap-y-4 border-[1px] w-full h-1/2 px-6 py-5 bg-white drop-shadow-sm")
    h2(@click="router.push('deck/index2')" class="font-semibold") Button Properties
    div(class="grid grid-cols-2 gap-8")
      div(class="flex flex-col gap-y-2 ")
        p(class="text-xs ") Label
        input(class="border-[1px] text-sm px-3 py-2 rounded-lg h-[2.5rem]" placeholder="Button Label" v-model="selectedButton.label")
      div(class="flex flex-col gap-y-2")
        p(class="text-xs ") Action
        select(v-model="selectedAction" class="border-[1px] text-sm px-3 h-[2.5rem] py-2 rounded-lg ")
          option() Keyboard Shortcut
          option() Macro
          option() Script
          option() Integration (OBS, etc.)
       
      div(class="flex flex-col gap-y-2")
        p(class="text-xs ") Icon
        button(@click="open" class="border-[1px] text-sm px-3 h-[2.5rem] py-2 gap-x-2 rounded-lg  flex items-center justify-center select-none hover:cursor-pointer")
       
          img(v-if="imageData" :src="imageData" alt="Uploaded Image Preview" class="rounded-lg w-8 h-8 object-cover")
          ArcticonsSimplicon(v-else class="w-6 h-6")
          p Choose Icon
      div(class="flex flex-col gap-y-2")
        p(class="text-xs ") Color
        div(class="text-sm  h-[2.5rem] py-2 gap-x-2 rounded-lg  flex items-center justify-start")
          div(class="rounded-full bg-red-500 w-6 h-6 outline outline-2 outline-offset-2	 outline-slate-600 hover:cursor-pointer ")
          div(class="rounded-full bg-blue-500 w-6 h-6 hover:cursor-pointer ")
          div(class="rounded-full bg-green-500 w-6 h-6 hover:cursor-pointer ")
          div(class="rounded-full bg-purple-500 w-6 h-6 hover:cursor-pointer ")
          div(class="rounded-full bg-yellow-500 w-6 h-6 hover:cursor-pointer ")

  DeckEditorRecord(v-if="selectedAction === 'Macro'")
  DeckEditorScript(v-if="selectedAction === 'Script'")
  div(class="flex justify-center mt-4")
  
    //- div(:class="isLandscape? 'rotate-90': '	 '" class="aspect-[9/16] h-[calc(100vh-12rem)] rotatable-container  bg-white rounded-2xl") 
    //-   component(:is="AsyncComp")
</template>

<script setup lang="ts">
// import Icons
import ArcticonsSimplicon from "~icons/arcticons/simplicon?width=48px&height=48px";
import Fa6SolidPlus from "~icons/fa6-solid/plus?width=392px&height=448px";
import SolarSmartphoneRotate2Linear from "~icons/solar/smartphone-rotate-2-linear?width=24px&height=24px";
// end import Icons
import { type Deck, type Button } from "@/types/deck";
import { useFileDialog } from '@vueuse/core'
import { invoke } from "@tauri-apps/api/core";
import { listen, emitTo } from "@tauri-apps/api/event";
import { useDeckStore} from "@/stores/Deck";
import { readFile, BaseDirectory } from '@tauri-apps/plugin-fs';
import { useDesktopUtils } from "@desktop/stores/desktopUtils";

const desktopUtils = useDesktopUtils();
const isProfileSearch = ref(false)
const router = useRouter()
const deckStore = useDeckStore();

// v-models
const selectedAction = ref('Keyboard Shortcut')



const selectedButton: Ref<Button> = ref({})


// Function to read the image file
// Function to load and convert image to base64
// Create the computed property to load and convert the image

// Define the ref to hold the base64 image data
// Define a ref to store the base64 data of the image



const initSimulator = () => {
  // emitTo('mobile_simulator', 'set_active_deck', { deck: deckStore.getActiveDeck });

}
const selectButton = (button) => {

  selectedButton.value = button.payload
  console.log(selectedButton.value)

};


await listen("mobileDeckLoaded", initSimulator);
await listen("ms_select_button", selectButton)

onMounted(async () => {
  await invoke("open_mobile_simulator");

});

const { $socket } = useNuxtApp();

const AsyncButton = defineAsyncComponent(
  () => import("@/components/Deck/Dynamic/Button.vue")
);

const AsyncLayout = defineAsyncComponent(() => import("@desktop/layouts/deck.vue"));

const sendExampleComponent = () => {
  if ($socket.value.readyState === WebSocket.OPEN) {
    $socket.value.send(
      JSON.stringify({
        action: "component",
        payload: "test component",
      })
    );
  }
};

const isLandscape = ref(false);
const rotateComponent = () => {
  isLandscape.value = !isLandscape.value;
};

const imageData = ref<string | null>(null);  // Store Base64 image data

const { files, open, reset, onCancel, onChange } = useFileDialog({
  accept: 'image/*', // Set to accept only image files
  // Select directories instead of files if set true
})

// Handle file selection
onChange(async (files) => {
  if (files[0]) {
    const file = files[0];

    // Create a FileReader to read the file as a Data URL (Base64)
    const reader = new FileReader();
    reader.onloadend = async () => {
      let base64Data = reader.result as string; // This is the file data in Base64 format


      imageData.value = base64Data; // Set the image preview
      // Remove the 'data:image/png;base64,' part from the Base64 string


      imageData.value = base64Data; // Set the image preview
      let base64SaveData = base64Data.split(',')[1]; // Get only the Base64 data without the prefix
      // Now send this to the Tauri command for processing
      await invoke('save_icon', {
        buttonId: `${selectedButton.value.id}`,
        base64Image: base64SaveData, // Send the clean Base64 data
      });
      await emitTo("mobile_simulator", "refresh");
    };
    reader.readAsDataURL(file); // This reads the file as a Base64 string
  }
});


onCancel(() => {
  /** do something on cancel */
})
</script>

<style lang="scss" scoped>
/* Rotation container simulating the phone screen */

$available-height: calc(100vh - 12rem);

.rotatable-container.landscape {
  width: $available-height; /* Use the calculated height */
}

.rotatable-container.portrait {
  height: $available-height; /* Use the same value for width */
}
</style>
