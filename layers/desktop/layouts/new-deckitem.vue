<template lang="pug">
    div(class="flex flex-col")
        WindowsTitlebar(class="z-[10001]")
       
       
        div(id="main-app" style="" class="h-[calc(100vh-30px)] z-[90] flex flex-row bg-themeBackground justify-center items-start w-full mt-[30px]  ")
           
            //- div#overlay(class="absolute overlaying top-0 left-0 w-full h-screen bg-black opacity-40 z-10 hidden-overlay")
          
           
            //- Navbar(class=" ")
            button(@click="router.go(-1)" class="fixed top-[3rem] left-[2rem] z-[10000] p-2 rounded-lg hover:bg-themeBackground3 transition-colors")
              svg(class="w-6 h-6 text-themeText" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7")
            
            // Enhanced topbar with step navigation (only for macro creation) - with overflow effect
            div(v-if="route.path.includes('/deck/new/macro')" class="fixed top-[2.5rem] left-1/2 transform -translate-x-1/2 z-[9999] bg-themeBackground2 rounded-xl px-6 py-3 border border-themeBorder2 shadow-xl min-h-[4rem] flex items-center")
              div(class="flex items-center justify-between w-full max-w-5xl")
                // Step title with icon
                div(class="flex items-center space-x-3 min-w-0 flex-1")
                  div(class="w-10 h-10 bg-themeButton/15 border border-themeButton/30 rounded-lg flex items-center justify-center flex-shrink-0")
                    svg(class="w-5 h-5 text-themeButton" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                      path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z")
                  
                  div(class="min-w-0 flex-1")
                    h1(class="text-lg font-bold text-themeText truncate") {{ getStepTitle() }}
                    p(class="text-xs text-themeText2 truncate") Step {{ currentStep + 1 }} of {{ steps.length }} • Create Macro
                                // Progress indicator with enhanced design
                div(class="flex items-center gap-4 ml-8")
                  template(v-for="(stepInfo, index) in steps" :key="index")
                    button(
                      @click="navigateToStep(index)"
                      :disabled="!canNavigateToStep(index)"
                      class="relative flex items-center justify-center w-10 h-10 rounded-full text-sm font-bold transition-all duration-300 hover:scale-110 disabled:cursor-not-allowed group"
                      :class="index <= currentStep ? 'bg-themeButton text-white shadow-sm' : 'bg-themeBackground border border-themeBorder2 text-themeText2 hover:border-themeButton/50'"
                      :title="stepInfo.title"
                    )
                      span {{ index + 1 }}
                      // Tooltip
                      div(class="absolute bottom-full mb-2 left-1/2 transform -translate-x-1/2 px-2 py-1 bg-themeBackground3 text-themeText text-xs border border-themeBorder2 rounded opacity-0 group-hover:opacity-100 transition-opacity duration-150 whitespace-nowrap z-10")
                        | {{ stepInfo.title }}
                    
                    // Connection line
                    div(v-if="index < steps.length - 1" class="w-12 h-1 rounded-full transition-all duration-300" 
                        :class="index < currentStep ? 'bg-themeButton' : 'bg-themeBorder2'")
            
            div(class="flex flex-col w-full  ")
              
              div(class=" flex flex-col  w-full  h-[calc(100vh-30px-4.2rem)] p-2  px-8" :class="route.path.includes('/deck/new/macro') ? 'mt-[4rem]' : 'mt-[4.2rem]'") <!-- Reduced margin for overlay effect -->
                
                //- mt-[9.1dvh]
                //- //min-h-[calc(100dvh-18.2dvh)]
            
                <slot class="" />
                <!-- Footer(v-if="route.path!=='/login' && route.path!=='/signup'" class="mt-12") -->
        
        div(class="cursor cursor--small")
 
 
        
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from "vue";
import { useRouter, useRoute } from "vue-router";
import { useMainStore } from "@/stores/Main";

const mainStore = useMainStore();
const router = useRouter();
const route = useRoute();

// Step navigation state - will be controlled by the macro page
const currentStep = ref(0);
const macroName = ref("");

const steps = [
  { title: "Name Your Macro", description: "Choose a descriptive name" },
  { title: "Record Actions", description: "Capture your keystrokes" },
];

const getStepTitle = () => {
  if (currentStep.value === 0) return "What should we call this macro?";
  if (currentStep.value === 1) return `Recording "${macroName.value}"`;
  return "Create Macro";
};

const canNavigateToStep = (stepIndex: number) => {
  // Can always go back to previous steps
  if (stepIndex <= currentStep.value) return true;

  // Can only go forward if current step is valid
  if (stepIndex === 1) return macroName.value.trim() !== "";

  return false;
};

const navigateToStep = (stepIndex: number) => {
  if (!canNavigateToStep(stepIndex)) return;

  // Emit event to macro page to change step
  if (typeof window !== "undefined") {
    window.dispatchEvent(
      new CustomEvent("navigate-to-step", {
        detail: { step: stepIndex },
      })
    );
  }
};

const handleStepUpdated = (e: any) => {
  currentStep.value = e.detail.step;
  if (e.detail.macroName !== undefined) {
    macroName.value = e.detail.macroName;
  }
};

// Listen for step updates from macro page
onMounted(() => {
  if (typeof window !== "undefined") {
    window.addEventListener("step-updated", handleStepUpdated);
  }
});

onBeforeUnmount(() => {
  if (typeof window !== "undefined") {
    window.removeEventListener("step-updated", handleStepUpdated);
  }
});
</script>t>

<style lang="sass">



*
    scroll-snap-type: y mandatory

body
    overflow-x: hidden



.app-container

    z-index: 10
    display: flex
    flex-direction: column
    justify-content: center



.panel
    margin-top: 60px
    text-align: center
    margin: 0 auto
    scroll-snap-align: center

    width: 90%
    p.title

        font-style: normal
        font-size: 5vh
        line-height: 0%
        align-self: center
        letter-spacing: 0.18em





.cursor
  position: fixed
  left: 0
  top: 0
  pointer-events: none

.cursor--small
  width: 5px
  height: 5px
  left: -2.5px
  top: -2.5px
  border-radius: 50%
  z-index: 11000
  background: var(--color-text)

.cursor--canvas
  width: 100vw
  height: 100vh
  z-index: 12000

.hidden-overlay
  visibility: hidden
  opacity: 0

.visible-overlay
  visibility: visible
  opacity: 0.4
</style>
