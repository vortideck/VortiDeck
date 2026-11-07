<template lang="pug">
div(class="flex flex-col w-full gap-y-6 max-w-7xl mx-auto pb-16 select-text")
  //- Access Warning for Production (with Dev Bypass)
  div(v-if="!isDev && !devBypass" class="flex flex-col items-center justify-center p-12 rounded-2xl bg-themeBackground2 border border-themeBorder text-center max-w-md mx-auto my-12 shadow-sm space-y-4")
    div(class="w-14 h-14 rounded-2xl bg-amber-500/10 border border-amber-500/20 flex items-center justify-center text-amber-400")
      MdiBug(class="w-7 h-7")
    div
      h2(class="text-xl font-bold text-themeText") Development Mode Only
      p(class="text-xs text-themeText2 mt-1") This console is intended for developer testing and internal diagnostics.
    button(
      @click="devBypass = true"
      class="px-4 py-2 rounded-xl bg-themeBackground hover:bg-themeBackground3 text-themeText border border-themeBorder text-xs font-semibold transition-all cursor-pointer shadow-2xs"
    ) Enable Developer Preview

  div(v-else class="space-y-6")
    //- Hero Header Banner
    div(class="flex flex-col md:flex-row md:items-center justify-between gap-4 p-6 rounded-2xl bg-themeBackground2 border border-themeBorder shadow-xs")
      div(class="space-y-1.5")
        div(class="flex items-center gap-3")
          div(class="w-11 h-11 rounded-2xl bg-themePrimary/10 border border-themePrimary/20 flex items-center justify-center text-themePrimary shadow-2xs")
            MdiBug(class="w-6 h-6")
          div
            div(class="flex items-center gap-2")
              h1(class="text-xl font-bold text-themeText") VortiDeck Developer Console
              span(class="px-2.5 py-0.5 rounded-full text-xs font-semibold bg-themePrimary/10 text-themePrimary border border-themePrimary/20") Dev Suite
            p(class="text-xs text-themeText2 mt-0.5") Core developer diagnostics: JWT simulator, plugin management, streaming overlay testing, and live event telemetry

      div(class="flex flex-wrap items-center gap-2")
        div(class="flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-themeBackground border border-themeBorder text-xs text-themeText2 shadow-2xs")
          span(class="w-2 h-2 rounded-full bg-emerald-500 animate-pulse")
          span(class="font-medium") {{ isDev ? 'Dev Environment' : 'Dev Preview' }}

        div(v-if="serverInfo && serverInfo.ip" class="flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-themeBackground border border-themeBorder text-xs text-themeText2 shadow-2xs")
          span(class="text-themeText3") WS Server:
          span(class="font-mono font-medium text-themeText") {{ serverInfo.ip }}:{{ serverInfo.port }}

        button(
          @click="clearAllResults"
          class="px-3.5 py-1.5 rounded-xl bg-red-500/10 hover:bg-red-500/20 text-red-400 border border-red-500/20 text-xs font-semibold transition-all cursor-pointer flex items-center gap-1.5 active:scale-95 shadow-2xs"
          title="Clear all test output results"
        )
          MdiTrashCan(class="w-3.5 h-3.5")
          span Clear Results

    //- Quick Navigation & Suite Filter Bar
    div(class="flex flex-col lg:flex-row lg:items-center justify-between gap-3 bg-themeBackground2 p-2 rounded-2xl border border-themeBorder shadow-2xs")
      //- Category Tabs
      div(class="flex flex-wrap gap-1")
        button(
          v-for="tab in navigationTabs"
          :key="tab.id"
          @click="activeTab = tab.id"
          :class="activeTab === tab.id ? 'bg-themeBackground text-themeText shadow-xs font-semibold border-themeBorder' : 'text-themeText2 hover:text-themeText border-transparent hover:bg-themeBackground/50'"
          class="px-3.5 py-1.5 rounded-xl text-xs font-medium transition-all border flex items-center gap-1.5 cursor-pointer"
        )
          component(:is="tab.icon" class="w-3.5 h-3.5")
          span {{ tab.label }}
          span(v-if="tab.id === 'events' && pluginEvents.length > 0" class="px-1.5 py-0.5 rounded-full text-[10px] bg-themeBackground3 text-themeText3 font-semibold") {{ pluginEvents.length }}

      //- Global Action Triggers
      div(class="flex flex-wrap items-center gap-2 px-1")
        button(
          @click="openLogsWindow"
          class="px-3 py-1.5 rounded-xl bg-themeBackground hover:bg-themeBackground3 text-themeText border border-themeBorder text-xs font-medium transition-colors cursor-pointer flex items-center gap-1.5 shadow-2xs active:scale-95"
          title="Open Dedicated Logs Window"
        )
          MdiFileDocumentOutline(class="w-3.5 h-3.5 text-themePrimary")
          span System Logs

        button(
          @click="openSimulatorWindow"
          class="px-3 py-1.5 rounded-xl bg-themeBackground hover:bg-themeBackground3 text-themeText border border-themeBorder text-xs font-medium transition-colors cursor-pointer flex items-center gap-1.5 shadow-2xs active:scale-95"
          title="Open Mobile Deck Simulator"
        )
          MdiPlay(class="w-3.5 h-3.5 text-blue-400")
          span Simulator

        button(
          @click="openOnboardingModal"
          class="px-3 py-1.5 rounded-xl bg-themeBackground hover:bg-themeBackground3 text-themeText border border-themeBorder text-xs font-medium transition-colors cursor-pointer flex items-center gap-1.5 shadow-2xs active:scale-95"
        )
          MdiQrcode(class="w-3.5 h-3.5 text-amber-400")
          span QR Onboarding

        button(
          @click="getSystemInfo"
          class="px-3 py-1.5 rounded-xl bg-themeBackground hover:bg-themeBackground3 text-themeText border border-themeBorder text-xs font-medium transition-colors cursor-pointer flex items-center gap-1.5 shadow-2xs active:scale-95"
        )
          MdiRefresh(class="w-3.5 h-3.5 text-themeText2" :class="{ 'animate-spin': isRefreshingSystem }")
          span System Specs

    //- Global Toast Notification
    transition(name="fade")
      div(v-if="toastMessage" class="flex items-center justify-between p-3.5 rounded-xl bg-themeBackground2 border border-themeBorder shadow-md text-xs text-themeText")
        div(class="flex items-center gap-2")
          MdiCheck(class="w-4 h-4 text-emerald-400")
          span {{ toastMessage }}
        button(@click="toastMessage = ''" class="text-themeText3 hover:text-themeText text-xs") ✕

    //- Main Diagnostic Cards Grid
    div(class="grid grid-cols-1 xl:grid-cols-2 gap-6")

      //- 1. JWT Authentication & User Tiers Section
      div(v-show="activeTab === 'all' || activeTab === 'auth'" class="flex flex-col justify-between p-6 rounded-2xl bg-themeBackground2 border border-themeBorder hover:border-themeBorder2 transition-all shadow-xs space-y-5")
        div(class="space-y-4")
          div(class="flex items-center justify-between border-b border-themeBorder pb-3")
            div(class="flex items-center gap-2.5")
              div(class="w-8 h-8 rounded-xl bg-blue-500/10 border border-blue-500/20 flex items-center justify-center text-blue-400")
                MdiShieldKey(class="w-4 h-4")
              div
                h2(class="text-sm font-bold text-themeText") JWT & User Authentication
                p(class="text-[11px] text-themeText2") Simulate guest, free, and premium user states in local dev
            div(class="px-2.5 py-0.5 rounded-full text-[11px] font-semibold bg-blue-500/10 text-blue-400 border border-blue-500/20")
              span {{ activeAuthTier }}

          //- Auth Actions Grid
          div(class="grid grid-cols-1 sm:grid-cols-2 gap-2.5")
            button(
              @click="testGuestMode"
              class="px-3.5 py-2.5 rounded-xl bg-themeBackground hover:bg-themeBackground3 text-themeText border border-themeBorder text-xs font-semibold transition-all flex items-center gap-2 cursor-pointer shadow-2xs active:scale-95 group"
            )
              span 🎭
              span(class="flex-1 text-left") Test Guest Mode
              span(class="text-[10px] text-themeText3 group-hover:text-themeText") 0 limits

            button(
              @click="checkPluginValidation"
              class="px-3.5 py-2.5 rounded-xl bg-themeBackground hover:bg-themeBackground3 text-themeText border border-themeBorder text-xs font-semibold transition-all flex items-center gap-2 cursor-pointer shadow-2xs active:scale-95 group"
            )
              span 🔍
              span(class="flex-1 text-left") Validate Permissions
              span(class="text-[10px] text-themeText3 group-hover:text-themeText") Check

        //- Output Viewer
        div(v-if="jwtTestResult" class="mt-4 pt-3 border-t border-themeBorder")
          div(class="flex items-center justify-between mb-2")
            div(class="flex items-center gap-1.5 text-xs text-themeText2")
              MdiLightningBolt(class="w-3.5 h-3.5 text-amber-400")
              span(class="font-mono") {{ jwtExecutionTime ? `⚡ ${jwtExecutionTime}ms` : 'Result' }}
            button(
              @click="copyText(jwtTestResult, 'JWT result copied!')"
              class="px-2 py-1 rounded-md text-[11px] bg-themeBackground hover:bg-themeBackground3 text-themeText2 hover:text-themeText transition-colors flex items-center gap-1 cursor-pointer"
            )
              MdiContentCopy(class="w-3 h-3")
              span Copy
          div(class="p-3 rounded-xl bg-themeBackground border border-themeBorder font-mono text-xs text-themeText whitespace-pre-wrap max-h-36 overflow-y-auto select-all leading-relaxed")
            | {{ jwtTestResult }}

      //- 2. Plugin System & Sandboxing Section
      div(v-show="activeTab === 'all' || activeTab === 'plugins'" class="flex flex-col justify-between p-6 rounded-2xl bg-themeBackground2 border border-themeBorder hover:border-themeBorder2 transition-all shadow-xs space-y-5")
        div(class="space-y-4")
          div(class="flex items-center justify-between border-b border-themeBorder pb-3")
            div(class="flex items-center gap-2.5")
              div(class="w-8 h-8 rounded-xl bg-purple-500/10 border border-purple-500/20 flex items-center justify-center text-purple-400")
                MdiPuzzle(class="w-4 h-4")
              div
                h2(class="text-sm font-bold text-themeText") Plugin System & Sandboxing
                p(class="text-[11px] text-themeText2") Inspect installed plugins, hot-reload, and inspect permissions
            div(class="px-2.5 py-0.5 rounded-full text-[11px] font-semibold bg-purple-500/10 text-purple-400 border border-purple-500/20")
              span Core Engine

          //- Plugin Actions Grid
          div(class="grid grid-cols-1 sm:grid-cols-2 gap-2.5")
            button(
              @click="getInstalledPlugins"
              class="px-3.5 py-2.5 rounded-xl bg-themeBackground hover:bg-themeBackground3 text-themeText border border-themeBorder text-xs font-semibold transition-all flex items-center gap-2 cursor-pointer shadow-2xs active:scale-95"
            )
              MdiTextBoxSearch(class="w-4 h-4 text-purple-400")
              span List Installed

            button(
              @click="refreshPlugins"
              class="px-3.5 py-2.5 rounded-xl bg-themeBackground hover:bg-themeBackground3 text-themeText border border-themeBorder text-xs font-semibold transition-all flex items-center gap-2 cursor-pointer shadow-2xs active:scale-95"
            )
              MdiRefresh(class="w-4 h-4 text-emerald-400")
              span Refresh Plugins

            button(
              @click="reloadPluginsWithAuth"
              class="px-3.5 py-2.5 rounded-xl bg-themeBackground hover:bg-themeBackground3 text-themeText border border-themeBorder text-xs font-semibold transition-all flex items-center gap-2 cursor-pointer shadow-2xs active:scale-95"
            )
              MdiShieldKey(class="w-4 h-4 text-amber-400")
              span Reload with Auth Check

            button(
              @click="checkPluginLimits"
              class="px-3.5 py-2.5 rounded-xl bg-themeBackground hover:bg-themeBackground3 text-themeText border border-themeBorder text-xs font-semibold transition-all flex items-center gap-2 cursor-pointer shadow-2xs active:scale-95"
            )
              MdiLightningBolt(class="w-4 h-4 text-blue-400")
              span Check Limits

            button(
              @click="getPluginsDirectory"
              class="px-3.5 py-2.5 rounded-xl bg-themeBackground hover:bg-themeBackground3 text-themeText border border-themeBorder text-xs font-semibold transition-all flex items-center gap-2 cursor-pointer shadow-2xs active:scale-95"
            )
              MdiFileDocumentOutline(class="w-4 h-4 text-orange-400")
              span Show Directory

            button(
              @click="debugPluginSubsystem"
              class="px-3.5 py-2.5 rounded-xl bg-themeBackground hover:bg-themeBackground3 text-themeText border border-themeBorder text-xs font-semibold transition-all flex items-center gap-2 cursor-pointer shadow-2xs active:scale-95"
            )
              MdiBug(class="w-4 h-4 text-rose-400")
              span Debug Subsystem

        //- Output Viewer
        div(v-if="pluginTestResult" class="mt-4 pt-3 border-t border-themeBorder")
          div(class="flex items-center justify-between mb-2")
            div(class="flex items-center gap-1.5 text-xs text-themeText2")
              MdiLightningBolt(class="w-3.5 h-3.5 text-amber-400")
              span(class="font-mono") {{ pluginExecutionTime ? `⚡ ${pluginExecutionTime}ms` : 'Plugin Output' }}
            button(
              @click="copyText(pluginTestResult, 'Plugin result copied!')"
              class="px-2 py-1 rounded-md text-[11px] bg-themeBackground hover:bg-themeBackground3 text-themeText2 hover:text-themeText transition-colors flex items-center gap-1 cursor-pointer"
            )
              MdiContentCopy(class="w-3 h-3")
              span Copy
          div(class="p-3 rounded-xl bg-themeBackground border border-themeBorder font-mono text-xs text-themeText whitespace-pre-wrap max-h-40 overflow-y-auto select-all leading-relaxed")
            | {{ pluginTestResult }}

      //- 3. Overlay & OBS Integration Section
      div(v-show="activeTab === 'all' || activeTab === 'overlay'" class="flex flex-col justify-between p-6 rounded-2xl bg-themeBackground2 border border-themeBorder hover:border-themeBorder2 transition-all shadow-xs space-y-5")
        div(class="space-y-4")
          div(class="flex items-center justify-between border-b border-themeBorder pb-3")
            div(class="flex items-center gap-2.5")
              div(class="w-8 h-8 rounded-xl bg-indigo-500/10 border border-indigo-500/20 flex items-center justify-center text-indigo-400")
                MdiTelevisionPlay(class="w-4 h-4")
              div
                h2(class="text-sm font-bold text-themeText") Streaming Overlay & OBS Server
                p(class="text-[11px] text-themeText2") Render streaming canvas, generate standalone HTML, and push to OBS
            div(v-if="overlayStatus" class="px-2.5 py-0.5 rounded-full text-[11px] font-semibold bg-indigo-500/10 text-indigo-400 border border-indigo-500/20")
              span {{ overlayStatus.elements_count }} elements

          //- Live Status Pill Box
          div(v-if="overlayStatus" class="p-3 rounded-xl bg-themeBackground border border-themeBorder text-xs text-themeText2 flex flex-wrap items-center justify-between gap-2")
            div(class="flex items-center gap-2")
              span(class="w-2 h-2 rounded-full bg-emerald-400")
              span(class="font-semibold text-themeText") {{ overlayStatus.design_name || 'Default Canvas' }}
            div(class="flex items-center gap-3 text-xs text-themeText3")
              span {{ overlayStatus.canvas_size?.width }}×{{ overlayStatus.canvas_size?.height }}
              span {{ overlayStatus.active ? '🟢 Active' : '⚪ Standby' }}

          //- Actions Grid
          div(class="grid grid-cols-1 sm:grid-cols-2 gap-2.5")
            button(
              @click="testInitializeOverlay"
              :disabled="overlayLoading"
              class="px-3.5 py-2.5 rounded-xl bg-themeBackground hover:bg-themeBackground3 disabled:opacity-50 text-themeText border border-themeBorder text-xs font-semibold transition-all flex items-center gap-2 cursor-pointer shadow-2xs active:scale-95"
            )
              MdiTelevisionPlay(class="w-4 h-4 text-blue-400")
              span Initialize Overlay

            button(
              @click="testGenerateHTML"
              :disabled="overlayLoading"
              class="px-3.5 py-2.5 rounded-xl bg-themeBackground hover:bg-themeBackground3 disabled:opacity-50 text-themeText border border-themeBorder text-xs font-semibold transition-all flex items-center gap-2 cursor-pointer shadow-2xs active:scale-95"
            )
              MdiFileDocumentOutline(class="w-4 h-4 text-emerald-400")
              span Generate HTML

            button(
              @click="testStartOverlayServer"
              :disabled="overlayLoading"
              class="px-3.5 py-2.5 rounded-xl bg-themeBackground hover:bg-themeBackground3 disabled:opacity-50 text-themeText border border-themeBorder text-xs font-semibold transition-all flex items-center gap-2 cursor-pointer shadow-2xs active:scale-95"
            )
              MdiRefresh(class="w-4 h-4 text-amber-400")
              span Start Overlay Server

            button(
              @click="testUpdateObsSource"
              :disabled="overlayLoading"
              class="px-3.5 py-2.5 rounded-xl bg-themeBackground hover:bg-themeBackground3 disabled:opacity-50 text-themeText border border-themeBorder text-xs font-semibold transition-all flex items-center gap-2 cursor-pointer shadow-2xs active:scale-95"
            )
              MdiPlay(class="w-4 h-4 text-rose-400")
              span Send to OBS

            button(
              @click="testAddElement"
              :disabled="overlayLoading"
              class="px-3.5 py-2.5 rounded-xl bg-themeBackground hover:bg-themeBackground3 disabled:opacity-50 text-themeText border border-themeBorder text-xs font-semibold transition-all flex items-center gap-2 cursor-pointer shadow-2xs active:scale-95"
            )
              MdiPuzzle(class="w-4 h-4 text-purple-400")
              span Add Test Element

            button(
              @click="testTriggerOverlayAd"
              :disabled="overlayLoading"
              class="px-3.5 py-2.5 rounded-xl bg-themeBackground hover:bg-themeBackground3 disabled:opacity-50 text-themeText border border-themeBorder text-xs font-semibold transition-all flex items-center gap-2 cursor-pointer shadow-2xs active:scale-95"
            )
              MdiBullhorn(class="w-4 h-4 text-orange-400")
              span Test Overlay Ad

        //- Output Viewer
        div(v-if="overlayResult" class="mt-4 pt-3 border-t border-themeBorder")
          div(class="flex items-center justify-between mb-2")
            div(class="flex items-center gap-1.5 text-xs text-themeText2")
              MdiLightningBolt(class="w-3.5 h-3.5 text-amber-400")
              span(class="font-mono") {{ overlayExecutionTime ? `⚡ ${overlayExecutionTime}ms` : 'Overlay Output' }}
            button(
              @click="copyText(overlayResult, 'Overlay result copied!')"
              class="px-2 py-1 rounded-md text-[11px] bg-themeBackground hover:bg-themeBackground3 text-themeText2 hover:text-themeText transition-colors flex items-center gap-1 cursor-pointer"
            )
              MdiContentCopy(class="w-3 h-3")
              span Copy
          div(class="p-3 rounded-xl bg-themeBackground border border-themeBorder font-mono text-xs text-themeText whitespace-pre-wrap max-h-40 overflow-y-auto select-all leading-relaxed")
            | {{ overlayResult }}

      //- 4. System & Runtime Environment Specs
      div(v-show="activeTab === 'all' || activeTab === 'system'" class="flex flex-col justify-between p-6 rounded-2xl bg-themeBackground2 border border-themeBorder hover:border-themeBorder2 transition-all shadow-xs space-y-4")
        div(class="space-y-4")
          div(class="flex items-center justify-between border-b border-themeBorder pb-3")
            div(class="flex items-center gap-2.5")
              div(class="w-8 h-8 rounded-xl bg-gray-500/10 border border-gray-500/20 flex items-center justify-center text-themeText")
                MdiInformation(class="w-4 h-4")
              div
                h2(class="text-sm font-bold text-themeText") Runtime Diagnostics
                p(class="text-[11px] text-themeText2") Host environment, server URL, and client resolution

            button(
              @click="copyText(systemInfo, 'System info copied!')"
              class="px-3 py-1.5 rounded-xl bg-themeBackground hover:bg-themeBackground3 text-themeText border border-themeBorder text-xs font-medium transition-colors flex items-center gap-1.5 cursor-pointer shadow-2xs"
            )
              MdiContentCopy(class="w-3.5 h-3.5")
              span Copy

          div(class="p-3.5 rounded-xl bg-themeBackground border border-themeBorder font-mono text-xs text-themeText whitespace-pre-wrap max-h-56 overflow-y-auto select-all leading-relaxed")
            | {{ systemInfo || 'Loading system diagnostics...' }}

    //- 5. Live Events Stream Monitor (Full Width)
    div(v-show="activeTab === 'all' || activeTab === 'events'" class="flex flex-col p-6 rounded-2xl bg-themeBackground2 border border-themeBorder hover:border-themeBorder2 transition-all shadow-xs space-y-4")
      div(class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-themeBorder pb-4")
        div(class="flex items-center gap-2.5")
          div(class="w-9 h-9 rounded-xl bg-emerald-500/10 border border-emerald-500/20 flex items-center justify-center text-emerald-400")
            MdiRadioTower(class="w-5 h-5")
          div
            div(class="flex items-center gap-2")
              h2(class="text-base font-bold text-themeText") Live IPC & Plugin Events Monitor
              div(class="flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-semibold bg-emerald-500/10 text-emerald-400 border border-emerald-500/20")
                span(class="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse")
                span Live Stream
            p(class="text-xs text-themeText2") Real-time event stream capturing plugin lifecycle and background triggers

        div(class="flex flex-wrap items-center gap-2")
          //- Search within events
          div(class="relative")
            input(
              v-model="eventFilterQuery"
              type="text"
              placeholder="Filter events..."
              class="w-40 sm:w-48 pl-7 pr-3 py-1.5 bg-themeBackground border border-themeBorder rounded-xl text-xs text-themeText placeholder:text-themeText3 focus:outline-none focus:border-themePrimary shadow-2xs"
            )
            MdiTextBoxSearch(class="w-3.5 h-3.5 text-themeText3 absolute left-2 top-2")

          button(
            @click="isListeningPaused = !isListeningPaused"
            :class="isListeningPaused ? 'bg-amber-500/10 text-amber-400 border-amber-500/20' : 'bg-themeBackground text-themeText border-themeBorder'"
            class="px-3 py-1.5 rounded-xl border hover:bg-themeBackground3 text-xs font-medium transition-all flex items-center gap-1 cursor-pointer shadow-2xs"
          )
            component(:is="isListeningPaused ? MdiPlay : MdiPause" class="w-3.5 h-3.5")
            span {{ isListeningPaused ? 'Resume' : 'Pause' }}

          button(
            @click="clearPluginEvents"
            class="px-3 py-1.5 rounded-xl bg-themeBackground hover:bg-themeBackground3 text-themeText border border-themeBorder text-xs font-medium transition-all flex items-center gap-1 cursor-pointer shadow-2xs"
          )
            MdiTrashCan(class="w-3.5 h-3.5 text-red-400")
            span Clear

          button(
            @click="exportEventsJson"
            :disabled="pluginEvents.length === 0"
            class="px-3 py-1.5 rounded-xl bg-themeBackground hover:bg-themeBackground3 disabled:opacity-40 text-themeText border border-themeBorder text-xs font-medium transition-all flex items-center gap-1 cursor-pointer shadow-2xs"
          )
            MdiDownload(class="w-3.5 h-3.5 text-themePrimary")
            span Export JSON

      //- Events Stream Box
      div(class="p-4 rounded-xl bg-themeBackground border border-themeBorder min-h-[12rem] max-h-80 overflow-y-auto font-mono text-xs space-y-2")
        div(v-if="filteredPluginEvents.length === 0" class="flex flex-col items-center justify-center h-40 text-center space-y-2")
          div(class="w-10 h-10 rounded-xl bg-themeBackground2 border border-themeBorder flex items-center justify-center text-themeText3")
            MdiRadioTower(class="w-5 h-5")
          p(class="text-xs font-semibold text-themeText") No Events Captured Yet
          p(class="text-[11px] text-themeText3 max-w-xs") Plugin additions, updates, or background reload triggers will stream live here.

        div(
          v-for="(event, index) in filteredPluginEvents"
          :key="index"
          class="flex items-start gap-3 py-1.5 px-2.5 rounded-lg bg-themeBackground2/50 border border-themeBorder/40 hover:bg-themeBackground2 transition-colors"
        )
          span(class="text-[11px] text-themeText3 shrink-0 mt-0.5") {{ event.timestamp }}
          span(
            :class="getEventTypeBadgeClass(event.type)"
            class="px-2 py-0.5 rounded text-[10px] font-bold uppercase shrink-0"
          ) {{ event.type }}
          span(class="text-xs text-themeText flex-1 break-all") {{ event.message }}

  //- Onboarding Instruction Modal
  ModalInstruction(
    :isOpen="showOnboardingModal"
    qrCodeUrl="download_client_qr.png"
    @close="closeOnboardingModal"
    @updatePreference="closeOnboardingModal"
  )
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useOverlayManager } from "../../desktop-streaming/composables/overlay/useOverlayManager";
import ModalInstruction from "../components/Modal/Instruction.vue";

// Icons via unplugin-icons
import MdiBug from "~icons/mdi/bug";
import MdiShieldKey from "~icons/mdi/shield-key-outline";
import MdiPuzzle from "~icons/mdi/puzzle-outline";
import MdiTelevisionPlay from "~icons/mdi/television-play";
import MdiBullhorn from "~icons/mdi/bullhorn-outline";
import MdiRadioTower from "~icons/mdi/radio-tower";
import MdiInformation from "~icons/mdi/information-outline";
import MdiTrashCan from "~icons/mdi/trash-can-outline";
import MdiContentCopy from "~icons/mdi/content-copy";
import MdiCheck from "~icons/mdi/check";
import MdiRefresh from "~icons/mdi/refresh";
import MdiPlay from "~icons/mdi/play";
import MdiPause from "~icons/mdi/pause";
import MdiDownload from "~icons/mdi/download";
import MdiQrcode from "~icons/mdi/qrcode";
import MdiTextBoxSearch from "~icons/mdi/text-box-search-outline";
import MdiLightningBolt from "~icons/mdi/lightning-bolt";
import MdiFileDocumentOutline from "~icons/mdi/file-document-outline";
import SolarArrowLeftLinear from "~icons/solar/arrow-left-linear?width=24px&height=24px";

definePageMeta({
  title: "Debug Console",
  description: "Developer testing environment for VortiDeck",
  contentPaddingPreset: "compact",
});

// Dev environment detection
const isDev = process.env.NODE_ENV === "development" || import.meta.dev;
const devBypass = ref(true);

// Navigation / Tabs
const activeTab = ref("all");
const navigationTabs = [
  { id: "all", label: "All Suites", icon: MdiBug },
  { id: "auth", label: "Auth & JWT", icon: MdiShieldKey },
  { id: "plugins", label: "Plugins", icon: MdiPuzzle },
  { id: "overlay", label: "Overlay & OBS", icon: MdiTelevisionPlay },
  { id: "events", label: "Live Events", icon: MdiRadioTower },
  { id: "system", label: "System Specs", icon: MdiInformation },
];

// Reactive outputs
const jwtTestResult = ref("");
const jwtExecutionTime = ref<number | null>(null);
const activeAuthTier = ref("Guest / Default");

const pluginTestResult = ref("");
const pluginExecutionTime = ref<number | null>(null);

const systemInfo = ref("");
const isRefreshingSystem = ref(false);
const serverInfo = ref<{ ip: string; port: number } | null>(null);

const toastMessage = ref("");
const showOnboardingModal = ref(false);

// Live Event Monitor
interface PluginEventItem {
  timestamp: string;
  type: string;
  message: string;
}
const pluginEvents = ref<PluginEventItem[]>([]);
const eventFilterQuery = ref("");
const isListeningPaused = ref(false);

const filteredPluginEvents = computed(() => {
  if (!eventFilterQuery.value.trim()) return pluginEvents.value;
  const q = eventFilterQuery.value.toLowerCase();
  return pluginEvents.value.filter(
    (e) =>
      e.message.toLowerCase().includes(q) || e.type.toLowerCase().includes(q)
  );
});

// Modern Overlay System Hook
const {
  initialize: initializeOverlay,
  generateOverlayHtml,
  startServer: startOverlayServer,
  sendToObs: sendOverlayToObs,
  loading: overlayLoading,
  overlayState,
} = useOverlayManager();
const debugOverlaySync = useOverlayV3Sync();

const overlayResult = ref("");
const overlayExecutionTime = ref<number | null>(null);

let unlistenPluginChanged: (() => void) | null = null;

// Helpers
const showToast = (msg: string) => {
  toastMessage.value = msg;
  setTimeout(() => {
    if (toastMessage.value === msg) toastMessage.value = "";
  }, 3500);
};

const copyText = async (text: string, successMsg = "Copied to clipboard!") => {
  try {
    await navigator.clipboard.writeText(text);
    showToast(successMsg);
  } catch (err) {
    showToast("❌ Could not copy to clipboard");
  }
};

const getEventTypeBadgeClass = (type: string) => {
  const t = type.toLowerCase();
  if (t === "added" || t === "install")
    return "bg-emerald-500/10 text-emerald-400 border border-emerald-500/20";
  if (t === "removed" || t === "uninstall")
    return "bg-red-500/10 text-red-400 border border-red-500/20";
  if (t === "updated" || t === "reload")
    return "bg-blue-500/10 text-blue-400 border border-blue-500/20";
  return "bg-amber-500/10 text-amber-400 border border-amber-500/20";
};

// Lifecycle
onMounted(async () => {
  await getSystemInfo();

  try {
    unlistenPluginChanged = await listen("plugin-changed", (event) => {
      if (isListeningPaused.value) return;

      const timestamp = new Date().toLocaleTimeString();
      const payload = event.payload as any;

      pluginEvents.value.unshift({
        timestamp,
        type: payload.event_type || "updated",
        message: `${payload.plugin_path || "Plugin"} ${
          payload.plugin_id ? `(${payload.plugin_id})` : ""
        }`,
      });

      if (pluginEvents.value.length > 100) {
        pluginEvents.value = pluginEvents.value.slice(0, 100);
      }
    });
  } catch (error) {
    console.error("Failed to setup plugin events listener:", error);
  }
});

onUnmounted(() => {
  if (unlistenPluginChanged) {
    unlistenPluginChanged();
  }
});

// Modal Actions
const openOnboardingModal = () => {
  showOnboardingModal.value = true;
};
const closeOnboardingModal = () => {
  showOnboardingModal.value = false;
};

const openLogsWindow = async () => {
  try {
    await invoke("open_logs");
    showToast("Opened System Logs window");
  } catch (e) {
    showToast(`Error opening logs: ${e}`);
  }
};

const openSimulatorWindow = async () => {
  try {
    await invoke("open_mobile_simulator");
    showToast("Opened Mobile Simulator");
  } catch (e) {
    showToast(`Error opening simulator: ${e}`);
  }
};

// --- JWT Testing Functions ---
const testGuestMode = async () => {
  const start = performance.now();
  try {
    await invoke("logout_user");
    activeAuthTier.value = "Guest Mode";

    setTimeout(async () => {
      try {
        const result = (await invoke("can_install_plugin")) as {
          can_install: boolean;
          message: string;
        };
        jwtExecutionTime.value = Math.round(performance.now() - start);
        jwtTestResult.value = `🎭 GUEST MODE VERIFIED:\n• Allowed to install: ${
          result.can_install ? "YES" : "NO (Blocked as expected)"
        }\n• Backend Response: ${result.message}`;
      } catch (error) {
        jwtTestResult.value = `🎭 Guest Mode Test Error: ${error}`;
      }
    }, 400);

    showToast("Activated Guest Mode");
  } catch (error) {
    jwtTestResult.value = `❌ Failed to enter guest mode: ${error}`;
  }
};

const checkPluginValidation = async () => {
  const start = performance.now();
  try {
    const result = (await invoke("can_install_plugin")) as {
      can_install: boolean;
      message: string;
    };
    jwtExecutionTime.value = Math.round(performance.now() - start);
    jwtTestResult.value = `🔍 Plugin Permission Check:\n• Can Install: ${
      result.can_install ? "ALLOWED ✅" : "BLOCKED ❌"
    }\n• Message: ${result.message}`;
  } catch (error) {
    jwtExecutionTime.value = Math.round(performance.now() - start);
    jwtTestResult.value = `❌ Validation check error: ${error}`;
  }
};

// --- Plugin System Functions ---
const getInstalledPlugins = async () => {
  const start = performance.now();
  try {
    const result = await invoke("get_installed_plugins");
    pluginExecutionTime.value = Math.round(performance.now() - start);
    pluginTestResult.value = JSON.stringify(result, null, 2);
    showToast("Retrieved installed plugins");
  } catch (error) {
    pluginExecutionTime.value = Math.round(performance.now() - start);
    pluginTestResult.value = `❌ Error getting plugins: ${error}`;
  }
};

const refreshPlugins = async () => {
  const start = performance.now();
  try {
    const result = await invoke("refresh_plugins");
    pluginExecutionTime.value = Math.round(performance.now() - start);
    pluginTestResult.value = `✅ Refreshed Plugins:\n${JSON.stringify(
      result,
      null,
      2
    )}`;
    showToast("Plugins refreshed successfully");
  } catch (error) {
    pluginExecutionTime.value = Math.round(performance.now() - start);
    pluginTestResult.value = `❌ Error refreshing plugins: ${error}`;
  }
};

const reloadPluginsWithAuth = async () => {
  const start = performance.now();
  try {
    const result = await invoke("reload_plugins_with_auth_check");
    pluginExecutionTime.value = Math.round(performance.now() - start);
    pluginTestResult.value = `🔄 Auth-Aware Plugin Reload Result:\n${JSON.stringify(
      result,
      null,
      2
    )}`;
    showToast("Reloaded plugins with auth check");
  } catch (error) {
    pluginExecutionTime.value = Math.round(performance.now() - start);
    pluginTestResult.value = `❌ Auth reload error: ${error}`;
  }
};

const checkPluginLimits = async () => {
  const start = performance.now();
  try {
    const result = await invoke("can_install_plugin");
    pluginExecutionTime.value = Math.round(performance.now() - start);
    pluginTestResult.value = `🎯 Plugin Limits Status:\n${JSON.stringify(
      result,
      null,
      2
    )}`;
  } catch (error) {
    pluginExecutionTime.value = Math.round(performance.now() - start);
    pluginTestResult.value = `❌ Limits check error: ${error}`;
  }
};

const getPluginsDirectory = async () => {
  const start = performance.now();
  try {
    const result = await invoke("get_plugins_directory");
    pluginExecutionTime.value = Math.round(performance.now() - start);
    pluginTestResult.value = `📂 Plugins Storage Directory:\n${result}`;
  } catch (error) {
    pluginExecutionTime.value = Math.round(performance.now() - start);
    pluginTestResult.value = `❌ Directory fetch error: ${error}`;
  }
};

const debugPluginSubsystem = async () => {
  const start = performance.now();
  try {
    const result = await invoke("debug_plugin_system");
    pluginExecutionTime.value = Math.round(performance.now() - start);
    pluginTestResult.value = `🧪 Subsystem Debug Report:\n${JSON.stringify(
      result,
      null,
      2
    )}`;
  } catch (error) {
    pluginExecutionTime.value = Math.round(performance.now() - start);
    pluginTestResult.value = `❌ Subsystem debug error: ${error}`;
  }
};

// --- Overlay Testing Functions ---
const testInitializeOverlay = async () => {
  const start = performance.now();
  try {
    overlayResult.value = "🎨 Initializing overlay system...";
    await initializeOverlay();
    overlayExecutionTime.value = Math.round(performance.now() - start);
    overlayResult.value = `✅ Overlay System Initialized:\n${JSON.stringify(
      overlayState.value,
      null,
      2
    )}`;
    showToast("Overlay initialized");
  } catch (error) {
    overlayExecutionTime.value = Math.round(performance.now() - start);
    overlayResult.value = `❌ Overlay initialization error: ${error}`;
  }
};

const testGenerateHTML = async () => {
  const start = performance.now();
  try {
    overlayResult.value = "📄 Generating standalone HTML payload...";
    const html = await generateOverlayHtml();
    overlayExecutionTime.value = Math.round(performance.now() - start);
    overlayResult.value = `✅ Generated HTML (${html.length} chars):\n${html.substring(
      0,
      350
    )}...`;
    showToast("Generated overlay HTML");
  } catch (error) {
    overlayExecutionTime.value = Math.round(performance.now() - start);
    overlayResult.value = `❌ HTML Generation error: ${error}`;
  }
};

const testStartOverlayServer = async () => {
  const start = performance.now();
  try {
    overlayResult.value = "🌐 Starting HTTP Overlay Server...";
    const url = await startOverlayServer();
    overlayExecutionTime.value = Math.round(performance.now() - start);
    overlayResult.value = `✅ Overlay Server Running!\n🔗 OBS Browser Source URL: ${url}`;
    showToast("Overlay server started");
  } catch (error) {
    overlayExecutionTime.value = Math.round(performance.now() - start);
    overlayResult.value = `❌ Server start error: ${error}`;
  }
};

const testUpdateObsSource = async () => {
  const start = performance.now();
  try {
    overlayResult.value = "📺 Sending canvas payload to OBS...";
    await sendOverlayToObs();
    overlayExecutionTime.value = Math.round(performance.now() - start);
    overlayResult.value = `✅ Canvas layout broadcast to OBS browser source`;
    showToast("Sent overlay to OBS");
  } catch (error) {
    overlayExecutionTime.value = Math.round(performance.now() - start);
    overlayResult.value = `❌ OBS broadcast error: ${error}`;
  }
};

const testAddElement = async () => {
  const start = performance.now();
  try {
    overlayResult.value = "➕ Appending test streaming widget...";
    const newEl: any = {
      id: `test-${Date.now()}`,
      name: "Test Alert",
      type: "alert",
      x: 100,
      y: 100,
      width: 300,
      height: 100,
      visible: true,
      data: { text: "Hello VortiDeck Live!" },
      style: { background: "rgba(0,0,0,0.8)", color: "#fff" }
    };
    await debugOverlaySync.syncElementAdd(newEl);
    overlayExecutionTime.value = Math.round(performance.now() - start);
    overlayResult.value = `✅ Test streaming element added to active canvas (${newEl.id})`;
    showToast("Test element added");
  } catch (error) {
    overlayExecutionTime.value = Math.round(performance.now() - start);
    overlayResult.value = `❌ Add element error: ${error}`;
  }
};

const testTriggerOverlayAd = async () => {
  const start = performance.now();
  try {
    overlayResult.value = "🎬 Triggering test advertisement overlay...";
    await invoke("obs_banner_set_data", {
      bannerData: {
        text: "⚡ Special Partner Sponsor: VortiDeck Pro",
        sponsor: "VortiDeck",
        logoUrl: "/icon.png",
        durationSecs: 15,
        theme: "cyberpunk"
      }
    });
    overlayExecutionTime.value = Math.round(performance.now() - start);
    overlayResult.value = `✅ Test sponsor banner broadcast triggered to OBS`;
    showToast("Overlay test ad triggered");
  } catch (error) {
    overlayExecutionTime.value = Math.round(performance.now() - start);
    overlayResult.value = `❌ Trigger ad error: ${error}`;
  }
};

// --- System Info & Diagnostics ---
const getSystemInfo = async () => {
  isRefreshingSystem.value = true;
  try {
    let serverData: any = null;
    try {
      serverData = await invoke("get_server_info_new");
      serverInfo.value = serverData;
    } catch (e) {
      console.warn("Could not get server info:", e);
    }

    const info = {
      environment: isDev ? "development" : "production",
      timestamp: new Date().toISOString(),
      url: window.location.href,
      userAgent: navigator.userAgent,
      screenResolution: `${window.screen.width}x${window.screen.height}`,
      devicePixelRatio: window.devicePixelRatio,
      activeServer: serverData
        ? `http://${serverData.ip}:${serverData.port}`
        : "Standby / Not running",
    };

    systemInfo.value = JSON.stringify(info, null, 2);
  } catch (error) {
    systemInfo.value = `❌ Error retrieving system info: ${error}`;
  } finally {
    isRefreshingSystem.value = false;
  }
};

// Clear All Outputs
const clearAllResults = () => {
  jwtTestResult.value = "";
  pluginTestResult.value = "";
  overlayResult.value = "";
  showToast("All diagnostic results cleared");
};

const clearPluginEvents = () => {
  pluginEvents.value = [];
  showToast("Live events log cleared");
};

const exportEventsJson = () => {
  const data = JSON.stringify(pluginEvents.value, null, 2);
  const blob = new Blob([data], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `vortideck_events_${Date.now()}.json`;
  a.click();
  URL.revokeObjectURL(url);
  showToast("Exported events to JSON");
};
</script>

<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
