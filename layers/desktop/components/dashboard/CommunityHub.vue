<template lang="pug">
div(class="flex flex-col p-4 rounded-xl bg-themeBackground2 shadow-sm")
  div(class="flex w-full justify-start items-center gap-x-3 mb-3")
    Fa6SolidUsers(class="w-6 h-6 text-indigo-400")
    p(class="font-semibold text-sm") Community Requests
    button(@click="showRequestsModal = true" class="ml-auto text-xs bg-blue-500 text-white px-3 py-1 rounded-full hover:bg-blue-600 transition-colors flex items-center gap-x-1")
      Fa6SolidChartSimple(class="w-3 h-3")
      span View All Requests
  
  //- Development timeline showing requested features being built
  div(class="relative px-4 mb-4")
    //- Background line
    div(class="absolute top-4 left-0 right-0 h-0.5 bg-themeBackground3")
    
    //- Progress indicator
    div(class="absolute top-4 left-0 h-0.5 bg-gradient-to-r from-blue-400 to-indigo-500" style="width: 70%")
    
    //- Timeline Items - features in development with progress
    div(class="flex justify-between relative")
      //- Started (45% complete)
      div(class="flex flex-col items-center")
        div(class="w-8 h-8 rounded-full bg-green-500 flex items-center justify-center mb-2 shadow-md relative")
          Fa6SolidPlay(class="w-3 h-3 text-white")
          //- Progress ring
          svg(class="absolute -inset-1 w-10 h-10 transform -rotate-90")
            circle(cx="20" cy="20" r="18" stroke="currentColor" stroke-width="2" fill="none" class="text-green-300/30")
            circle(cx="20" cy="20" r="18" stroke="currentColor" stroke-width="2" fill="none" stroke-dasharray="113" stroke-dashoffset="62" class="text-green-400")
        p(class="text-xs font-medium") Discord RPC
        p(class="text-xs text-green-600") 45% Complete
      
      //- Building (20% complete)
      div(class="flex flex-col items-center")
        div(class="w-8 h-8 rounded-full bg-blue-500 flex items-center justify-center mb-2 shadow-md animate-pulse relative")
          Fa6SolidHammer(class="w-3 h-3 text-white")
          //- Progress ring
          svg(class="absolute -inset-1 w-10 h-10 transform -rotate-90")
            circle(cx="20" cy="20" r="18" stroke="currentColor" stroke-width="2" fill="none" class="text-blue-300/30")
            circle(cx="20" cy="20" r="18" stroke="currentColor" stroke-width="2" fill="none" stroke-dasharray="113" stroke-dashoffset="90" class="text-blue-400")
        p(class="text-xs font-medium") Linux Support
        p(class="text-xs text-blue-600") 20% Complete
      
      //- Next Up
      div(class="flex flex-col items-center opacity-60")
        div(class="w-8 h-8 rounded-full bg-themeBackground3 border-2 border-gray-400 border-dashed flex items-center justify-center mb-2")
          Fa6SolidClock(class="w-3 h-3 text-gray-400")
        p(class="text-xs font-medium") Themes
        p(class="text-xs text-themeText/50") Queued
  
  //- Submit idea and popular requests
  div(class="mt-4 pt-3 border-t border-themeText/10")
    div(class="grid grid-cols-1 md:grid-cols-2 gap-3")
      //- Submit idea section
      div(class="bg-themeBackground3/30 rounded-lg p-3")
        div(class="flex items-center gap-x-2 mb-2")
          Fa6SolidLightbulb(class="w-4 h-4 text-blue-400")
          span(class="text-sm font-semibold") Submit Your Idea
        p(class="text-xs text-themeText/60 mb-2") Have a feature request or improvement idea?
        button(@click="openRequestsModalWithSubmission" class="text-xs bg-blue-500 hover:bg-blue-600 text-white px-3 py-1.5 rounded transition-colors w-full flex items-center justify-center gap-x-1")
          Fa6SolidPlus(class="w-3 h-3")
          span Submit Idea
      
      //- Popular upcoming requests
      div(class="space-y-2 relative")
        div(class="text-xs font-bold text-themeText/70 uppercase mb-1") Popular Requests
        div(class="space-y-2 mb-6")
          div(class="flex items-center justify-between bg-themeBackground3/30 rounded px-2 py-1.5")
            p(class="text-xs font-medium") Custom Themes
            div(class="flex items-center gap-x-1")
              Fa6SolidHeart(class="w-3 h-3 text-red-400")
              span(class="text-xs text-blue-600 font-semibold") 234
          div(class="flex items-center justify-between bg-themeBackground3/30 rounded px-2 py-1.5")
            p(class="text-xs font-medium") Spotify Control
            div(class="flex items-center gap-x-1")
              Fa6SolidHeart(class="w-3 h-3 text-red-400")
              span(class="text-xs text-blue-600 font-semibold") 156
          div(class="flex items-center justify-between bg-themeBackground3/30 rounded px-2 py-1.5")
            p(class="text-xs font-medium") Auto Backup
            div(class="flex items-center gap-x-1")
              Fa6SolidHeart(class="w-3 h-3 text-red-400")
              span(class="text-xs text-blue-600 font-semibold") 98
  
  //- Community Requests Modal
  CommunityRequestsModal(:isOpen="showRequestsModal" :startInSubmission="startInSubmission" @close="closeRequestsModal")
</template>

<script setup lang="ts">
import { ref } from 'vue'
import Fa6SolidUsers from "~icons/fa6-solid/users?width=16px&height=16px";
import Fa6SolidChartSimple from "~icons/fa6-solid/chart-simple?width=16px&height=16px";
import Fa6SolidPlay from "~icons/fa6-solid/play?width=16px&height=16px";
import Fa6SolidHammer from "~icons/fa6-solid/hammer?width=16px&height=16px";
import Fa6SolidClock from "~icons/fa6-solid/clock?width=16px&height=16px";
import Fa6SolidLightbulb from "~icons/fa6-solid/lightbulb?width=16px&height=16px";
import Fa6SolidHeart from "~icons/fa6-solid/heart?width=16px&height=16px";
import Fa6SolidPlus from "~icons/fa6-solid/plus?width=16px&height=16px";
import CommunityRequestsModal from '../Modal/CommunityRequests.vue'

const showRequestsModal = ref(false)
const startInSubmission = ref(false)

const openRequestsModalWithSubmission = () => {
  startInSubmission.value = true
  showRequestsModal.value = true
}

const closeRequestsModal = () => {
  showRequestsModal.value = false
  startInSubmission.value = false
}
</script>
