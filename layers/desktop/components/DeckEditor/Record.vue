<template lang='pug'>
div(class="rounded-lg flex flex-col gap-y-4 border-[1px] w-full h-1/2 px-6 py-5 bg-white drop-shadow-sm")
  div(class="flex items-center justify-between gap-x-2")
    h2(class="font-semibold") Recording Tools
    div(v-if="!isRecording" class="flex items-center gap-x-2")
      SiPlayFill(@click="recordMacro"  class="w-6 h-6 ")
      p Start Recording
    div(v-else class="flex items-center gap-x-2")
      SiPauseFill(@click="stopRecordingMacro" class="w-6 h-6")
      p Stop Recording
    div(class="flex items-center gap-x-2")
      div(:class="isRecording? 'bg-themeDisconnected  ': 'bg-transparent'" class="w-3 h-3 bg-themeConnected   rounded-full group")
    p(@click="resetRecordedMacro") Reset Macros
   
  div(class="rounded-lg grid grid-cols-7 gap-3 h-full  bg-white overflow-y-auto ")
    
    div( v-for="keybind in recordedMacro" class="w-full p-2 rounded-md aspect-square border-[2px] border-dashed  border-spacing-8 items-center justify-center flex flex-col  gap-y-3 ")
      p {{keybind[0]}}
      //- p delay: {{ (Math.floor(keybind[1] * 1000) / 1000).toFixed(2) }}

</template>
        
<script setup lang='ts'>
import SiPlayFill from '~icons/si/play-fill?width=24px&height=24px';
import SiPauseFill from '~icons/si/pause-fill?width=24px&height=24px';

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";


const recordedMacro = ref([])

const addKeybind = (e) => {
  console.log(e)
  recordedMacro.value.push(e.payload)

  
}



let unlistenMacroRecordedPress: UnlistenFn | undefined;

const isRecording = ref(false);

// Use onMounted to set up event listeners only once
onMounted(async () => {
  // Prevent multiple subscriptions by checking if listeners are already active
  if (!unlistenMacroRecordedPress) {
    unlistenMacroRecordedPress = await listen("macro-step-recorded", addKeybind);
  }
  
});

// Clean up listeners before unmounting
onBeforeUnmount(() => {
if (unlistenMacroRecordedPress) {
  unlistenMacroRecordedPress();
}
  stopRecordingMacro();
});

const recordMacro = async () => {
  if (isRecording.value) {
    console.log("Already recording, please stop the current recording first.");
    return; // Prevent starting a new recording if already recording
  }

  isRecording.value = true; // Mark as recording
  await invoke("start_real_time_macro_recording"); // Start recording
  nextTick();
};

const stopRecordingMacro = async () => {
  if (!isRecording.value) {
    console.log("No recording in progress.");
    return; // Prevent stopping if no recording is in progress
  }

  isRecording.value = false; // Mark as not recording
  await invoke("stop_real_time_macro_recording"); // Stop recording
  nextTick();
};


const resetRecordedMacro = async () => {
  await invoke("reset_macro_recording");
  recordedMacro.value = []
  stopRecordingMacro()
}
</script>
        
<style lang='scss'></style>
