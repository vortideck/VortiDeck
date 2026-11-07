<template lang="pug">
div(class="h-full flex flex-col bg-themeBackground overflow-hidden")

  // Header
  div(class="p-6 border-b border-themeBorder2")
    div(class="flex items-center justify-between")
      div(class="pl-[3rem]")
        h1(class="text-2xl font-bold text-themeText flex items-center gap-2")
          MaterialSymbolsCloudOutline(class="w-7 h-7")
          | Virtual OBS Instances
        p(class="text-sm text-themeText2 mt-1") Manage containerized OBS instances for your streams

      button(
        @click="launchNewInstance"
        class="px-4 py-2 bg-blue-500 hover:bg-blue-600 rounded-lg font-medium flex items-center gap-2 transition-colors"
      )
        //- MaterialSymbolsAddOutline(class="w-5 h-5")
        span Launch Instance

  // Main content
  div(class="flex-1 overflow-y-auto p-6")

    // Active instances
    div(v-if="activeInstances.length > 0" class="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-8")
      div(
        v-for="instance in activeInstances"
        :key="instance.id"
        class="p-6 bg-themeBackground2 rounded-lg border border-themeBorder2"
      )
        // Header
        div(class="flex items-center justify-between mb-4")
          div(class="flex items-center gap-3")
            div(class="w-10 h-10 bg-purple-500/20 rounded-lg flex items-center justify-center")
              MaterialSymbolsCloudOutline(class="w-6 h-6 text-purple-400")
            div
              h3(class="font-semibold text-themeText") {{ instance.name }}
              div(class="flex items-center gap-2 mt-1")
                span(class="w-2 h-2 rounded-full bg-green-500 animate-pulse")
                span(class="text-xs text-green-400") Running

          button(
            @click="stopInstance(instance)"
            class="px-3 py-1.5 text-sm bg-red-500/20 hover:bg-red-500/30 text-red-400 rounded transition-colors"
          ) Stop

        // Linked session
        div(v-if="instance.session_id" class="mb-4 p-3 bg-themeBackground rounded border border-themeBorder2")
          div(class="text-xs text-themeText2 mb-1") Linked Session
          div(class="font-medium text-themeText") {{ instance.session_name }}

        // Connection info
        div(class="space-y-2 mb-4")
          div(class="flex items-center justify-between p-2 bg-themeBackground rounded text-sm")
            span(class="text-themeText2") OBS WebSocket
            code(class="text-xs text-blue-400") {{ instance.obs_websocket_url }}

          div(class="flex items-center justify-between p-2 bg-themeBackground rounded text-sm")
            span(class="text-themeText2") RTMP Endpoint
            code(class="text-xs text-blue-400") {{ instance.rtmp_url }}

        // Resources
        div(class="grid grid-cols-2 gap-3 mb-4")
          div(class="p-3 bg-themeBackground rounded")
            div(class="text-xs text-themeText2 mb-1") CPU Usage
            div(class="text-lg font-semibold text-themeText") {{ instance.cpu_usage }}%
            div(class="w-full h-1 bg-gray-700 rounded-full overflow-hidden mt-2")
              div(class="h-full bg-blue-500" :style="{ width: instance.cpu_usage + '%' }")

          div(class="p-3 bg-themeBackground rounded")
            div(class="text-xs text-themeText2 mb-1") Memory
            div(class="text-lg font-semibold text-themeText") {{ instance.memory_usage }}
            div(class="w-full h-1 bg-gray-700 rounded-full overflow-hidden mt-2")
              div(class="h-full bg-purple-500" :style="{ width: (parseFloat(instance.memory_usage) / 2) * 100 + '%' }")

        // Actions
        div(class="flex items-center gap-2")
          button(
            @click="connectToInstance(instance)"
            class="flex-1 px-3 py-2 bg-blue-500 hover:bg-blue-600 rounded transition-colors text-sm"
          ) Connect
          button(
            @click="configureInstance(instance)"
            class="px-3 py-2 bg-themeBackground3 hover:bg-themeBackground rounded transition-colors"
          )
            MaterialSymbolsSettingsOutline(class="w-5 h-5")
          button(
            @click="restartInstance(instance)"
            class="px-3 py-2 bg-orange-500/20 hover:bg-orange-500/30 text-orange-400 rounded transition-colors"
          )
            MaterialSymbolsRefreshOutline(class="w-5 h-5")

    // Empty state
    div(
      v-if="activeInstances.length === 0"
      class="flex flex-col items-center justify-center h-[60vh] text-center"
    )
      MaterialSymbolsCloudOutline(class="w-24 h-24 text-gray-600 mb-4 opacity-50")
      h3(class="text-xl font-semibold text-themeText mb-2") No virtual instances running
      p(class="text-themeText2 mb-6 max-w-md")
        | Launch a virtual OBS instance to stream with pre-configured settings, plugins, and scenes in an isolated container

      button(
        @click="launchNewInstance"
        class="px-6 py-3 bg-blue-500 hover:bg-blue-600 rounded-lg font-medium flex items-center gap-2 transition-colors"
      )
        //- MaterialSymbolsAddOutline(class="w-5 h-5")
        span Launch First Instance

    // Info card
    div(class="mt-8 p-6 bg-blue-500/10 border border-blue-500/20 rounded-lg")
      h3(class="font-semibold text-themeText mb-2 flex items-center gap-2")
        MaterialSymbolsInfoOutline(class="w-5 h-5 text-blue-400")
        | About Virtual Instances

      p(class="text-sm text-themeText2 mb-3")
        | Virtual OBS instances run in Docker containers with pre-configured settings, scenes, and plugins. They provide:

      ul(class="space-y-1 text-sm text-themeText2")
        li • Isolated streaming environments
        li • Pre-configured scene collections
        li • Automatic plugin installation
        li • Remote WebSocket control
        li • Multi-platform restreaming
        li • Easy resource management
</template>

<script setup lang="ts">
import { ref } from "vue";
import MaterialSymbolsCloudOutline from "~icons/material-symbols/cloud-outline";
// import MaterialSymbolsAddOutline from '~icons/material-symbols/add-outline';
import MaterialSymbolsPlayArrowOutline from "~icons/material-symbols/play-arrow-outline";

// Define layout
definePageMeta({
  layout: "streaming",
  title: "Virtual OBS Instances",
  description: "Manage containerized OBS instances",
});

// Mock data - will be replaced with actual virtual instance management
const activeInstances = ref<any[]>([]);

function launchNewInstance() {
  console.log("Launch new instance");
  // TODO: Open instance configuration modal
}

function stopInstance(instance: any) {
  if (confirm(`Stop instance "${instance.name}"?`)) {
    console.log("Stopping instance:", instance);
  }
}

function connectToInstance(instance: any) {
  console.log("Connect to instance:", instance);
  // TODO: Connect to OBS WebSocket
}

function configureInstance(instance: any) {
  console.log("Configure instance:", instance);
}

function restartInstance(instance: any) {
  console.log("Restart instance:", instance);
}
</script>
