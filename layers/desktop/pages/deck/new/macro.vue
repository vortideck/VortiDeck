<template lang="pug">
  div(class="flex flex-col h-full w-full max-w-[50rem] mx-auto pb-6 px-6")
    // Content area - More height since header moved to topbar
    div(class="macro-action-body w-full flex flex-col h-[600px] overflow-hidden relative")
      
      // Transition container with fixed positioning
      div(class="absolute inset-6")
        transition(name="slide-fade" mode="out-in")
          // Step 0: Macro Name
          div(v-if="step === 0" key="step0" class="absolute inset-0 flex flex-col items-center justify-center space-y-6")
            div(class="text-center space-y-3 ")
              div(class="w-14 h-14 mx-auto bg-themeButton/15 border border-themeButton/30 text-themeButton rounded-xl flex items-center justify-center mb-4")
                svg(class="w-7 h-7 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                  path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 4V2a1 1 0 011-1h8a1 1 0 011 1v2M7 4h10M7 4l-1 16h12l-1-16")
              
              h2(class="text-xl font-bold text-themeText") What should we call this macro?
              p(class="text-themeText2 max-w-md") Choose a descriptive name that helps you remember what this macro does
            
            div(class="w-full max-w-md")
              input(
                v-model="macroName"
                @keydown.enter="nextStep"
                class="w-full px-4 py-3 text-lg text-center border border-themeBorder2 rounded-lg focus:border-themeButton focus:ring-2 focus:ring-themeButton/20 focus:outline-none transition-colors bg-themeBackground text-themeText"
                placeholder="Enter macro name..."
                autofocus
              )
              
              // Reserved space container to prevent layout shift
              div(class="h-8 flex items-center justify-center mt-2")
                transition(name="fade")
                  p(v-show="macroName.trim()" class="text-center text-sm text-themeButton font-medium") Press Enter to continue →
          // Step 2: Recording Interface  
          div(v-else-if="step === 1" key="step1" class="absolute inset-6 flex flex-col space-y-4 pt-4 overflow-hidden")
            // Clean Header
            div(class="flex justify-between items-center p-4 bg-themeBackground2 rounded-xl border border-themeBorder2 shadow-sm flex-shrink-0")
              div(class="flex items-center space-x-3")
                div(class="w-10 h-10 bg-themeButton/15 border border-themeButton/30 text-themeButton rounded-lg flex items-center justify-center")
                  svg(class="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                    path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z")
                div
                  input(
                    class="text-xl font-bold text-themeText bg-transparent border-none focus:ring-0 focus:outline-none w-full"
                    v-model="macroName"
                    :readonly="isRecording"
                    @keydown.enter.prevent
                    placeholder="Enter macro name"
                  )
                  p(class="text-sm text-themeText2") Macro Recording
            
            // Settings Panel
            div(class="flex-shrink-0 bg-themeBackground2 rounded-xl border border-themeBorder2 shadow-sm p-4")
              div(class="flex flex-wrap items-center gap-6")
                // Standard Delays
                div(class="flex items-center space-x-3")
                  div(class="flex items-center space-x-2")
                    input(type="checkbox" v-model="isStandardDelay" class="w-4 h-4 accent-themeButton rounded")
                    label(class="text-sm font-medium text-themeText cursor-pointer" @click="isStandardDelay = !isStandardDelay") Standard Delays
                  div(v-if="isStandardDelay" class="flex items-center space-x-2 ml-2")
                    input(
                      v-model.number="standardDelay" 
                      type="number" 
                      min="20" 
                      max="99999"
                      class="w-16 px-2 py-1 border border-themeBorder2 rounded text-center focus:border-themeButton focus:outline-none bg-themeBackground text-themeText text-sm"
                    )
                    span(class="text-themeText2 text-sm") ms
                
                // Key States
                div(class="flex items-center space-x-2")
                  input(type="checkbox" v-model="isKeyDownUpVisible" class="w-4 h-4 accent-themeButton rounded")
                  label(class="text-sm font-medium text-themeText cursor-pointer" @click="isKeyDownUpVisible = !isKeyDownUpVisible") Show Key States
                
                // Recording Status
                div(class="flex items-center space-x-2 ml-auto")
                  div(class="w-2.5 h-2.5 rounded-full" :class="isRecording ? 'bg-themeError animate-pulse' : 'bg-themeSuccess'")
                  span(class="text-sm font-medium" :class="isRecording ? 'text-themeError' : 'text-themeSuccess'")
                    | {{ isRecording ? 'Recording...' : `${getVisibleActions().length} actions recorded` }}
            
            // Recording Controls
            div(class="flex justify-center space-x-4 flex-shrink-0")
              button(
                v-if="!isRecording"
                @click="recordMacro"
                class="px-6 py-3 bg-themeButton text-white rounded-lg font-semibold hover:brightness-105 transition-all duration-150 shadow-sm"
              )
                div(class="flex items-center space-x-2")
                  svg(class="w-5 h-5" fill="currentColor" viewBox="0 0 24 24")
                    circle(cx="12" cy="12" r="10")
                  span Start Recording
              
              button(
                v-else
                @click="stopRecordingMacro"
                class="px-6 py-3 bg-themeError text-white rounded-lg font-semibold hover:brightness-105 transition-all duration-150 shadow-sm"
              )
                div(class="flex items-center space-x-2")
                  div(class="w-5 h-5 bg-white rounded-sm")
                  span Stop Recording
            
            // Recorded Actions Display
            div(v-if="getVisibleActions().length > 0" class="flex-1 flex flex-col min-h-0 pb-16")
              div(class="flex items-center justify-between mb-3")
                h3(class="text-lg font-semibold text-themeText") Recorded Actions ({{ getVisibleActions().length }})
                button(
                  @click="resetRecordedMacro"
                  class="px-3 py-1 text-sm bg-themeError/10 text-themeError border border-themeError/25 rounded-lg hover:bg-themeError/15 transition-colors"
                )
                  div(class="flex items-center space-x-2")
                    svg(class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                      path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16")
                    span Clear All
              
              // Scrollable container
              div(class="flex-1 w-full overflow-hidden bg-themeBackground3 rounded-xl border border-themeBorder2 shadow-sm p-2")
                div(ref="actionsContainer" class="macro-actions-scroll w-full h-full overflow-x-auto overflow-y-hidden pb-2")
                  div(class="flex items-center space-x-4 h-full py-2 px-4" style="min-width: max-content; width: max-content;")
                    div(
                      v-for="(action, index) in getVisibleActions()"
                      :key="index"
                      class="flex-shrink-0 relative group"
                    )
                      div(class="flex flex-col items-center space-y-2 p-2 bg-themeBackground rounded-xl hover:bg-themeHover transition-colors duration-150 w-[120px] border border-themeBorder2 shadow-sm")
                        // Action number with gradient
                        div(class="w-8 h-8 bg-themeButton text-white rounded-lg flex items-center justify-center text-xs font-bold shadow-sm")
                          span {{ index + 1 }}
                        
                        // Action details
                        div(class="text-center w-full")
                          p(class="font-semibold text-themeText text-xs truncate") {{ getActionDisplay(action.action) }}
                          div(class="mt-1 flex items-center justify-center bg-themeBackground3 rounded-full px-2 py-0.5 border border-transparent focus-within:border-themeButton transition-colors w-[85px] mx-auto")
                            input(
                              type="number"
                              :value="isStandardDelay ? standardDelay : action.delay"
                              @change="updateActionDelay(getOriginalIndex(action), $event)"
                              :disabled="isRecording"
                              class="w-[45px] text-right text-xs text-themeText bg-transparent border-none focus:outline-none focus:ring-0 p-0 disabled:cursor-not-allowed [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none"
                              min="0"
                              max="99999"
                            )
                            span(class="text-[10px] text-themeText3 ml-1") ms
                        
                        // Remove button
                        button(
                          @click="removeAction(getOriginalIndex(action))"
                          class="absolute -top-1 -right-1 w-5 h-5 bg-themeError text-white rounded-full opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center"
                        )
                          svg(class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")
                    
      // Navigation buttons - Outside transition container to prevent jumping
      div(class="absolute bottom-4 left-0 right-0 flex justify-between items-center px-6")
        button(
          v-if="step > 0"
          @click="prevStep"
          class="px-4 py-2 border border-themeBorder2 text-themeText rounded-lg hover:bg-themeBackground3 transition-colors"
        ) 
          | ← Previous
        
        div(class="flex-1")
        
        // Next button for step 0
        button(
          v-if="step < 1"
          @click="nextStep"
          :disabled="!canProceed"
          class="px-6 py-2 bg-themeButton text-white rounded-lg font-semibold hover:brightness-105 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
        ) 
          | Next →
        
        // Save button for step 1 (recording)
        button(
          v-if="step === 1"
          @click="macroId ? updateMacroInDb() : saveMacro()"
          :disabled="recordedMacro.length === 0"
          class="px-6 py-2 bg-themeSuccess text-white rounded-lg font-semibold hover:brightness-105 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
        )
          div(class="flex items-center space-x-2")
            svg(class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7")
            span {{ macroId ? 'Update Macro' : 'Save Macro' }}
  
    // Recording Paused Modal - Only show when actually recording AND paused
    Teleport(to="body")
      Transition(name="fade")
        div(v-if="isRecording && isRecordingPaused" class="fixed inset-0 z-[9999]")
          // Titlebar area - visually covered but no pointer events
          div(class="absolute top-0 left-0 right-0 h-[30px] bg-black/50 pointer-events-none")
          
          // Main clickable area - below titlebar
          div(@click="resumeRecording" class="absolute top-[30px] left-0 right-0 bottom-0 bg-black/50 flex items-center justify-center cursor-pointer")
      
      Transition(name="jump")
        div(v-if="isRecording && isRecordingPaused" class="fixed inset-0 flex items-center justify-center z-[9999] pointer-events-none")
          div(@click.stop="resumeRecording" class="modal-content bg-themeBackground2 rounded-lg p-8 text-center border border-themeBorder2 shadow-xl pointer-events-auto cursor-pointer")
            div(class="w-8 h-8 bg-yellow-500/20 rounded-full flex items-center justify-center mx-auto mb-4")
              svg(class="w-4 h-4 text-yellow-400" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 9v6m4-6v6m7-3a9 9 0 11-18 0 9 9 0 0118 0z")
            
            h3(class="text-lg font-semibold text-white mb-2") Recording Paused
            p(class="text-sm text-themeText2 mb-4") Click anywhere to resume
            
            button(
              @click="resumeRecording"
              class="px-4 py-2 bg-yellow-500 hover:bg-yellow-600 text-white rounded text-sm transition-colors"
            ) Resume
  </template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, nextTick, toRaw, watch, type Ref } from 'vue';
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { useMainStore } from "@/stores/Main";
import { useRouter, useRoute } from "vue-router";

// Define MacroType locally since the import is not working
type MacroType = 'RunOnce' | 'RepeatWhilePressing' | 'Switch' | 'Queue' | '';


const mainStore = useMainStore()
const route = useRoute()
const router = useRouter()
const isKeyDownUpVisible = ref(true)
const standardDelay = ref(50) //ms
const isStandardDelay = ref(true)

const macroName = ref('')
const macroType: Ref<MacroType> = ref('')
const actionsContainer = ref<HTMLElement | null>(null)
const isRecordingPaused = ref(false)
let wheelEventHandler: ((e: WheelEvent) => void) | null = null

// Simple notification functions
const showSuccessNotification = (message: string, title?: string) => {
  mainStore.addNotification({
    title: title || "Success",
    message: message,
    type: "success",
    autoClose: true,
  });
}

const showFeatureLimitNotification = async (error: any, feature: string) => {
  const errorMessage = typeof error === 'string' ? error : error?.message || 'An error occurred';
  mainStore.addNotification({
    title: "Feature Limit",
    message: errorMessage,
    type: "error",
    autoClose: true,
  });
}

const shouldShowKey = (key: any) => {
  if (!key || !key.action) return false;
  // If isKeyDownUpVisible is true, show everything
  if (isKeyDownUpVisible.value) {
    return true;
  }

  // If isKeyDownUpVisible is false, only show KeyDown events
  const actionType = Object.keys(key.action)[0];
  return actionType === 'KeyDown';
}

const step = ref(route.query.step ? parseInt(route.query.step as string) : 0)

// Watch for step changes and stop recording when leaving step 1
watch(step, (newStep, oldStep) => {
  // If we're leaving step 1 (recording) and recording is active, stop it
  if (oldStep === 1 && newStep !== 1 && isRecording.value) {
    stopRecordingMacro()
  }
})

// Watch for macroName changes and propagate to layout
watch(macroName, () => {
  emitStepUpdate()
})



// Navigation functions
const nextStep = () => {
  if (step.value === 0) {
    step.value = 1
    macroType.value = 'RunOnce'
    emitStepUpdate()
  }
}

const prevStep = () => {
  if (step.value === 1) {
    step.value = 0
    emitStepUpdate()
  }
}

const canProceed = computed(() => {
  if (step.value === 0) return macroName.value.trim() !== ''
  return true
})

const emitStepUpdate = () => {
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent("step-updated", {
      detail: {
        step: step.value,
        macroName: macroName.value,
        macroType: macroType.value
      }
    }))
  }
}

const macroId = ref(route.query.macroId)
const copyFromId = ref(route.query.copyFrom)


const selectType = (type: MacroType) => {
    macroType.value = type
    step.value = 1
}


const recordedMacro = ref([])




function scrollToBottom() {
  const macroContainer = document.getElementById('macro-container'); // No `#` prefix here
  if (macroContainer) {
    macroContainer.scrollTop = macroContainer.scrollHeight;
  }
}

function scrollToTop() {
  const macroContainer = document.getElementById('macro-container'); // No `#` prefix here
  if (macroContainer) {
    macroContainer.scrollTop = 0; // Scroll to the top of the container
  }
}


const addKeybind = (e: any) => {
    console.log(e.payload)

    recordedMacro.value.push(e.payload);

    nextTick(() => {
        // Auto-scroll to newest action (scroll to right)
        if (actionsContainer.value) {
            actionsContainer.value.scrollLeft = actionsContainer.value.scrollWidth;
        }
    });
};
const saveMacro = async () => {
  try {
    const rawActions = toRaw(recordedMacro.value);
    const finalActions = isStandardDelay.value 
      ? rawActions.map(action => ({ ...action, delay: standardDelay.value }))
      : rawActions;
    const macroCommandString = JSON.stringify(finalActions);

    console.log("Saving Macro Actions:", finalActions);

    // Pass the parameters to the backend (new format)
    const response = await invoke('save_macro_to_db', {
      macroName: macroName.value, // Individual parameter
      macroType: macroType.value, // Individual parameter
      macroChain: macroCommandString,
    });

    console.log("Macro saved successfully:", response); // Handle success response

    // Show success notification
    showSuccessNotification(response || "Macro saved successfully", "Macro Saved");

    router.go(-1); // Navigate back
  } catch (error) {
    console.error("Error saving macro:", error); // Handle any errors

    // Show error notification using the reusable composable
    await showFeatureLimitNotification(error, "macro");
  }
};





let unlistenMacroRecordedPress: UnlistenFn | undefined;

const isRecording = ref(false);

const preventCheckboxKeyboardInteraction = (event: KeyboardEvent) => {
  const target = event.target as HTMLElement;

  // If the focused element is a checkbox, prevent the default behavior
  if (target && target.tagName === 'INPUT' && (target as HTMLInputElement).type === 'checkbox') {
    event.preventDefault();
  }
};

// Add window focus/blur event handlers with delay to prevent titlebar click interference
let blurTimeout: NodeJS.Timeout | null = null;

const handleWindowFocus = () => {
  // Clear any pending blur timeout
  if (blurTimeout) {
    clearTimeout(blurTimeout);
    blurTimeout = null;
  }

  if (isRecording.value && isRecordingPaused.value) {
    resumeRecording();
  }
};

const handleWindowBlur = () => {
  // Add a small delay to prevent titlebar clicks from triggering pause
  blurTimeout = setTimeout(() => {
    if (isRecording.value && !isRecordingPaused.value) {
      pauseRecording();
    }
  }, 100); // 100ms delay
};

const pauseRecording = () => {
  if (isRecording.value) {
    isRecordingPaused.value = true;
    // Could also invoke backend pause if needed
    // await invoke("pause_macro_recording");
  }
};

const resumeRecording = () => {
  if (isRecording.value && isRecordingPaused.value) {
    isRecordingPaused.value = false;
    // Could also invoke backend resume if needed
    // await invoke("resume_macro_recording");
  }
};

const handleNavigateToStep = (e: any) => {
  step.value = e.detail.step;
  emitStepUpdate();
};

// Use onMounted to set up event listeners only once
onMounted(async () => {
  // RESET RECORDING STATE ON PAGE REFRESH
  // Stop any ongoing recording from previous session
  try {
    await invoke("stop_real_time_macro_recording");
  } catch (error) {
    // Ignore errors if recording wasn't active
    console.log("No active recording to stop");
  }

  // Reset recording state variables to ensure clean start
  isRecording.value = false;
  isRecordingPaused.value = false;

  // Clear recorded macro if not editing existing macro
  if (!macroId.value) {
    recordedMacro.value = [];
  }

  // Prevent multiple subscriptions by checking if listeners are already active
  if (!unlistenMacroRecordedPress) {
    unlistenMacroRecordedPress = await listen("macro-step-recorded", addKeybind);
  }
  document.addEventListener('keydown', preventCheckboxKeyboardInteraction);

  // Add window focus/blur/navigation listeners
  window.addEventListener('focus', handleWindowFocus);
  window.addEventListener('blur', handleWindowBlur);
  window.addEventListener('navigate-to-step', handleNavigateToStep);

  // Initial step update
  emitStepUpdate();
});

// Clean up listeners before unmounting
onBeforeUnmount(() => {
if (unlistenMacroRecordedPress) {
  unlistenMacroRecordedPress();
}
  stopRecordingMacro();
  document.removeEventListener('keydown', preventCheckboxKeyboardInteraction);

  // Clean up window listeners
  window.removeEventListener('focus', handleWindowFocus);
  window.removeEventListener('blur', handleWindowBlur);
  window.removeEventListener('navigate-to-step', handleNavigateToStep);

  // Clear any pending timeout
  if (blurTimeout) {
    clearTimeout(blurTimeout);
  }

  // Clean up wheel event listener
  if (actionsContainer.value && wheelEventHandler) {
    actionsContainer.value.removeEventListener('wheel', wheelEventHandler);
  }
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
  scrollToTop()

};


const resetRecordedMacro = async () => {
  await invoke("reset_macro_recording");
  recordedMacro.value = []
  stopRecordingMacro()
}


const findCurrentMacroData = async () => {
  try {
    // Check if we're in copy mode or edit mode
    const idToLoad = copyFromId.value || macroId.value;
    if (!idToLoad) return;

    const result: any = await invoke("get_macro_by_id", {
        macroId: Number(idToLoad),
    });
    console.log("Loaded macro data:", result);

    if (result && result.data) {
      const macroData = result.data;
      macroName.value = copyFromId.value ? `${macroData.name} (Copy)` : macroData.name;
      macroType.value = macroData.macro_type;
      
      try {
        recordedMacro.value = JSON.parse(macroData.macro_chain || "[]");
        
        // Auto-detect standard delays based on loaded steps
        if (recordedMacro.value.length > 0) {
          const firstDelay = recordedMacro.value[0].delay;
          const allDelaysEqual = recordedMacro.value.every((action: any) => action.delay === firstDelay);
          if (allDelaysEqual) {
            isStandardDelay.value = true;
            standardDelay.value = firstDelay;
          } else {
            isStandardDelay.value = false;
          }
        } else {
          isStandardDelay.value = true;
        }
      } catch (parseError) {
        console.error("Failed to parse macro chain JSON:", parseError);
        recordedMacro.value = [];
        isStandardDelay.value = true;
      }
    } else {
      console.warn("No data returned for macro ID:", idToLoad);
    }

    // If copying, clear the macroId so it creates a new entry
    if (copyFromId.value) {
        macroId.value = null;
    }
  } catch (error) {
    console.error("Error loading macro data:", error);
  }
}

const updateMacroInDb = async () => {
  try {
    const rawActions = toRaw(recordedMacro.value);
    const finalActions = isStandardDelay.value 
      ? rawActions.map(action => ({ ...action, delay: standardDelay.value }))
      : rawActions;
    const macroCommandString = JSON.stringify(finalActions);

    console.log("Updating macro with data:", finalActions);

    const response: any = await invoke('update_macro', {
      macroId: Number(macroId.value),
      macroName: macroName.value,
      macroType: macroType.value,
      macroChain: macroCommandString,
    });

    if (response.status === "success") {
      showSuccessNotification(response.message || "Macro updated successfully", "Macro Updated");
      router.go(-1);
    } else {
      throw new Error(response.message || "Unknown error occurred");
    }
  } catch (error: any) {
    console.error("Error updating macro:", error);
    await showFeatureLimitNotification(error, "macro");
  }
};

// Remove an action from the recorded macro
const removeAction = (index: number) => {
  recordedMacro.value.splice(index, 1);
};

// Watch for macroId on page load to populate data for editing
onMounted(async () => {
  await findCurrentMacroData();
});

// Watch for actionsContainer to become available and set up horizontal scrolling
watch(actionsContainer, (newContainer) => {
  // Clean up previous event listener if it exists
  if (wheelEventHandler && actionsContainer.value) {
    actionsContainer.value.removeEventListener('wheel', wheelEventHandler);
  }

    if (newContainer) {
    // Remove smooth behavior for more responsive infinite scroll
    newContainer.style.scrollBehavior = 'auto';

    let lastScrollTime = 0;
    let scrollAccumulator = 0;
    let animationId: number | null = null;

    const smoothScroll = () => {
      if (Math.abs(scrollAccumulator) > 0.1) {
        // Apply a portion of the accumulated scroll for smooth movement
        const scrollStep = scrollAccumulator * 0.15; // Smooth step size
        newContainer.scrollLeft += scrollStep;
        scrollAccumulator -= scrollStep;

        animationId = requestAnimationFrame(smoothScroll);
      } else {
        animationId = null;
        scrollAccumulator = 0;
      }
    };

    wheelEventHandler = (e: WheelEvent) => {
      // Always prevent default and scroll horizontally for this container
      e.preventDefault();
      e.stopPropagation();

      const now = Date.now();
      const timeDiff = now - lastScrollTime;
      const scrollAmount = e.deltaY * 1.5;

      // If it's been less than 16ms since last scroll (60fps), it's likely infinite scroll
      if (timeDiff < 16) {
        // For infinite scroll: accumulate for smooth continuous movement
        scrollAccumulator += scrollAmount;
      } else {
        // For regular scroll: direct movement
        newContainer.scrollLeft += scrollAmount;
      }

      // Start smooth animation if not already running
      if (!animationId) {
        animationId = requestAnimationFrame(smoothScroll);
      }

      lastScrollTime = now;
    };

    newContainer.addEventListener('wheel', wheelEventHandler, { passive: false });
  }
}, { immediate: true });

const selectTypeAndNext = (type: MacroType) => {
  macroType.value = type
  // Auto-redirect immediately for type selection
  nextStep()
}

const getActionTypeDisplay = (action: any) => {
  if (!action) return 'Unknown';
  const actionType = Object.keys(action)[0];
  if (!actionType) return 'Unknown';
  switch (actionType) {
    case 'KeyDown': return 'Key Press'
    case 'KeyUp': return 'Key Release'
    case 'MouseMove': return 'Mouse Movement'
    case 'MouseClick': return 'Mouse Click'
    case 'ObsAction': return 'OBS Action'
    case 'SystemControl': return 'System Control'
    default: return actionType
  }
}

const getActionDisplay = (action: any) => {
  if (!action) return 'Unknown';
  const actionType = Object.keys(action)[0];
  if (!actionType) return 'Unknown';
  const actionValue = action[actionType]

  switch (actionType) {
    case 'KeyDown': return `Press ${actionValue}`
    case 'KeyUp': return `Release ${actionValue}`
    case 'MouseMove': return `Mouse Move`
    case 'MouseClick': return `Mouse Click`
    case 'ObsAction': return 'OBS Action'
    case 'SystemControl': return 'System Control'
    default: return actionType
  }
}

const getVisibleActions = () => {
  return recordedMacro.value.filter(action => shouldShowKey(action));
}

const getOriginalIndex = (action: any) => {
  return recordedMacro.value.findIndex(a => a === action);
}

const updateActionDelay = (originalIndex: number, event: Event) => {
  if (originalIndex !== -1) {
    const target = event.target as HTMLInputElement;
    let delay = Number(target.value);
    if (isNaN(delay) || delay < 0) {
      delay = 0;
    }
    recordedMacro.value[originalIndex].delay = delay;
    // Automatically turn off standard delay if they edit a delay manually
    isStandardDelay.value = false;
  }
}
</script>

<style scoped>
.macro-action-body {
  border: 1px solid hsl(var(--border2));
  border-radius: 0.9rem;
  background: hsl(var(--background2));
}

.macro-actions-scroll {
  scrollbar-width: thin;
  scrollbar-color: hsl(var(--button) / 0.55) hsl(var(--background));
}

.macro-actions-scroll::-webkit-scrollbar {
  height: 7px;
}

.macro-actions-scroll::-webkit-scrollbar-button {
  display: none;
  width: 0;
  height: 0;
}

.macro-actions-scroll::-webkit-scrollbar-track {
  background: hsl(var(--background));
  border-radius: 9999px;
}

.macro-actions-scroll::-webkit-scrollbar-thumb {
  background: hsl(var(--button) / 0.55);
  border-radius: 9999px;
}

.macro-actions-scroll::-webkit-scrollbar-thumb:hover {
  background: hsl(var(--button) / 0.75);
}

/* Smooth transitions for absolutely positioned steps */
.slide-fade-enter-active {
  transition: opacity 0.4s ease, transform 0.4s ease;
}

.slide-fade-leave-active {
  transition: opacity 0.3s ease, transform 0.3s ease;
}

.slide-fade-enter-from {
  opacity: 0;
  transform: translateX(20px);
}

.slide-fade-leave-to {
  opacity: 0;
  transform: translateX(-20px);
}

/* Transition container needs relative positioning */
.bg-themeBackground2 {
  position: relative;
}

/* Modal styles */
.modal-content {
  background-color: hsl(var(--background2));
  min-width: 300px;
}

/* Fade animation for dynamic elements */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

/* Modal jump animation */
.jump-enter-active {
  transition: transform 0.3s ease, opacity 0.3s ease;
}

.jump-leave-active {
  transition: transform 0.3s ease, opacity 0.3s ease;
}

.jump-enter-from {
  transform: translateY(-50px);
  opacity: 0;
}

.jump-leave-to {
  transform: translateY(-50px);
  opacity: 0;
}

/* Custom scrollbar for recorded actions */
.overflow-y-auto::-webkit-scrollbar {
  width: 6px;
}

.overflow-y-auto::-webkit-scrollbar-track {
  @apply bg-gray-100 dark:bg-gray-700 rounded-full;
}

.overflow-y-auto::-webkit-scrollbar-thumb {
  @apply bg-gray-300 dark:bg-gray-500 rounded-full;
}

.overflow-y-auto::-webkit-scrollbar-thumb:hover {
  @apply bg-gray-400 dark:bg-gray-400;
}

/* Pulse animation for recording indicator */
@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.5;
  }
}

.animate-pulse {
  animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
}

/* Hover effects for cards */
.hover\:scale-105:hover {
  transform: scale(1.05);
}

/* Focus styles for better accessibility */
input:focus,
button:focus {
  outline: 2px solid rgba(59, 130, 246, 0.5);
  outline-offset: 2px;
}

/* Gradient background animation */
@keyframes gradient {
  0% {
    background-position: 0% 50%;
  }
  50% {
    background-position: 100% 50%;
  }
  100% {
    background-position: 0% 50%;
  }
}

.bg-gradient-to-br {
  background-size: 400% 400%;
  animation: gradient 15s ease infinite;
}

/* Button press animation */
.transform:active {
  transform: scale(0.98);
}

/* Progress bar animation */
.progress-step {
  transition: all 0.3s ease;
}

/* Card hover shadow enhancement */
.hover\:shadow-lg:hover {
  box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04);
}

/* Dark mode specific adjustments */
@media (prefers-color-scheme: dark) {
  .hover\:shadow-lg:hover {
    box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.3), 0 10px 10px -5px rgba(0, 0, 0, 0.2);
  }
}

/* Responsive design improvements */
@media (max-width: 768px) {
  .grid-cols-2 {
    grid-template-columns: 1fr;
  }

  .max-w-4xl {
    max-width: 95vw;
  }
}
</style>
