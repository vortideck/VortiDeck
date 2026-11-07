<template lang="pug">
Teleport(to="body" :disabled="isPage")
  Transition(name="fade")
    div(
      v-if="!isPage && settingsModal.isOpen.value"
      class="vd-modal-overlay !z-[50000]"
      @mousedown.self="handleBackdropClick"
    )
      // Top 30px titlebar drag region over the backdrop
      div(
        class="absolute top-0 left-0 w-[calc(100vw-220px)] h-[30px] z-10 pointer-events-auto bg-transparent"
        data-tauri-drag-region
      )

  Transition(:name="isPage ? '' : 'jump'")
    div(
      v-if="settingsModal.isOpen.value || isPage"
      :class="isPage ? 'relative w-full h-full !z-auto' : 'fixed inset-0 z-[50005] flex items-center justify-center p-3 md:p-6 pointer-events-none'"
    )
      div(
        class="relative w-full max-w-5xl h-[88vh] max-h-[820px] bg-themeBackground rounded-2xl border border-themeBorder shadow-2xl overflow-hidden flex flex-col pointer-events-auto"
        :class="isPage ? '!h-full !max-h-none !max-w-none !border-none !shadow-none !rounded-none' : ''"
      )
        //- Modal Top Header Bar
        div(
          class="flex items-center justify-between px-5 py-3 border-b border-themeBorder bg-themeBackground2/70 select-none"
          data-tauri-drag-region
        )
          div(class="flex items-center gap-2.5 pointer-events-none" data-tauri-drag-region)
            HugeiconsSettings01(class="w-4 h-4 text-themePrimary")
            span(class="text-xs font-bold text-themeText tracking-tight") Settings
            span(class="text-[11px] text-themeText3 font-normal") — System & workspace preferences
          
          div(class="flex items-center gap-2 pointer-events-auto titlebar-no-drag" v-if="!isPage")
            span(class="text-[10px] font-mono text-themeText3 bg-themeBackground px-2 py-0.5 rounded-md border border-themeBorder hidden sm:inline-block") ESC
            button(
              type="button"
              @click="settingsModal.closeSettings()"
              class="w-6 h-6 rounded-lg text-themeText3 hover:text-themeText hover:bg-themeBackground border border-transparent hover:border-themeBorder transition-colors flex items-center justify-center cursor-pointer text-xs"
              title="Close (Esc)"
            ) ✕

        //- Master-Detail Body
        div(class="flex-1 flex min-h-0 overflow-hidden")
          //- Left Navigation Sidebar (Hermes-Style)
          div(class="w-52 shrink-0 flex flex-col justify-between bg-themeBackground2/40 p-3 border-r border-themeBorder overflow-y-auto")
            nav(class="flex flex-col gap-y-1")
              button(
                v-for="tab in navTabs"
                :key="tab.id"
                type="button"
                @click="settingsModal.activeTab.value = tab.id"
                :class="settingsModal.activeTab.value === tab.id ? 'bg-themePrimary/15 text-themePrimary font-bold border border-themePrimary/25 shadow-2xs' : 'text-themeText2 hover:text-themeText hover:bg-themeBackground2 border border-transparent'"
                class="flex items-center gap-2.5 px-3 py-2 rounded-xl text-xs transition-all text-left cursor-pointer"
              )
                component(:is="tab.icon" class="w-4 h-4 shrink-0")
                span {{ tab.label }}

            div(class="pt-3 border-t border-themeBorder px-2 flex items-center justify-between text-[11px] text-themeText3")
              span VortiDeck v{{ appVersion }}
              button(@click="openPath('https://vortideck.com')" class="hover:text-themePrimary transition-colors" title="Documentation") Docs

          //- Right Content Panel
          div(class="flex-1 min-w-0 bg-themeBackground overflow-y-auto p-6 settings-content-panel")
      
            //- 1. Appearance Tab
            div(v-if="activeNavTab === 'appearance'" class="space-y-6")
              div(class="border-b border-themeBorder pb-4")
                h2(class="text-base font-bold text-themeText") Appearance
                p(class="text-xs text-themeText2 mt-0.5") Customize color themes, desktop palettes, scale, and language.

              //- Hermes-Style Appearance & Theme Section
              div(class="flex flex-col gap-y-3.5")
                div(class="flex items-start justify-between")
                  div
                    h3(class="text-sm font-semibold text-themeText") Color Mode
                    p(class="text-xs text-themeText2 mt-0.5") The selected mode is applied on top of the active palette.
            
                  //- Segmented Light / Dark / System Mode Switcher
                  div(class="flex items-center bg-themeBackground p-1 rounded-xl border border-themeBorder shadow-2xs")
                    button(
                      type="button"
                      @click="mainStore.setColorMode('light')"
                      :class="mainStore.getColorMode === 'light' ? 'bg-themeBackground2 text-themeText shadow-xs font-semibold' : 'text-themeText2 hover:text-themeText'"
                      class="px-3 py-1.5 rounded-lg text-xs transition-all flex items-center gap-1.5 cursor-pointer"
                    )
                      span ☀️
                      span Light
                    button(
                      type="button"
                      @click="mainStore.setColorMode('dark')"
                      :class="mainStore.getColorMode === 'dark' ? 'bg-themeBackground2 text-themeText shadow-xs font-semibold' : 'text-themeText2 hover:text-themeText'"
                      class="px-3 py-1.5 rounded-lg text-xs transition-all flex items-center gap-1.5 cursor-pointer"
                    )
                      span 🌙
                      span Dark
                    button(
                      type="button"
                      @click="mainStore.setColorMode('system')"
                      :class="mainStore.getColorMode === 'system' ? 'bg-themeBackground2 text-themeText shadow-xs font-semibold' : 'text-themeText2 hover:text-themeText'"
                      class="px-3 py-1.5 rounded-lg text-xs transition-all flex items-center gap-1.5 cursor-pointer"
                    )
                      span 💻
                      span System

                //- Palette Search Bar
                div(class="relative pt-2")
                  input(
                    type="text"
                    v-model="paletteSearch"
                    placeholder="Search your themes or custom palettes..."
                    class="w-full bg-themeBackground border border-themeBorder text-themeText placeholder-themeText3 text-xs rounded-xl px-3.5 py-2 focus:outline-none focus:ring-1 focus:ring-themeButton focus:border-themeButton transition-all"
                  )

                //- 2-Column Palette Cards Grid
                div(class="grid grid-cols-1 md:grid-cols-2 gap-3 pt-1")
                  button(
                    v-for="pal in filteredPalettes"
                    :key="pal.id"
                    type="button"
                    @click="mainStore.setThemePalette(pal.id)"
                    :class="mainStore.getThemePalette === pal.id ? 'border-themeButton ring-2 ring-themeButton/20 bg-themeBackground' : 'border-themeBorder hover:border-themeBorder2 bg-themeBackground'"
                    class="group p-3 rounded-xl border text-left transition-all cursor-pointer flex flex-col gap-2.5 relative shadow-2xs hover:shadow-xs"
                  )
                    //- Miniature Hermes-style Window Preview Box
                    div(
                      class="w-full h-16 rounded-lg p-2.5 flex flex-col justify-between border overflow-hidden transition-colors"
                      :style="mainStore.getColorMode === 'light' ? { backgroundColor: pal.lightPreview.bg, borderColor: pal.lightPreview.border } : { backgroundColor: pal.darkPreview.bg, borderColor: pal.darkPreview.border }"
                    )
                      div(class="flex items-center justify-between")
                        div(class="flex items-center gap-2")
                          div(
                            class="w-2 h-7 rounded-sm"
                            :style="mainStore.getColorMode === 'light' ? { backgroundColor: pal.lightPreview.sidebar } : { backgroundColor: pal.darkPreview.sidebar }"
                          )
                          div(class="flex flex-col gap-1")
                            div(
                              class="w-14 h-2 rounded-full"
                              :style="mainStore.getColorMode === 'light' ? { backgroundColor: pal.lightPreview.line } : { backgroundColor: pal.darkPreview.line }"
                            )
                            div(
                              class="w-20 h-1.5 rounded-full opacity-60"
                              :style="mainStore.getColorMode === 'light' ? { backgroundColor: pal.lightPreview.line } : { backgroundColor: pal.darkPreview.line }"
                            )
                        div(
                          class="w-8 h-4 rounded-full"
                          :style="mainStore.getColorMode === 'light' ? { backgroundColor: pal.lightPreview.accent } : { backgroundColor: pal.darkPreview.accent }"
                        )
              
                    div(class="flex items-start justify-between pr-1")
                      div
                        div(class="text-xs font-semibold text-themeText") {{ pal.name }}
                        div(class="text-[11px] text-themeText2 mt-0.5 line-clamp-1") {{ pal.description }}
                      div(v-if="mainStore.getThemePalette === pal.id" class="w-4 h-4 rounded-full bg-themeButton text-white flex items-center justify-center text-[10px] font-bold shrink-0 mt-0.5") ✓

              //- UI Scale & Language Inset Card
              div(class="bg-themeBackground p-1.5 rounded-xl border border-themeBorder flex flex-col divide-y divide-themeBorder/40")
                div(class="flex justify-between items-center p-3 px-3.5")
                  div
                    h3(class="text-xs font-semibold text-themeText") UI Scale
                    p(class="text-[11px] text-themeText2 mt-0.5") Adjust application layout scale (Ctrl +/-/0)
                  div
                    select(
                      :value="appScale.scale.value"
                      @change="(e: any) => appScale.setScale(parseFloat(e.target.value))"
                      class="bg-themeBackground2 border border-themeBorder text-themeText text-xs rounded-lg focus:ring-1 focus:ring-themeButton focus:border-themeButton block p-1.5 px-2.5 transition-colors cursor-pointer"
                    )
                      option(v-for="preset in appScale.presets" :key="preset.value" :value="preset.value") {{ preset.label }}
          
                div(class="flex justify-between items-center p-3 px-3.5 gap-4")
                  div(class="w-1/3 shrink-0")
                    h3(class="text-xs font-semibold text-themeText") Language
                    p(class="text-[11px] text-themeText2 mt-0.5") Select your preferred language
                  div(class="w-2/3")
                    Multiselect(
                      v-model="settings.language"
                      @update:modelValue="changeLanguage"
                      :options="languages"
                      :searchable="true"
                      :multiple="false"
                      :close-on-select="true"
                      :show-labels="false"
                      :allow-empty="false"
                      track-by="code"
                      label="name"
                      placeholder="Select a language"
                    )

            //- 2. General Tab
            div(v-else-if="activeNavTab === 'general'" class="space-y-6")
              div(class="border-b border-themeBorder pb-4")
                h2(class="text-base font-bold text-themeText") General Settings
                p(class="text-xs text-themeText2 mt-0.5") Startup behavior, notifications, and privacy options.

              //- Grouped Inset Card
              div(class="bg-themeBackground p-1.5 rounded-xl border border-themeBorder flex flex-col divide-y divide-themeBorder/40")
                div(class="flex justify-between items-center p-3 px-3.5")
                  div
                    h3(class="text-xs font-semibold text-themeText") Launch on system startup
                    p(class="text-[11px] text-themeText2 mt-0.5") {{ autostartSupported ? 'Automatically start VortiDeck when you log in' : 'Available in packaged builds; disabled during development' }}
                  div
                    label(class="relative inline-flex items-center" :class="autostartSupported ? 'cursor-pointer' : 'cursor-not-allowed opacity-50'")
                      input(type="checkbox" :checked="settings.autostart" :disabled="!autostartSupported" class="sr-only peer" @change="toggleAutostart")
                      div(class="settings-toggle-track")

                div(class="flex justify-between items-center gap-4 p-3 px-3.5")
                  div(class="min-w-0")
                    h3(class="text-xs font-semibold text-themeText") Workflow approval presentation
                    p(class="text-[11px] text-themeText2 mt-0.5") Choose whether approval requests notify you or interrupt with a dialog.
                  select(
                    v-model="workflowApprovalPresentation"
                    class="shrink-0 rounded-lg border border-themeBorder bg-themeBackground2 px-2.5 py-1.5 text-xs text-themeText"
                  )
                    option(value="notification") Notification and badge
                    option(value="dialog") Open approval dialog

                div(class="flex justify-between items-center p-3 px-3.5")
                  div
                    h3(class="text-xs font-semibold text-themeText") Native approval notifications
                    p(class="text-[11px] text-themeText2 mt-0.5") Use Windows notifications when every VortiDeck window is unfocused.
                  div
                    label(class="relative inline-flex items-center cursor-pointer")
                      input(
                        type="checkbox"
                        :checked="mainStore.getNativeWorkflowApprovalNotifications"
                        class="sr-only peer"
                        @change="toggleNativeApprovalNotifications"
                      )
                      div(class="settings-toggle-track")

                div(class="flex justify-between items-center p-3 px-3.5")
                  div
                    h3(class="text-xs font-semibold text-themeText") Anonymous Usage Analytics
                    p(class="text-[11px] text-themeText2 mt-0.5") Help improve VortiDeck by sending anonymous usage data
                  div
                    label(class="relative inline-flex items-center cursor-pointer")
                      input(type="checkbox" v-model="settings.privacy.analytics" class="sr-only peer" @change="updateAnalytics")
                      div(class="settings-toggle-track")

            //- 3. Connection Tab
            div(v-else-if="activeNavTab === 'connection'" class="space-y-6")
              div(class="border-b border-themeBorder pb-4")
                h2(class="text-base font-bold text-themeText") Connection & Network
                p(class="text-xs text-themeText2 mt-0.5") Inspect direct routes and manage relay and broadcast integrations.

              //- Live connection diagnostics
              div(class="bg-themeBackground p-4 rounded-xl border border-themeBorder shadow-2xs")
                div(class="flex items-center justify-between gap-3 mb-3")
                  div
                    h3(class="text-sm font-semibold text-themeText") Desktop connectivity
                    p(class="text-[11px] text-themeText2 mt-0.5") Live endpoints advertised to paired clients
                  div(class="flex items-center gap-2")
                    details(class="relative group")
                      summary(class="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg border border-themeBorder bg-themeBackground2 text-[10px] cursor-pointer list-none select-none")
                        span(class="w-1.5 h-1.5 rounded-full" :class="mdnsDiagnostics?.active ? 'bg-themeConnected' : 'bg-themeText3'")
                        span(class="text-themeText3") mDNS
                        span(class="font-mono text-themeText2 max-w-32 truncate") {{ mdnsDiagnostics?.advertisement?.hostname || 'Unavailable' }}
                      div(class="absolute right-0 top-full mt-2 z-30 w-80 rounded-xl border border-themeBorder bg-themeBackground shadow-xl p-3 text-[10px]")
                        div(class="flex justify-between gap-3 py-1")
                          span(class="text-themeText3") Status
                          span(:class="mdnsDiagnostics?.active ? 'text-themeConnected' : 'text-themeText3'") {{ mdnsDiagnostics?.active ? 'Advertising' : 'Unavailable' }}
                        div(class="flex justify-between gap-3 py-1")
                          span(class="text-themeText3") Hostname
                          span(class="font-mono text-themeText2 text-right select-all") {{ mdnsDiagnostics?.advertisement?.hostname || '—' }}
                        div(class="flex justify-between gap-3 py-1")
                          span(class="text-themeText3") Instance
                          span(class="text-themeText2 text-right select-all") {{ mdnsDiagnostics?.advertisement?.instanceName || '—' }}
                        div(class="flex justify-between gap-3 py-1")
                          span(class="text-themeText3") Service
                          span(class="font-mono text-themeText2 select-all") {{ mdnsDiagnostics?.advertisement?.serviceType || '_vortideck._tcp.local.' }}
                        p(v-if="mdnsDiagnostics?.error" class="mt-2 pt-2 border-t border-themeBorder/50 text-themeError") {{ mdnsDiagnostics.error }}
                    button(
                      type="button"
                      @click="loadConnectionDiagnostics"
                      :disabled="connectionDiagnosticsLoading"
                      class="px-3 py-1.5 rounded-lg border border-themeBorder bg-themeBackground2 text-xs text-themeText2 hover:text-themeText disabled:opacity-50 transition-colors"
                    ) {{ connectionDiagnosticsLoading ? 'Refreshing…' : 'Refresh' }}

                div(v-if="connectionDiagnosticsError" class="p-3 rounded-lg border border-themeError/20 bg-themeError/10 text-xs text-themeError") {{ connectionDiagnosticsError }}
                div(v-else-if="connectionDiagnosticsLoading && !serverDiagnostics" class="p-4 text-center text-xs text-themeText3") Loading connection diagnostics…
                template(v-else)
                  div(class="grid grid-cols-1 md:grid-cols-3 gap-2.5")
                    div(class="rounded-xl border border-themeBorder bg-themeBackground2 p-3")
                      div(class="flex items-center justify-between gap-2")
                        span(class="text-[11px] font-semibold text-themeText") Local server
                        span(
                          class="text-[10px] font-medium"
                          :class="serverDiagnostics ? 'text-themeConnected' : 'text-themeText3'"
                        ) {{ serverDiagnostics ? 'Listening' : 'Unavailable' }}
                      p(class="mt-2 text-xs font-mono text-themeText select-all") {{ localServerEndpoint }}
                      p(class="mt-1 text-[10px] text-themeText3") Preferred port 9001; the displayed port is authoritative.

                    div(class="rounded-xl border border-themeBorder bg-themeBackground2 p-3")
                      div(class="flex items-center justify-between gap-2")
                        span(class="text-[11px] font-semibold text-themeText") Tailscale direct
                        span(
                          class="text-[10px] font-medium"
                          :class="tailscaleCandidates.length ? 'text-themeConnected' : 'text-themeText3'"
                        ) {{ tailscaleCandidates.length ? 'Detected' : 'Not detected' }}
                      p(class="mt-2 text-xs text-themeText2") {{ tailscaleCandidates.length ? `${tailscaleCandidates.length} endpoint${tailscaleCandidates.length === 1 ? '' : 's'} advertised` : 'Unavailable at VortiDeck startup' }}
                      p(class="mt-1 text-[10px] text-themeText3") Automatic; VortiDeck never changes your Tailscale configuration.

                    div(class="rounded-xl border border-themeBorder bg-themeBackground2 p-3")
                      div(class="flex items-center justify-between gap-2")
                        span(class="text-[11px] font-semibold text-themeText") VortiDeck relay
                        span(
                          class="text-[10px] font-medium"
                          :class="relayDiagnostics?.connected ? 'text-themeConnected' : relayEnabled ? 'text-themeConnecting' : 'text-themeText3'"
                        ) {{ relayStatusLabel }}
                      p(class="mt-2 text-xs text-themeText2") {{ relayEnabled ? 'Remote fallback for paired devices' : 'Remote connections are disabled' }}
                      p(class="mt-1 text-[10px] text-themeText3") {{ relayDiagnostics?.desktop_id ? `Desktop ${relayDiagnostics.desktop_id.slice(0, 8)}…` : 'No active relay identity' }}

                  div(v-if="advertisedCandidates.length" class="mt-3 rounded-xl border border-themeBorder overflow-hidden")
                    div(
                      v-for="candidate in advertisedCandidates"
                      :key="`${candidate.transport}-${candidate.host}-${candidate.port}`"
                      class="flex items-center justify-between gap-3 px-3 py-2.5 border-b border-themeBorder/40 last:border-b-0"
                    )
                      div(class="min-w-0")
                        div(class="flex items-center gap-2")
                          span(class="text-[10px] uppercase tracking-wide font-semibold text-themePrimary") {{ candidateRouteLabel(candidate) }}
                          span(class="text-[10px] text-themeText3") priority {{ candidate.priority }}
                        p(class="text-[11px] font-mono text-themeText2 truncate select-all" :title="candidateUrl(candidate)") {{ candidateUrl(candidate) }}
                      span(class="text-[10px] text-themeText3 shrink-0") Advertised

              //- Grouped Inset Card
              div(class="bg-themeBackground p-1.5 rounded-xl border border-themeBorder flex flex-col divide-y divide-themeBorder/40")
                div(class="flex justify-between items-center p-3 px-3.5")
                  div(class="flex-1 pr-4")
                    h3(class="text-xs font-semibold text-themeText") Enable Relay Connection
                    p(class="text-[11px] text-themeText2 mt-0.5") Allow mobile devices to connect via relay server
                    p(class="text-[10px] text-themeText3 mt-0.5") Changes take effect on next app startup or reconnect
                  div
                    label(class="relative inline-flex items-center cursor-pointer")
                      input(
                        type="checkbox"
                        v-model="relayEnabled"
                        @change="updateRelaySettings"
                        class="sr-only peer"
                      )
                      div(class="settings-toggle-track")

                div(class="flex justify-between items-center p-3 px-3.5")
                  div(class="flex-1 pr-4")
                    h3(class="text-xs font-semibold text-themeText") Enable Streaming Connection
                    p(class="text-[11px] text-themeText2 mt-0.5") Connect to chat platforms (Twitch, YouTube, Kick) for streaming commands
                    p(class="text-[10px] text-themeText3 mt-0.5") Changes take effect on next app startup or reconnect
                  div
                    label(class="relative inline-flex items-center cursor-pointer")
                      input(
                        type="checkbox"
                        v-model="streamingEnabled"
                        @change="updateStreamingSettings"
                        class="sr-only peer"
                      )
                      div(class="settings-toggle-track")

              //- Connection policy information
              div(class="bg-themePrimary/10 border border-themePrimary/20 p-3 px-3.5 rounded-xl flex items-center gap-2.5")
                span(class="text-themePrimary text-xs font-medium flex items-center gap-2") 
                  i(class="fas fa-info-circle")
                  | Route selection belongs to each mobile client. Desktop advertises every reachable route; mobile prefers LAN, then Tailscale, then relay.

              SettingsObsCompanionManager

            //- 4. AI & MCP Tab
            div(v-else-if="activeNavTab === 'ai'" class="space-y-6")
              div(class="border-b border-themeBorder pb-4")
                h2(class="text-base font-bold text-themeText") AI Settings & MCP Servers
                p(class="text-xs text-themeText2 mt-0.5") Connect VortiDeck to external MCP servers and configure LLM providers.

              //- Enable AI MCP Toggle Inset Card
              div(class="bg-themeBackground p-3.5 px-4 rounded-xl border border-themeBorder flex justify-between items-center")
                div
                  h3(class="text-xs font-semibold text-themeText") Enable AI and outbound MCP connections
                  p(class="text-[11px] text-themeText2 mt-0.5") Let the local assistant use configured models and external MCP tools
                  p(class="text-[10px] text-themeText3 mt-0.5") Find compatible servers at smithery.ai / mcpservers.org
                div
                  label(class="relative inline-flex items-center cursor-pointer")
                    input(type="checkbox" :checked="settings.ai.enabled" @click.prevent="toggleMcpEnabled" class="sr-only peer")
                    div(class="settings-toggle-track")

              //- MCP Servers Section
              div(class="bg-themeBackground p-4 rounded-xl border border-themeBorder shadow-2xs" :class="{ 'opacity-50 pointer-events-none': !settings.ai.enabled }")
                div(class="flex items-center justify-between mb-3")
                  div(class="flex items-center gap-2")
                    ServerIcon(class="text-themePrimary text-base")
                    h3(class="text-sm font-semibold text-themeText") MCP Servers
                  div(class="flex gap-2")
                    button(
                      @click="showAddServerModal = true"
                      class="px-3 py-1 bg-themeButton hover:bg-themeButtonHover text-white rounded-lg text-xs font-medium transition-colors"
                      :disabled="!settings.ai.enabled"
                    ) + Add Server
                    button(
                      @click="openMcpConfigFile"
                      class="px-3 py-1 bg-themeBackground2 border border-themeBorder hover:bg-themeBackground text-themeText rounded-lg text-xs font-medium transition-colors"
                      title="Open config file directly in your text editor"
                    ) Edit File

                div(v-if="mcpServers.length" class="space-y-2 mb-2")
                  div(
                    v-for="(server, index) in mcpServers" 
                    :key="index"
                    class="p-3 bg-themeBackground2 rounded-lg border-l-4 border-themePrimary border-t border-r border-b border-themeBorder text-xs flex justify-between items-center"
                  )
                    pre(class="select-text text-xs text-themeText overflow-x-auto whitespace-pre-wrap font-mono flex-1") {{ server.trim() }}
                    div(class="flex gap-2 shrink-0 ml-2")
                      button(class="px-2 py-1 bg-themePrimary/15 text-themePrimary rounded hover:bg-themePrimary/25 text-xs font-semibold cursor-pointer" @click="editServer(index)") Edit
                      button(class="px-2 py-1 bg-red-500/15 text-red-400 rounded hover:bg-red-500/25 text-xs font-semibold cursor-pointer" @click="confirmDeleteServer(index)") Remove
                div(v-else class="p-4 bg-themeBackground2 rounded-lg text-center text-xs text-themeText3") No servers configured. Add a server to get started.

              //- AI Providers Section
              div(class="bg-themeBackground p-4 rounded-xl border border-themeBorder shadow-2xs" :class="{ 'opacity-50 pointer-events-none': !settings.ai.enabled }")
                div(class="flex items-center justify-between mb-3")
                  div(class="flex items-center gap-2")
                    ModelIcon(class="text-themePrimary text-base")
                    h3(class="text-sm font-semibold text-themeText") AI Providers
                  div(class="flex gap-2")
                    button(
                      @click="refreshAIModels"
                      :disabled="!settings.ai.enabled || aiModelsStore.isLoading"
                      class="px-3 py-1 bg-themeBackground2 border border-themeBorder hover:bg-themeBackground text-themeText rounded-lg text-xs font-medium transition-colors cursor-pointer"
                    ) {{ aiModelsStore.isLoading ? 'Fetching...' : 'Refresh Models' }}
                    button(
                      @click="showAddModelModal = true"
                      class="px-3 py-1 bg-themeButton hover:bg-themeButtonHover text-white rounded-lg text-xs font-medium transition-colors cursor-pointer"
                      :disabled="!settings.ai.enabled"
                    ) + Add Provider

                div(v-if="modelsList.length" class="space-y-2 mb-2")
                  div(
                    v-for="(model, index) in modelsList" 
                    :key="index"
                    class="p-3 bg-themeBackground2 rounded-lg border-l-4 border-themePrimary border-t border-r border-b border-themeBorder text-xs flex justify-between items-center"
                  )
                    pre(class="select-text text-xs text-themeText overflow-x-auto whitespace-pre-wrap font-mono flex-1") {{ model.trim() }}
                    div(class="flex gap-2 shrink-0 ml-2")
                      button(class="px-2 py-1 bg-themePrimary/15 text-themePrimary rounded hover:bg-themePrimary/25 text-xs font-semibold cursor-pointer" @click="editModel(index)") Edit
                      button(class="px-2 py-1 bg-red-500/15 text-red-400 rounded hover:bg-red-500/25 text-xs font-semibold cursor-pointer" @click="confirmDeleteModel(index)") Remove
                div(v-else class="p-4 bg-themeBackground2 rounded-lg text-center text-xs text-themeText3") No providers configured. Add a provider to get started.

            //- 5. Performance Tab
            div(v-else-if="activeNavTab === 'performance'" class="space-y-6")
              div(class="border-b border-themeBorder pb-4")
                h2(class="text-base font-bold text-themeText") Performance Limits
                p(class="text-xs text-themeText2 mt-0.5") Configure execution timeouts and concurrent task thresholds.

              //- Grouped Inset Card
              div(class="bg-themeBackground p-1.5 rounded-xl border border-themeBorder flex flex-col divide-y divide-themeBorder/40")
                div(class="flex justify-between items-center p-3 px-3.5")
                  div(class="flex-1 pr-4")
                    h3(class="text-xs font-semibold text-themeText") Maximum Concurrent Actions
                    p(class="text-[11px] text-themeText2 mt-0.5") Number of actions executing simultaneously (0 = Unlimited, 1–50)
                  div(class="w-28")
                    input(
                      type="number"
                      v-model.number="executionSettings.max_concurrent_actions"
                      @change="updateExecutionSettings"
                      min="0"
                      max="50"
                      class="w-full bg-themeBackground2 border border-themeBorder text-themeText text-xs rounded-lg focus:ring-1 focus:ring-themeButton focus:border-themeButton p-1.5 px-2.5 transition-colors"
                    )

                div(class="flex justify-between items-center p-3 px-3.5")
                  div(class="flex-1 pr-4")
                    h3(class="text-xs font-semibold text-themeText") Action Execution Timeout
                    p(class="text-[11px] text-themeText2 mt-0.5") Maximum execution time in milliseconds (1000–600000)
                  div(class="w-28")
                    input(
                      type="number"
                      v-model.number="executionSettings.max_action_timeout_ms"
                      @change="updateExecutionSettings"
                      min="1000"
                      max="600000"
                      step="1000"
                      class="w-full bg-themeBackground2 border border-themeBorder text-themeText text-xs rounded-lg focus:ring-1 focus:ring-themeButton focus:border-themeButton p-1.5 px-2.5 transition-colors"
                    )

            //- 6. About Tab
            div(v-else-if="activeNavTab === 'about'" class="space-y-6")
              div(class="border-b border-themeBorder pb-4")
                h2(class="text-base font-bold text-themeText") About VortiDeck
                p(class="text-xs text-themeText2 mt-0.5") Version details, system diagnostics, and community links.

              div(class="bg-themeBackground p-4 rounded-xl border border-themeBorder flex items-center justify-between")
                div
                  h3(class="text-sm font-bold text-themeText") VortiDeck Desktop
                  p(class="text-xs text-themeText2 mt-0.5") Open-source macro deck & streaming automation engine
                  p(class="text-xs font-mono text-themePrimary mt-1.5") Version {{ appVersion }}
                button(
                  @click="openLogsWindow"
                  class="px-3.5 py-1.5 bg-themeButton hover:bg-themeButtonHover text-white rounded-lg text-xs font-medium transition-colors"
                ) View App Logs

  //- Add Server Modal
  BaseModal(:is-open="showAddServerModal" :z-index="50020" @close="closeServerModal")
    div(class="vd-modal-card p-6 w-full max-w-md max-h-[90vh] overflow-auto bg-themeBackground2 text-themeText border border-themeBorder2 rounded-xl")
      h3(class="text-base font-semibold text-themeText mb-4") {{ editingServerIndex === -1 ? 'Add MCP Server' : 'Edit MCP Server' }}
      div(class="mb-4")
        label(class="block text-xs font-medium text-themeText2 mb-1") Server Name
        input(
          type="text"
          v-model="newServer.name"
          class="w-full px-3 py-2 border border-themeBorder2 rounded-lg bg-themeBackground text-themeText text-xs focus:outline-none focus:border-themeButton"
          placeholder="weather"
        )
      div(class="mb-4")
        label(class="block text-xs font-medium text-themeText2 mb-1") Protocol
        select(
          v-model="newServer.protocol"
          class="w-full px-3 py-2 border border-themeBorder2 rounded-lg bg-themeBackground text-themeText text-xs focus:outline-none focus:border-themeButton"
        )
          option(value="http") Streamable HTTP
          option(value="stdio") Standard input/output (stdio)
          option(v-if="newServer.protocol === 'sse'" value="sse") Legacy SSE configuration alias
          option(v-if="newServer.protocol === 'ws'" value="ws") Custom WebSocket (experimental)
        p(class="text-[11px] text-themeText2 mt-1.5") VortiDeck is an MCP client. It does not expose an inbound MCP server.
        p(v-if="newServer.protocol === 'sse'" class="text-[11px] text-amber-500 mt-1") This legacy name currently uses Streamable HTTP. Save as Streamable HTTP when possible.
        p(v-if="newServer.protocol === 'ws'" class="text-[11px] text-amber-500 mt-1") WebSocket is a non-standard compatibility transport and is not offered for new connections.
      
      div(v-if="newServer.protocol !== 'stdio'" class="mb-4")
        label(class="block text-xs font-medium text-themeText2 mb-1") URL
        input(
          type="text"
          v-model="newServer.url"
          class="w-full px-3 py-2 border border-themeBorder2 rounded-lg bg-themeBackground text-themeText text-xs focus:outline-none focus:border-themeButton"
          placeholder="https://server.smithery.ai/@username/service"
        )

      div(class="mb-4")
        label(class="block text-xs font-medium text-themeText2 mb-1") Authentication
        select(
          v-model="newServer.authType"
          class="w-full px-3 py-2 border border-themeBorder2 rounded-lg bg-themeBackground text-themeText text-xs focus:outline-none focus:border-themeButton"
        )
          option(value="none") None
          option(value="apiKey" :disabled="newServer.protocol === 'stdio' || newServer.protocol === 'sse'") Bearer / service API key
          option(value="oauth" :disabled="newServer.protocol !== 'sse'") OAuth (legacy SSE configuration only)

      div(v-if="newServer.authType === 'apiKey'" class="mb-4")
        label(class="block text-xs font-medium text-themeText2 mb-1") API Key
        div(class="relative")
          input(
            :type="showServerApiKey ? 'text' : 'password'"
            v-model="serverApiKey"
            autocomplete="off"
            class="w-full px-3 py-2 pr-10 border border-themeBorder2 rounded-lg bg-themeBackground text-themeText text-xs font-mono hide-password-reveal focus:outline-none focus:border-themeButton"
            placeholder="Enter API key for this server"
          )
          button(
            @click="showServerApiKey = !showServerApiKey"
            type="button"
            class="absolute right-3 top-1/2 -translate-y-1/2 text-themeText2 hover:text-themeText transition-colors"
          )
            MdiEyeOutline(v-if="!showServerApiKey" class="w-4 h-4")
            MdiEyeOffOutline(v-else class="w-4 h-4")
        p(class="text-[11px] text-themeText2 mt-1") Sent as an HTTP Bearer credential; managed service endpoints may use their documented API-key query parameter.

      div(v-if="newServer.authType === 'oauth'" class="mb-4 p-3 border border-themeBorder2 rounded-lg bg-themeBackground")
        label(class="block text-xs font-medium text-themeText2 mb-3") OAuth Configuration
        div(class="space-y-3")
          div
            label(class="block text-xs font-medium text-themeText2 mb-1") Redirect URI
            input(
              type="text"
              v-model="newServer.oauth.redirect_uri"
              class="w-full px-3 py-2 border border-themeBorder2 rounded-lg bg-themeBackground2 text-themeText text-xs focus:outline-none focus:border-themeButton"
              placeholder="http://localhost:8080/callback"
            )
          div
            label(class="block text-xs font-medium text-themeText2 mb-1") Callback Port
            input(
              type="number"
              v-model="newServer.oauth.callback_port"
              class="w-full px-3 py-2 border border-themeBorder2 rounded-lg bg-themeBackground2 text-themeText text-xs focus:outline-none focus:border-themeButton"
              placeholder="8080"
            )

      template(v-if="newServer.protocol === 'stdio'")
        div(class="mb-4")
          label(class="block text-xs font-medium text-themeText2 mb-1") Command
          input(
            type="text"
            v-model="newServer.command"
            class="w-full px-3 py-2 border border-themeBorder2 rounded-lg bg-themeBackground text-themeText text-xs focus:outline-none focus:border-themeButton"
            placeholder="cmd"
          )
        div(class="mb-4 max-h-48 overflow-y-auto")
          label(class="block text-xs font-medium text-themeText2 mb-1") Command Arguments
          div(class="border border-themeBorder2 rounded-lg bg-themeBackground p-3")
            div(v-for="(arg, index) in newServer.args" :key="index" class="flex gap-2 mb-2 items-center")
              input(
                type="text"
                v-model="newServer.args[index]"
                class="flex-1 px-3 py-1.5 border border-themeBorder2 rounded-lg bg-themeBackground2 text-themeText text-xs focus:outline-none focus:border-themeButton"
                placeholder="argument"
              )
              button(
                type="button"
                @click="newServer.args.splice(index, 1)"
                class="p-1.5 text-red-400 hover:text-red-300 hover:bg-red-500/10 rounded transition-colors flex-shrink-0"
                title="Remove argument"
              )
                svg(class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                  path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")
            button(
              @click="newServer.args.push('')"
              class="text-xs text-themePrimary hover:underline mt-2 flex items-center gap-1 font-medium"
            )
              i(class="fas fa-plus")
              | Add Argument

      div(class="mb-4")
        label(class="block text-xs font-medium text-themeText2 mb-1") Environment Variables
        div(
          v-for="(key, index) in newServerEnvKeys"
          :key="index"
          class="flex gap-2 mb-2 items-center"
        )
          input(
            type="text"
            v-model="newServerEnvKeys[index]"
            class="flex-1 px-3 py-1.5 border border-themeBorder2 rounded-lg bg-themeBackground text-themeText text-xs focus:outline-none focus:border-themeButton"
            placeholder="KEY"
          )
          input(
            type="text"
            v-model="newServerEnvValues[index]"
            class="flex-1 px-3 py-1.5 border border-themeBorder2 rounded-lg bg-themeBackground text-themeText text-xs focus:outline-none focus:border-themeButton"
            placeholder="value"
          )
          button(
            type="button"
            @click="removeEnvVar(index)"
            class="p-1.5 text-red-400 hover:text-red-300 hover:bg-red-500/10 rounded transition-colors flex-shrink-0"
            title="Remove variable"
          )
            svg(class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")
        button(
          @click="addEnvVar"
          class="text-xs text-themePrimary hover:underline flex items-center gap-1 font-medium"
        )
          i(class="fas fa-plus")
          | Add Environment Variable
      div(class="flex justify-end gap-2 mt-6")
        button(
          @click="closeServerModal"
          class="px-4 py-2 border border-themeBorder2 rounded-lg text-xs font-medium text-themeText2 hover:text-themeText bg-themeBackground hover:bg-themeBackground3 transition-colors"
        ) Cancel
        button(
          @click="saveServerConfig"
          class="px-4 py-2 bg-themeButton hover:bg-themeButtonHover text-white rounded-lg text-xs font-medium transition-colors"
        ) {{ editingServerIndex === -1 ? 'Add Server' : 'Save Changes' }}

  //- Add Provider Modal
  BaseModal(:is-open="showAddModelModal" :z-index="50020" @close="closeModelModal")
    div(class="vd-modal-card p-6 w-full max-w-lg max-h-[90vh] overflow-y-auto bg-themeBackground2 text-themeText border border-themeBorder2 rounded-xl")
      h3(class="text-base font-semibold text-themeText mb-4") {{ editingModelIndex === -1 ? 'Add AI Provider' : 'Edit AI Provider' }}

      div(class="mb-4")
        label(class="block text-xs font-medium text-themeText2 mb-1") Provider Type
        select(
          v-model="newModel.provider"
          @change="onProviderChange"
          class="w-full px-3 py-2 border border-themeBorder2 rounded-lg bg-themeBackground text-themeText text-xs focus:outline-none focus:border-themeButton"
        )
          option(value="openai") OpenAI
          option(value="anthropic") Anthropic (Claude)
          option(value="azure") Azure OpenAI
          option(value="openrouter") OpenRouter
          option(value="ollama") Ollama (Local)
          option(value="custom") Custom (OpenAI-compatible)
        p(class="text-[11px] text-themeText2 mt-1") {{ getProviderDescription(newModel.provider) }}

      div(class="mb-4")
        label(class="block text-xs font-medium text-themeText2 mb-1") Provider Name
        input(
          type="text"
          v-model="newModel.name"
          class="w-full px-3 py-2 border border-themeBorder2 rounded-lg bg-themeBackground text-themeText text-xs focus:outline-none focus:border-themeButton"
          :placeholder="getProviderNamePlaceholder(newModel.provider)"
        )
        p(class="text-[11px] text-themeText2 mt-1") A friendly name to identify this provider

      div(class="mb-4")
        label(class="block text-xs font-medium text-themeText2 mb-1") API Key
        div(class="relative")
          input(
            :type="showApiKey ? 'text' : 'password'"
            v-model="newModel.apiKey"
            class="w-full px-3 py-2 pr-10 border border-themeBorder2 rounded-lg bg-themeBackground text-themeText text-xs font-mono focus:outline-none focus:border-themeButton"
            :placeholder="getApiKeyPlaceholder(newModel.provider)"
          )
          button(
            @click="showApiKey = !showApiKey"
            type="button"
            class="absolute right-3 top-1/2 -translate-y-1/2 text-themeText2 hover:text-themeText transition-colors"
          )
            MdiEyeOutline(v-if="!showApiKey" class="w-4 h-4")
            MdiEyeOffOutline(v-if="showApiKey" class="w-4 h-4")
        p(class="text-[11px] text-themeText2 mt-1") Get your API key from {{ getProviderKeyUrl(newModel.provider) }}

      div(class="mb-4")
        button(
          @click="showAdvancedProviderSettings = !showAdvancedProviderSettings"
          class="text-xs text-themePrimary hover:underline flex items-center gap-1"
        )
          i(:class="showAdvancedProviderSettings ? 'fas fa-chevron-down' : 'fas fa-chevron-right'")
          | Advanced Settings (Custom URLs)

      div(v-if="showAdvancedProviderSettings" class="mb-4 p-3 bg-themeBackground rounded-lg border border-themeBorder space-y-3")
        div(v-if="newModel.provider === 'custom'" class="space-y-3")
          div
            label(class="block text-xs font-medium text-themeText2 mb-1") Base URL / Host
            input(
              type="text"
              v-model="newModel.baseUrl"
              class="w-full p-2 border border-themeBorder2 rounded-lg bg-themeBackground2 text-themeText text-xs focus:outline-none focus:border-themeButton"
              placeholder="localhost or 192.168.1.100"
            )
          div
            label(class="block text-xs font-medium text-themeText2 mb-1") Port
            input(
              type="number"
              v-model.number="newModel.port"
              class="w-full p-2 border border-themeBorder2 rounded-lg bg-themeBackground2 text-themeText text-xs focus:outline-none focus:border-themeButton"
              placeholder="1234 (LM Studio), 8000 (vLLM)"
            )
          div
            label(class="block text-xs font-medium text-themeText2 mb-1") Chat Endpoint Path
            input(
              type="text"
              v-model="newModel.chatEndpoint"
              class="w-full p-2 border border-themeBorder2 rounded-lg bg-themeBackground2 text-themeText text-xs font-mono focus:outline-none focus:border-themeButton"
              placeholder="/v1/chat/completions"
            )
          div
            label(class="block text-xs font-medium text-themeText2 mb-1") Models Endpoint Path
            input(
              type="text"
              v-model="newModel.modelsEndpoint"
              class="w-full p-2 border border-themeBorder2 rounded-lg bg-themeBackground2 text-themeText text-xs font-mono focus:outline-none focus:border-themeButton"
              placeholder="/v1/models"
            )
        div(v-else)
          div
            label(class="block text-xs font-medium text-themeText2 mb-1") Custom Base URL
            input(
              type="text"
              v-model="newModel.url"
              class="w-full p-2 border border-themeBorder2 rounded-lg bg-themeBackground2 text-themeText text-xs font-mono focus:outline-none focus:border-themeButton"
              :placeholder="getUrlPlaceholder(newModel.provider)"
            )
          div(v-if="!['openai', 'anthropic', 'openrouter', 'ollama'].includes(newModel.provider)")
            label(class="block text-xs font-medium text-themeText2 mb-1") Models URL
            input(
              type="text"
              v-model="newModel.modelsUrl"
              class="w-full p-2 border border-themeBorder2 rounded-lg bg-themeBackground2 text-themeText text-xs font-mono focus:outline-none focus:border-themeButton"
              placeholder="https://api.example.com/v1/models"
            )

      div(class="mb-4")
        label(class="block text-xs font-medium text-themeText2 mb-1") Default Model (Optional)
        input(
          type="text"
          v-model="newModel.defaultModel"
          class="w-full p-2 border border-themeBorder2 rounded-lg bg-themeBackground text-themeText text-xs focus:outline-none focus:border-themeButton"
          :placeholder="getDefaultModelPlaceholder(newModel.provider)"
        )

      div(v-if="newModel.provider === 'azure'" class="mb-4")
        label(class="block text-xs font-medium text-themeText2 mb-1") Azure Deployment Name
        input(
          type="text"
          v-model="newModel.deployment"
          class="w-full p-2 border border-themeBorder2 rounded-lg bg-themeBackground text-themeText text-xs focus:outline-none focus:border-themeButton"
          placeholder="gpt-4-deployment"
        )

      div(class="flex justify-end gap-2 mt-6")
        button(
          @click="closeModelModal"
          class="px-4 py-2 border border-themeBorder2 rounded-lg text-xs"
        ) Cancel
        button(
          @click="saveModelConfig"
          class="px-4 py-2 bg-themeButton hover:bg-themeButtonHover text-white rounded-lg text-xs font-medium"
        ) {{ editingModelIndex === -1 ? 'Add Provider' : 'Save Changes' }}
</template>

<script setup lang="ts">
const props = withDefaults(
  defineProps<{
    isPage?: boolean;
  }>(),
  {
    isPage: false,
  }
);

const settingsModal = useSettingsModal();
const activeNavTab = computed({
  get: () => settingsModal.activeTab.value as any,
  set: (val: any) => {
    settingsModal.activeTab.value = val;
  },
});

const handleBackdropClick = () => {
  if (!props.isPage) {
    settingsModal.closeSettings();
  }
};

const handleKeydown = (e: KeyboardEvent) => {
  if (e.key === "Escape" && settingsModal.isOpen.value && !props.isPage) {
    settingsModal.closeSettings();
  }
};

onMounted(() => {
  if (typeof window !== "undefined") {
    window.addEventListener("keydown", handleKeydown);
  }
});

onUnmounted(() => {
  if (typeof window !== "undefined") {
    window.removeEventListener("keydown", handleKeydown);
  }
});

import BaseModal from "../Modal/BaseModal.vue";
import MdiEyeOutline from "~icons/mdi/eye-outline";
import MdiEyeOffOutline from "~icons/mdi/eye-off-outline";
import ServerIcon from "~icons/mdi/server-outline";
import ModelIcon from "~icons/carbon/model";
import HugeiconsPaintBoard from "~icons/hugeicons/paint-board";
import HugeiconsSettings01 from "~icons/hugeicons/settings-01";
import HeroiconsGlobeAlt from "~icons/heroicons/globe-alt";
import HugeiconsAiBrain02 from "~icons/hugeicons/ai-brain-02";
import MdiTuner from "~icons/mdi/tuner";
import HeroiconsInformationCircle from "~icons/heroicons/information-circle";
import { platform } from "@tauri-apps/plugin-os";
import { mkdir, readTextFile, writeTextFile, BaseDirectory } from "@tauri-apps/plugin-fs";

import { ref, computed, onMounted, onUnmounted, watchEffect } from "vue";
import Multiselect from "vue-multiselect";
import "vue-multiselect/dist/vue-multiselect.css";

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getVersion } from "@tauri-apps/api/app";

import { useMainStore } from "@/stores/Main";
import { useAIModelsStore } from "@/stores/AIModels";
import { enable, isEnabled, disable } from "@tauri-apps/plugin-autostart";
import { useI18n } from "vue-i18n";
import { openPath } from "@tauri-apps/plugin-opener";
import { appConfigDir } from "@tauri-apps/api/path";
import {
  isPermissionGranted,
  requestPermission,
} from "@tauri-apps/plugin-notification";

type ProviderId = "openai" | "anthropic" | "azure" | "openrouter" | "ollama" | "custom";

interface ProviderPreset {
  url?: string;
  modelsUrl?: string;
  baseUrl?: string;
  port?: number;
  chatEndpoint?: string;
  modelsEndpoint?: string;
  defaultModel: string;
  description: string;
}

interface McpServerEntry {
  name: string;
  protocol: string;
  url?: string;
  command?: string;
  args?: string[];
  envs?: Record<string, string>;
  oauth?: { redirect_uri?: string; callback_port?: number };
  api_key?: string;
  [key: string]: unknown;
}

interface McpModelEntry {
  name: string;
  provider: ProviderId;
  apiKey?: string;
  url?: string;
  modelsUrl?: string;
  default_model?: string;
  deployment?: string;
  baseUrl?: string;
  port?: number | null;
  chatEndpoint?: string;
  modelsEndpoint?: string;
  [key: string]: unknown;
}

interface McpConfig {
  models: { model: McpModelEntry[]; [key: string]: unknown };
  mcp: { server: McpServerEntry[]; [key: string]: unknown };
  [key: string]: unknown;
}

interface AppConfigData {
  mcp_enabled: boolean;
  max_concurrent_actions: number;
  max_action_timeout_ms: number;
  relay_enabled: boolean;
  streaming_enabled: boolean;
  analytics_enabled: boolean;
}

interface AppConfigResponse {
  data: AppConfigData;
  status: number;
  message: string;
}

interface ConnectionCandidateDiagnostic {
  transport: "lan" | "tailscale";
  host: string;
  port: number;
  path: string;
  secure: boolean;
  priority: number;
}

interface ServerDiagnostics {
  ip: string;
  port: number;
  serverName: string;
  connectionCandidates: ConnectionCandidateDiagnostic[];
}

interface RelayDiagnostics {
  enabled: boolean;
  connected: boolean;
  relay_url: string;
  desktop_id: string | null;
}

interface MdnsDiagnostics {
  active: boolean;
  advertisement: {
    serviceType: string;
    instanceName: string;
    hostname: string;
    displayHostname: string;
    ip: string;
    port: number;
  } | null;
  error: string | null;
}

interface ConnectionDiagnostics {
  server: ServerDiagnostics | null;
  relay: RelayDiagnostics;
  mdns: MdnsDiagnostics;
}

interface ProviderForm {
  name: string;
  provider: ProviderId;
  url: string;
  modelsUrl: string;
  apiKey: string;
  defaultModel: string;
  deployment: string;
  baseUrl: string;
  port: number | null;
  chatEndpoint: string;
  modelsEndpoint: string;
}

interface ServerForm {
  name: string;
  protocol: string;
  url: string;
  command: string;
  args: string[];
  envs: Record<string, string>;
  oauth: { redirect_uri: string; callback_port: number };
  authType: "none" | "apiKey" | "oauth";
}

const errorMessage = (error: unknown, fallback: string) =>
  error instanceof Error ? error.message : fallback;

const emptyMcpConfig = (): McpConfig => ({
  models: { model: [] },
  mcp: { server: [] },
});

const parseMcpConfig = (source: string): McpConfig => {
  const parsed = JSON.parse(source) as Partial<McpConfig>;
  const models = parsed.models && typeof parsed.models === "object" ? parsed.models : {};
  const mcp = parsed.mcp && typeof parsed.mcp === "object" ? parsed.mcp : {};
  return {
    ...parsed,
    models: {
      ...models,
      model: Array.isArray(parsed.models?.model) ? parsed.models.model : [],
    },
    mcp: {
      ...mcp,
      server: Array.isArray(parsed.mcp?.server) ? parsed.mcp.server : [],
    },
  };
};

const { locale, locales, setLocale } = useI18n();

const mainStore = useMainStore();
const appScale = useAppScale();
const workflowApprovalPresentation = computed({
  get: () => mainStore.getWorkflowApprovalPresentation,
  set: (value: "notification" | "dialog") =>
    mainStore.setWorkflowApprovalPresentation(value),
});

const toggleNativeApprovalNotifications = async (event: Event) => {
  const enabled = (event.target as HTMLInputElement).checked;
  if (!enabled) {
    mainStore.setNativeWorkflowApprovalNotifications(false);
    return;
  }

  try {
    const alreadyGranted = await isPermissionGranted();
    const permission = alreadyGranted ? "granted" : await requestPermission();
    if (permission !== "granted") {
      mainStore.setNativeWorkflowApprovalNotifications(false);
      mainStore.addNotification({
        title: "Notifications were not enabled",
        message: "Allow notifications in Windows settings to receive background approval alerts.",
        type: "warning",
      });
      return;
    }
    mainStore.setNativeWorkflowApprovalNotifications(true);
  } catch (error) {
    mainStore.setNativeWorkflowApprovalNotifications(false);
    mainStore.addNotification({
      title: "Could not enable notifications",
      message: errorMessage(error, "Native notification setup failed"),
      type: "error",
    });
  }
};

const navTabs = [
  { id: 'appearance', label: 'Appearance', icon: HugeiconsPaintBoard },
  { id: 'general', label: 'General', icon: HugeiconsSettings01 },
  { id: 'connection', label: 'Connection', icon: HeroiconsGlobeAlt },
  { id: 'ai', label: 'AI & MCP', icon: HugeiconsAiBrain02 },
  { id: 'performance', label: 'Performance', icon: MdiTuner },
  { id: 'about', label: 'About', icon: HeroiconsInformationCircle },
];

const openLogsWindow = async () => {
  try {
    await invoke("open_logs_window");
  } catch (err) {
    console.error("Failed to open logs window:", err);
  }
};

// AI Models store
const aiModelsStore = useAIModelsStore();

// Note: Confirmation modals and server connection testing can be implemented using existing modal system
// Theme handling
const selectedTheme = ref(mainStore.getTheme);

const changeTheme = () => {
  mainStore.setTheme(selectedTheme.value);
};

// Initialize with default values
const autostart = ref(false);
const autostartSupported = !import.meta.dev;
const mcpEnabled = ref(false);
const languages = ref(locales);
const currentLocale = computed(() => {
  return languages.value.find((i) => i.code === locale.value);
});

// Settings object
const settings = ref({
  autostart: false,
  language: {
    code: "",
    name: "",
  },
  notifications: {
    enabled: true,
    connections: true,
    updates: true,
  },
  performance: {
    hardwareAcceleration: true,
  },
  updates: {
    autoUpdate: true,
    checkFrequency: "daily",
  },
  network: {
    interface: "wlan0",
    port: 9001,
  },
  privacy: {
    analytics: false,
  },
  shortcuts: {
    toggleDeck: "Ctrl+D",
    openSettings: "Ctrl+,",
    newMacro: "Ctrl+N",
    connectDevice: "Ctrl+Alt+C",
  },
  ai: {
    enabled: false,
    model: "gpt-4",
    temperature: 0.5,
    contextWindow: 10,
    serverCommand: "",
  },
});

// App config settings (includes execution, relay, and streaming settings)
const executionSettings = ref({
  max_concurrent_actions: 10,
  max_action_timeout_ms: 300000,
});
const relayEnabled = ref(true);
const streamingEnabled = ref(true);
const serverDiagnostics = ref<ServerDiagnostics | null>(null);
const relayDiagnostics = ref<RelayDiagnostics | null>(null);
const mdnsDiagnostics = ref<MdnsDiagnostics | null>(null);
const connectionDiagnosticsLoading = ref(false);
const connectionDiagnosticsError = ref("");

const advertisedCandidates = computed(() =>
  [...(serverDiagnostics.value?.connectionCandidates ?? [])]
    .sort((left, right) => left.priority - right.priority)
);
const tailscaleCandidates = computed(() =>
  advertisedCandidates.value.filter((candidate) => candidate.transport === "tailscale")
);
const localServerEndpoint = computed(() => {
  const server = serverDiagnostics.value;
  return server ? `${server.ip}:${server.port}` : "Unavailable";
});
const relayStatusLabel = computed(() => {
  if (!relayEnabled.value) return "Disabled";
  return relayDiagnostics.value?.connected ? "Connected" : "Disconnected";
});

const candidateUrl = (candidate: ConnectionCandidateDiagnostic) => {
  const scheme = candidate.secure ? "wss" : "ws";
  const host = candidate.host.includes(":") && !candidate.host.startsWith("[")
    ? `[${candidate.host}]`
    : candidate.host;
  return `${scheme}://${host}:${candidate.port}${candidate.path}`;
};

const candidateRouteLabel = (candidate: ConnectionCandidateDiagnostic) => {
  if (candidate.transport === "lan") return "LAN";
  if (candidate.host.endsWith(".ts.net")) return "MagicDNS";
  return candidate.host.includes(":") ? "Tailscale IPv6" : "Tailscale IP";
};

const loadConnectionDiagnostics = async () => {
  connectionDiagnosticsLoading.value = true;
  connectionDiagnosticsError.value = "";
  try {
    const diagnostics = await invoke<ConnectionDiagnostics>("get_connection_diagnostics");
    serverDiagnostics.value = diagnostics.server;
    relayDiagnostics.value = diagnostics.relay;
    mdnsDiagnostics.value = diagnostics.mdns;
  } catch (error) {
    connectionDiagnosticsError.value = errorMessage(
      error,
      "Could not load connection diagnostics",
    );
  } finally {
    connectionDiagnosticsLoading.value = false;
  }
};

// Load all app config settings from backend (single source of truth)
const loadAppConfigSettings = async () => {
  try {
    const response = await invoke<AppConfigResponse>("get_app_config");
    console.log("✅ Loaded app config settings:", response);

    if (response && response.data) {
      const config = response.data;

      // Load execution settings
      executionSettings.value = {
        max_concurrent_actions: config.max_concurrent_actions,
        max_action_timeout_ms: config.max_action_timeout_ms,
      };

      // Load relay setting
      relayEnabled.value = config.relay_enabled || false;

      // Load streaming setting
      streamingEnabled.value = config.streaming_enabled || false;

      // Load analytics setting (PostHog opt-out is synced via app_config_updated event listener)
      settings.value.privacy.analytics = config.analytics_enabled ?? true;
    }
  } catch (error) {
    console.error("❌ Failed to load app config settings:", error);
  }
};

// Update analytics setting
const updateAnalytics = async () => {
  try {
    // update_app_config emits app_config_updated — PostHog plugin listens and syncs opt-out
    await invoke("update_app_config", {
      newAnalyticsEnabled: settings.value.privacy.analytics,
    });

    mainStore.addNotification({
      title: "Settings Updated",
      message: settings.value.privacy.analytics
        ? "Usage analytics enabled"
        : "Usage analytics disabled",
      type: "success",
      autoClose: true,
      duration: 3000,
    });
  } catch (error) {
    console.error("❌ Failed to update analytics setting:", error);
  }
};

// Update execution settings
const updateExecutionSettings = async () => {
  try {
    await invoke("update_app_config", {
      newMaxConcurrentActions: executionSettings.value.max_concurrent_actions,
      newMaxActionTimeoutMs: executionSettings.value.max_action_timeout_ms,
    });
    console.log("✅ Updated execution settings:", executionSettings.value);

    mainStore.addNotification({
      title: "Settings Updated",
      message: "Execution settings have been saved successfully",
      type: "success",
      autoClose: true,
      duration: 3000,
    });
  } catch (error) {
    console.error("❌ Failed to update execution settings:", error);

    mainStore.addNotification({
      title: "Failed to Update",
      message: errorMessage(error, "Could not save execution settings"),
      type: "error",
      autoClose: true,
      duration: 5000,
    });
  }
};

// Update relay settings
const updateRelaySettings = async () => {
  try {
    await invoke("update_app_config", {
      newRelayEnabled: relayEnabled.value,
    });
    console.log("✅ Updated relay settings:", relayEnabled.value);

    mainStore.addNotification({
      title: "Relay Settings Updated",
      message: relayEnabled.value
        ? "Relay will connect on next startup"
        : "Relay disabled - restart app to disconnect",
      type: "success",
      autoClose: true,
      duration: 3000,
    });
  } catch (error) {
    console.error("❌ Failed to update relay settings:", error);

    mainStore.addNotification({
      title: "Failed to Update",
      message: errorMessage(error, "Could not save relay settings"),
      type: "error",
      autoClose: true,
      duration: 5000,
    });
  }
};

// Update streaming settings
const updateStreamingSettings = async () => {
  try {
    await invoke("update_app_config", {
      newStreamingEnabled: streamingEnabled.value,
    });
    console.log("✅ Updated streaming settings:", streamingEnabled.value);

    mainStore.addNotification({
      title: "Streaming Settings Updated",
      message: streamingEnabled.value
        ? "Streaming will connect on next startup"
        : "Streaming disabled - restart app to disconnect",
      type: "success",
      autoClose: true,
      duration: 3000,
    });
  } catch (error) {
    console.error("❌ Failed to update streaming settings:", error);

    mainStore.addNotification({
      title: "Failed to Update",
      message: errorMessage(error, "Could not save streaming settings"),
      type: "error",
      autoClose: true,
      duration: 5000,
    });
  }
};

// Initialize data that requires async operations
const initializeData = async () => {
  // Get autostart setting
  autostart.value = autostartSupported ? await isEnabled() : false;
  settings.value.autostart = autostart.value;

  // Get MCP enabled setting
  try {
    const config = await invoke<AppConfigResponse>("get_app_config");
    mcpEnabled.value = config.data.mcp_enabled;
    settings.value.ai.enabled = mcpEnabled.value;
  } catch (error) {
    console.error("Failed to get app config:", error);
  }

  // Set language from locale
  if (currentLocale.value) {
    settings.value.language = {
      code: currentLocale.value.code,
      name: String(currentLocale.value.name ?? currentLocale.value.code),
    };
  }

  // Load app config settings (execution + relay)
  await loadAppConfigSettings();
};

// MCP server config
const mcpServerConfig = ref("");
const loadMcpConfig = async () => {
  try {
    const configContent = await readTextFile("resources/mcp/config.json", {
      baseDir: BaseDirectory.AppConfig,
    });
    mcpServerConfig.value = configContent;
  } catch (error) {
    console.info("MCP config does not exist yet; creating an empty configuration", error);
    const emptyConfig = JSON.stringify(emptyMcpConfig(), null, 2);
    await mkdir("resources/mcp", {
      baseDir: BaseDirectory.AppConfig,
      recursive: true,
    });
    await writeTextFile("resources/mcp/config.json", emptyConfig, {
      baseDir: BaseDirectory.AppConfig,
    });
    mcpServerConfig.value = emptyConfig;
  }
};

// Function to open MCP config file directly
const openMcpConfigFile = async () => {
  try {
    // Get the app config directory path
    const configRoot = await appConfigDir();
    const configFilePath = `${configRoot}/resources/mcp/config.json`;

    // Open the file with the default system application
    await openPath(configFilePath);
  } catch (error) {
    console.error("Failed to open MCP config file:", error);
    alert("Failed to open MCP config file. Please check the logs for more details.");
  }
};

let appVersion = ref("dev");
const loadAppVersion = async () => {
  try {
    appVersion.value = await getVersion();
  } catch (error) {
    console.error("Failed to get app version:", error);
    appVersion.value = "Unknown";
  }
};

onMounted(() => {
  loadAppVersion();
});

// AI Models refresh function
const refreshAIModels = async () => {
  try {
    // Build provider configs from current model settings
    const providerConfigs = [];
    await loadMcpConfig(); // Load the config file content into mcpServerConfig ref

    // Parse the JSON config
    let config: McpConfig;
    try {
      config = parseMcpConfig(mcpServerConfig.value);
    } catch (e) {
      console.error("❌ Failed to parse MCP config:", e);
      throw new Error("Invalid MCP config JSON");
    }

    if (config?.models?.model) {
      for (const model of config.models.model) {
        if (model.provider) {
          providerConfigs.push({
            provider: model.provider,
            apiKey: model.apiKey || "", // API key is optional for custom providers
            url: model.url, // Pass the base URL from config
            modelsUrl: model.modelsUrl, // Pass custom models URL if set
            default_model: model.default_model, // Pass default model for fallback
            name: model.name, // Pass provider name
          });
        }
      }
    }

    console.log("🔄 Refreshing models from", providerConfigs.length, "provider(s)");

    await aiModelsStore.refresh(providerConfigs);

    mainStore.addNotification({
      title: "Models Refreshed",
      message: `Successfully fetched latest models from ${providerConfigs.length} provider(s)`,
      type: "success",
      autoClose: true,
      duration: 3000,
    });
  } catch (error) {
    console.error("❌ Failed to refresh models:", error);
    mainStore.addNotification({
      title: "Failed to Refresh Models",
      message: errorMessage(error, "Could not fetch models from providers"),
      type: "error",
      autoClose: true,
      duration: 5000,
    });
  }
};

// Format last updated time for display
const formatLastUpdated = (isoString: string) => {
  const date = new Date(isoString);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMins / 60);
  const diffDays = Math.floor(diffHours / 24);

  if (diffMins < 1) return "just now";
  if (diffMins < 60) return `${diffMins} minute${diffMins !== 1 ? "s" : ""} ago`;
  if (diffHours < 24) return `${diffHours} hour${diffHours !== 1 ? "s" : ""} ago`;
  return `${diffDays} day${diffDays !== 1 ? "s" : ""} ago`;
};

// Call initialization functions when component is mounted
onMounted(async () => {
  await Promise.all([
    initializeData(),
    loadMcpConfig(),
    loadAppVersion(),
    loadConnectionDiagnostics(),
  ]);

  console.log("Settings loaded");
});

let unlistenRelayStatus: (() => void) | null = null;
onMounted(async () => {
  unlistenRelayStatus = await listen<{ type?: string }>("relay_event", (event) => {
    if (event.payload.type === "statusChanged") void loadConnectionDiagnostics();
  });
});

onUnmounted(() => {
  unlistenRelayStatus?.();
  unlistenRelayStatus = null;
});

const toggleAutostart = async () => {
  if (!autostartSupported) {
    settings.value.autostart = false;
    return;
  }
  const enabled = await isEnabled();
  const currentState = settings.value.autostart;
  console.log(enabled);
  if (currentState) {
    await disable().then(() => {
      settings.value.autostart = false;
    });
    console.log("Autostart: ", settings.value.autostart);
  } else {
    await enable().then(() => {
      settings.value.autostart = true;
    });
    console.log("Autostart: ", settings.value.autostart);
  }
};

const changeLanguage = async (language: { code: string }) => {
  console.log("changeLanguage", language);
  // The camelCase to snake_case conversion isn't happening automatically
  await invoke("update_app_config", { newLanguage: language.code });
};

// Labels for shortcuts
const shortcutLabels = {
  toggleDeck: "Toggle Deck View",
  openSettings: "Open Settings",
  newMacro: "Create New Macro",
  connectDevice: "Connect New Device",
};

// Function to edit a keyboard shortcut
const editShortcut = (key: keyof typeof shortcutLabels) => {
  // In a real implementation, this would open a modal to capture key combinations
  alert(`Editing shortcut for ${shortcutLabels[key]}`);
};

listen<AppConfigResponse>("app_config_updated", async (event) => {
  settings.value.ai.enabled = event.payload.data.mcp_enabled;
});

const toggleMcpEnabled = async () => {
  const res = (await invoke("update_app_config", {
    newMcpEnabled: !settings.value.ai.enabled,
  })) as { status: number; message: string };
};

// Save settings
const saveSettings = async () => {
  try {
    // In a real implementation, this would save to Tauri storage
    // Example: await invoke("update_app_config", { config: JSON.stringify(settings.value) });

    // For now, we'll just log the settings
    console.log("Saving settings:", settings.value);
  } catch (error) {
    console.error("Failed to save settings:", error);
  }
};
// MCP SERVER SETTINGS
const showAddServerModal = ref(false);
const showAdvancedProviderSettings = ref(false);
const showAddModelModal = ref(false);
const showApiKey = ref(false);
const editingServerIndex = ref(-1);
const editingModelIndex = ref(-1);
const newServer = ref<ServerForm>({
  name: "",
  protocol: "http",
  url: "",
  command: "",
  args: [] as string[],
  envs: {
    PATH: "%PATH%",
  },
  oauth: {
    redirect_uri: "http://localhost:8080/callback",
    callback_port: 8080,
  },
  authType: "none" as "none" | "apiKey" | "oauth",
});
const newModel = ref<ProviderForm>({
  name: "",
  provider: "openai",
  url: "https://api.openai.com/v1",
  modelsUrl: "", // Custom models URL (optional)
  apiKey: "",
  defaultModel: "",
  deployment: "",
  // Custom provider flexible fields
  baseUrl: "",
  port: null,
  chatEndpoint: "",
  modelsEndpoint: "",
});
const newEnvVar = ref("");
const newServerEnvKeys = ref(["PATH"]);
const newServerEnvValues = ref(["%PATH%"]);
const serverApiKey = ref("");
const showServerApiKey = ref(false);

// Provider helper functions
const providerPresets: Record<ProviderId, ProviderPreset> = {
  openai: {
    url: "https://api.openai.com/v1/chat/completions",
    modelsUrl: "https://api.openai.com/v1/models",
    defaultModel: "gpt-4",
    description: "Official OpenAI API (requires API key from platform.openai.com)",
  },
  anthropic: {
    url: "https://api.anthropic.com/v1/messages",
    modelsUrl: "https://api.anthropic.com/v1/models",
    defaultModel: "claude-3-5-sonnet-20241022",
    description: "Official Anthropic API (requires API key from console.anthropic.com)",
  },
  azure: {
    url:
      "https://YOUR-RESOURCE.openai.azure.com/openai/deployments/YOUR-DEPLOYMENT/chat/completions?api-version=2024-02-15-preview",
    defaultModel: "gpt-4",
    description: "Azure OpenAI Service (enterprise)",
  },
  openrouter: {
    url: "https://openrouter.ai/api/v1/chat/completions",
    modelsUrl: "https://openrouter.ai/api/v1/models",
    defaultModel: "openai/gpt-4",
    description: "Unified gateway to multiple AI models with single API key",
  },
  ollama: {
    url: "http://localhost:11434/api/chat",
    modelsUrl: "http://localhost:11434/api/tags",
    defaultModel: "llama2",
    description: "Local Ollama instance - no API key required",
  },
  custom: {
    baseUrl: "localhost",
    port: 1234,
    chatEndpoint: "/v1/chat/completions",
    modelsEndpoint: "/v1/models",
    defaultModel: "gpt-3.5-turbo",
    description: "Custom OpenAI-compatible API (LM Studio, vLLM, text-gen-webui, LocalAI, etc.)",
  },
};

const onProviderChange = () => {
  const preset = providerPresets[newModel.value.provider];
  if (preset) {
    // Handle custom provider with flexible configuration
    if (newModel.value.provider === 'custom') {
      newModel.value.baseUrl = preset.baseUrl || "";
      newModel.value.port = preset.port || null;
      newModel.value.chatEndpoint = preset.chatEndpoint || "";
      newModel.value.modelsEndpoint = preset.modelsEndpoint || "";
      newModel.value.defaultModel = preset.defaultModel;
      // Construct URLs from components
      if (preset.baseUrl && preset.port && preset.chatEndpoint) {
        newModel.value.url = `http://${preset.baseUrl}:${preset.port}${preset.chatEndpoint}`;
      }
      if (preset.baseUrl && preset.port && preset.modelsEndpoint) {
        newModel.value.modelsUrl = `http://${preset.baseUrl}:${preset.port}${preset.modelsEndpoint}`;
      }
    } else {
      // Standard providers with full URLs
      newModel.value.url = preset.url ?? "";
      newModel.value.defaultModel = preset.defaultModel;
      newModel.value.modelsUrl = preset.modelsUrl || "";
    }
  }
};

const getProviderDescription = (provider: ProviderId) => {
  return providerPresets[provider]?.description || "";
};

const getProviderNamePlaceholder = (provider: ProviderId) => {
  const names: Record<ProviderId, string> = {
    openai: "My OpenAI API",
    anthropic: "My Claude API",
    azure: "Azure Production",
    openrouter: "OpenRouter",
    ollama: "Local Ollama",
    custom: "Custom Provider",
  };
  return names[provider] || "Provider Name";
};

const getApiKeyPlaceholder = (provider: ProviderId) => {
  const placeholders: Record<ProviderId, string> = {
    openai: "sk-proj-...",
    anthropic: "sk-ant-...",
    azure: "your-azure-key",
    openrouter: "sk-or-v1-...",
    ollama: "not required",
    custom: "optional-api-key",
  };
  return placeholders[provider] || "API Key";
};

const getUrlPlaceholder = (provider: ProviderId) => {
  return providerPresets[provider]?.url || "https://api.example.com/v1/chat/completions";
};

const getDefaultModelPlaceholder = (provider: ProviderId) => {
  const placeholders: Record<ProviderId, string> = {
    openai: "gpt-4, gpt-4-turbo, gpt-3.5-turbo",
    anthropic: "claude-3-5-sonnet-20241022, claude-3-opus",
    azure: "deployment-name",
    openrouter: "openai/gpt-4, anthropic/claude-3.5-sonnet",
    ollama: "llama2, deepseek-r1:8b, qwen2.5-coder",
    custom: "llama2, mistral",
  };
  return placeholders[provider] || "model-name";
};

const getProviderKeyUrl = (provider: ProviderId) => {
  const urls: Record<ProviderId, string> = {
    openai: "platform.openai.com",
    anthropic: "console.anthropic.com",
    azure: "Azure Portal",
    openrouter: "openrouter.ai/keys",
    ollama: "N/A (local)",
    custom: "your provider",
  };
  return urls[provider] || "your provider";
};

const addServerToConfig = async () => {
  try {
    // Parse current config
    let config: McpConfig;
    try {
      config = parseMcpConfig(mcpServerConfig.value);
    } catch (error) {
      // Initialize empty config if parsing fails
      config = emptyMcpConfig();
    }

    // Create new server entry
    const newServerEntry: McpServerEntry = {
      name: newServer.value.name,
      protocol: newServer.value.protocol,
    };

    // Add protocol-specific fields
    if (newServer.value.protocol === "stdio") {
      newServerEntry.command = newServer.value.command;
      if (newServer.value.args.length > 0) {
        newServerEntry.args = [...newServer.value.args];
      }
    } else {
      newServerEntry.url = newServer.value.url;
    }

    // Add OAuth configuration only when authType is 'oauth'
    if (newServer.value.authType === "oauth") {
      newServerEntry.oauth = {
        redirect_uri: newServer.value.oauth.redirect_uri,
        callback_port: newServer.value.oauth.callback_port,
      };
    }
    if (newServer.value.authType === "apiKey" && serverApiKey.value.trim()) {
      newServerEntry.api_key = serverApiKey.value.trim();
    }

    // Add environment variables if any
    if (Object.keys(newServer.value.envs).length > 0) {
      newServerEntry.envs = { ...newServer.value.envs };
    }

    // Add new server to config
    config.mcp.server.push(newServerEntry);

    // Save updated config
    const updatedConfig = JSON.stringify(config, null, 2);
    mcpServerConfig.value = updatedConfig;

    await writeTextFile("resources/mcp/config.json", updatedConfig, {
      baseDir: BaseDirectory.AppConfig,
    });

    // Reset form and close modal
    newServer.value = {
      name: "",
      protocol: "http",
      url: "",
      command: "",
      args: [] as string[],
      envs: {
        PATH: "%PATH%",
      },
      oauth: {
        redirect_uri: "http://localhost:8080/callback",
        callback_port: 8080,
      },
      authType: "none",
    };
    showAddServerModal.value = false;

    return newServerEntry;
  } catch (error) {
    console.error("Failed to add server:", error);
    return null;
  }
};

const addModelToConfig = async () => {
  try {
    // Parse current config
    let config: McpConfig;
    try {
      config = parseMcpConfig(mcpServerConfig.value);
    } catch (error) {
      // Initialize empty config if parsing fails
      config = emptyMcpConfig();
    }

    // Create new provider entry
    const newModelEntry: McpModelEntry = {
      name: newModel.value.name,
      provider: newModel.value.provider,
      apiKey: newModel.value.apiKey,
    };

    // Handle custom provider with flexible configuration
    const hasFlexibleConfig = newModel.value.provider === 'custom'
      && newModel.value.baseUrl
      && newModel.value.port;

    if (hasFlexibleConfig) {
      // For custom providers with components: Save ONLY components (source of truth)
      // Backend will construct URLs from these components
      newModelEntry.baseUrl = newModel.value.baseUrl;
      newModelEntry.port = newModel.value.port;
      if (newModel.value.chatEndpoint) {
        newModelEntry.chatEndpoint = newModel.value.chatEndpoint;
      }
      if (newModel.value.modelsEndpoint) {
        newModelEntry.modelsEndpoint = newModel.value.modelsEndpoint;
      }

      // Also save a constructed URL for backward compatibility with old backend code
      const protocol = "http://";
      const base = `${protocol}${newModel.value.baseUrl}:${newModel.value.port}`;
      newModelEntry.url = newModel.value.chatEndpoint ? `${base}${newModel.value.chatEndpoint}` : base;

      if (newModel.value.modelsEndpoint) {
        newModelEntry.modelsUrl = `${base}${newModel.value.modelsEndpoint}`;
      }
    } else {
      // For non-custom providers or custom without components: Use traditional URL
      const finalUrl = (newModel.value.url || "").trim() ||
                       providerPresets[newModel.value.provider]?.url ||
                       "https://api.openai.com/v1";
      newModelEntry.url = finalUrl;

      const finalModelsUrl = (newModel.value.modelsUrl || "").trim();
      if (finalModelsUrl) {
        newModelEntry.modelsUrl = finalModelsUrl;
      }
    }

    // Add other optional fields
    if (newModel.value.defaultModel) {
      newModelEntry.default_model = newModel.value.defaultModel;
    }
    if (newModel.value.deployment) {
      newModelEntry.deployment = newModel.value.deployment;
    }

    // Add new model to config
    config.models.model.push(newModelEntry);

    // Save updated config
    const updatedConfig = JSON.stringify(config, null, 2);
    mcpServerConfig.value = updatedConfig;

    await writeTextFile("resources/mcp/config.json", updatedConfig, {
      baseDir: BaseDirectory.AppConfig,
    });

    // Reset form and close modal
    newModel.value = {
      name: "",
      provider: "openai",
      url: "https://api.openai.com/v1",
      modelsUrl: "",
      apiKey: "",
      defaultModel: "",
      deployment: "",
      baseUrl: "",
      port: null,
      chatEndpoint: "",
      modelsEndpoint: "",
    };
    showAddModelModal.value = false;
    showApiKey.value = false;
    showAdvancedProviderSettings.value = false;
  } catch (error) {
    console.error("Failed to add model:", error);
  }
};

const removeServer = async (index: number) => {
  try {
    // Parse config
    const config = parseMcpConfig(mcpServerConfig.value);

    // Remove server at index
    if (
      config.mcp &&
      config.mcp.server &&
      Array.isArray(config.mcp.server) &&
      index >= 0 &&
      index < config.mcp.server.length
    ) {
      // Get server name before removing for disconnect
      const serverName = config.mcp.server[index]?.name;

      config.mcp.server.splice(index, 1);

      // Save updated config
      const updatedConfig = JSON.stringify(config, null, 2);
      mcpServerConfig.value = updatedConfig;

      await writeTextFile("resources/mcp/config.json", updatedConfig, {
        baseDir: BaseDirectory.AppConfig,
      });

      // Disconnect the removed server
      if (serverName && settings.value.ai.enabled) {
        try {
          await invoke("disconnect_mcp_server", { serverName });
          mainStore.addNotification({
            title: "MCP Server Removed",
            message: `Disconnected from ${serverName}`,
            type: "success",
            autoClose: true,
          });
        } catch (e: any) {
          console.error("Failed to disconnect MCP server:", e);
          mainStore.addNotification({
            title: "MCP Server Disconnect Failed",
            message: e?.message || e || "Could not disconnect server",
            type: "error",
            autoClose: true,
          });
        }
      }
    }
  } catch (error) {
    console.error("Failed to remove server:", error);
  }
};

const removeModel = async (index: number) => {
  try {
    // Parse config
    const config = parseMcpConfig(mcpServerConfig.value);

    // Remove model at index
    if (
      config.models &&
      config.models.model &&
      Array.isArray(config.models.model) &&
      index >= 0 &&
      index < config.models.model.length
    ) {
      config.models.model.splice(index, 1);

      // Save updated config
      const updatedConfig = JSON.stringify(config, null, 2);
      mcpServerConfig.value = updatedConfig;

      await writeTextFile("resources/mcp/config.json", updatedConfig, {
        baseDir: BaseDirectory.AppConfig,
      });
    }
  } catch (error) {
    console.error("Failed to remove model:", error);
  }
};

// Add the new confirmation functions here
const confirmDeleteServer = (index: number) => {
  if (confirm(`Are you sure you want to remove this server?`)) {
    removeServer(index);
  }
};

const confirmDeleteModel = (index: number) => {
  if (confirm(`Are you sure you want to remove this model?`)) {
    removeModel(index);
  }
};

const addEnvVar = () => {
  newServerEnvKeys.value.push("");
  newServerEnvValues.value.push("");
};

const removeEnvVar = (index: number) => {
  newServerEnvKeys.value.splice(index, 1);
  newServerEnvValues.value.splice(index, 1);
};

// Helper to get server name at index for disconnect
const getServerNameAtIndex = (index: number): string | null => {
  try {
    const config = parseMcpConfig(mcpServerConfig.value);
    if (config.mcp?.server?.[index]) {
      return config.mcp.server[index].name;
    }
    return null;
  } catch {
    return null;
  }
};

// Computed properties for server and model lists
const mcpServers = computed(() => {
  if (!mcpServerConfig.value) return [];

  try {
    const config = parseMcpConfig(mcpServerConfig.value);
    if (!config.mcp || !config.mcp.server || !Array.isArray(config.mcp.server)) {
      return [];
    }

    // Format each server as a string for display
    return config.mcp.server.map((server: McpServerEntry) => {
      return Object.entries(server)
        .map(([key, value]) => {
          if (key === "args" && Array.isArray(value)) {
            return `${key} = [\n  ${value.map((arg) => `"${arg}"`).join(",\n  ")}\n]`;
          } else if (key === "envs" && value !== null && typeof value === "object") {
            return `envs = {\n  ${Object.entries(value)
              .map(([k, v]) => `${k} = "${v}"`)
              .join(",\n  ")}\n}`;
          } else if (key === "oauth" && value !== null && typeof value === "object") {
            return `oauth = {\n  ${Object.entries(value)
              .map(([k, v]) => `${k} = ${typeof v === "string" ? `"${v}"` : v}`)
              .join(",\n  ")}\n}`;
          } else if (typeof value === "string") {
            return `${key} = "${value}"`;
          } else {
            return `${key} = ${JSON.stringify(value)}`;
          }
        })
        .join("\n");
    });
  } catch (error) {
    console.error("Error parsing JSON config:", error);
    return [];
  }
});

const modelsList = computed(() => {
  if (!mcpServerConfig.value) return [];

  try {
    const config = parseMcpConfig(mcpServerConfig.value);
    if (!config.models || !config.models.model || !Array.isArray(config.models.model)) {
      return [];
    }

    // Format each model as a string for display with masked API keys
    return config.models.model.map((model: McpModelEntry) => {
      return Object.entries(model)
        .map(([key, value]) => {
          // Mask API keys for security
          if (key === "apiKey" && typeof value === "string" && value.length > 0) {
            const maskedLength = Math.max(0, Math.min(value.length - 7, 20));
            const masked = value.slice(0, 7) + "•".repeat(maskedLength);
            return `${key} = "${masked}"`;
          }

          if (typeof value === "string") {
            return `${key} = "${value}"`;
          } else {
            return `${key} = ${JSON.stringify(value)}`;
          }
        })
        .join("\n");
    });
  } catch (error) {
    console.error("Error parsing JSON config:", error);
    return [];
  }
});

console.log(modelsList.value);
const models = ref<string[]>([]);

// Update models ref when modelsList changes
watchEffect(() => {
  models.value = modelsList.value;
});

console.log(mcpServers.value);

const editServer = (index: number) => {
  try {
    // Parse config
    const config = parseMcpConfig(mcpServerConfig.value);

    if (
      config.mcp &&
      config.mcp.server &&
      Array.isArray(config.mcp.server) &&
      index >= 0 &&
      index < config.mcp.server.length
    ) {
      const server = config.mcp.server[index];
      if (!server) return;

      // Detect auth type from config
      let detectedAuthType: "none" | "apiKey" | "oauth" = "none";
      if (server.api_key || (server.envs && server.envs.API_KEY)) {
        detectedAuthType = "apiKey";
      } else if (server.oauth && (server.oauth.redirect_uri || server.oauth.callback_port)) {
        detectedAuthType = "oauth";
      }

      // Initialize server object with values from config
      newServer.value = {
        name: server.name || "",
        protocol: server.protocol || "http",
        url: server.url || "",
        command: server.command || "",
        args: Array.isArray(server.args) ? [...server.args] : [],
        envs: server.envs ? { ...server.envs } : { PATH: "%PATH%" },
        oauth: server.oauth
          ? {
              redirect_uri: server.oauth.redirect_uri || "http://localhost:8080/callback",
              callback_port: server.oauth.callback_port || 8080,
            }
          : {
              redirect_uri: "http://localhost:8080/callback",
              callback_port: 8080,
            },
        authType: detectedAuthType,
      };

      // Extract API_KEY from envs if present
      if (server.api_key || (server.envs && server.envs.API_KEY)) {
        serverApiKey.value = server.api_key || server.envs?.API_KEY || "";
        // Remove API_KEY from envs display (it has its own field)
        const { API_KEY, ...restEnvs } = server.envs || {};
        newServer.value.envs = Object.keys(restEnvs).length > 0 ? restEnvs : { PATH: "%PATH%" };
      } else {
        serverApiKey.value = "";
      }

      // Initialize environment variables arrays (without API_KEY)
      newServerEnvKeys.value = Object.keys(newServer.value.envs);
      newServerEnvValues.value = Object.values(newServer.value.envs) as string[];

      editingServerIndex.value = index;
      showAddServerModal.value = true;
    }
  } catch (error) {
    console.error("Failed to edit server:", error);
  }
};

// Parse URL into components for migration from old configs
const parseUrlToComponents = (url: string) => {
  try {
    const parsed = new URL(url);
    return {
      baseUrl: parsed.hostname,
      port: parsed.port ? parseInt(parsed.port) : null,
      endpoint: parsed.pathname,
    };
  } catch {
    return { baseUrl: "", port: null, endpoint: "" };
  }
};

const editModel = (index: number) => {
  try {
    // Parse config
    const config = parseMcpConfig(mcpServerConfig.value);

    if (
      config.models &&
      config.models.model &&
      Array.isArray(config.models.model) &&
      index >= 0 &&
      index < config.models.model.length
    ) {
      const model = config.models.model[index];
      if (!model) return;

      // For custom providers: parse URL into components if components don't exist (migration)
      let baseUrl = model.baseUrl || "";
      let port = model.port || null;
      let chatEndpoint = model.chatEndpoint || "";
      let modelsEndpoint = model.modelsEndpoint || "";

      if (model.provider === "custom" && !model.baseUrl && model.url) {
        // Migration: parse old URL format into components
        const parsedChat = parseUrlToComponents(model.url);
        baseUrl = parsedChat.baseUrl;
        port = parsedChat.port;
        chatEndpoint = parsedChat.endpoint;

        // Try to parse models URL too if it exists
        if (model.modelsUrl) {
          const parsedModels = parseUrlToComponents(model.modelsUrl);
          modelsEndpoint = parsedModels.endpoint;
        }

        console.log(`🔄 Migrating old custom provider URL format:`, {
          oldUrl: model.url,
          newComponents: { baseUrl, port, chatEndpoint, modelsEndpoint }
        });
      }

      // Initialize model object with values from config
      newModel.value = {
        name: model.name || "",
        provider: model.provider || "openai",
        url: model.url || "",
        modelsUrl: model.modelsUrl || "",
        apiKey: model.apiKey || "",
        defaultModel: model.default_model || "",
        deployment: model.deployment || "",
        baseUrl,
        port,
        chatEndpoint,
        modelsEndpoint,
      };

      editingModelIndex.value = index;
      showAddModelModal.value = true;
    }
  } catch (error) {
    console.error("Failed to edit model:", error);
  }
};

const closeServerModal = () => {
  showAddServerModal.value = false;
  editingServerIndex.value = -1;
  newServer.value = {
    name: "",
    protocol: "http",
    url: "",
    command: "",
    args: [],
    envs: {
      PATH: "%PATH%",
    },
    oauth: {
      redirect_uri: "http://localhost:8080/callback",
      callback_port: 8080,
    },
    authType: "none",
  };
  newServerEnvKeys.value = ["PATH"];
  newServerEnvValues.value = ["%PATH%"];
  serverApiKey.value = "";
  showServerApiKey.value = false;
};

const closeModelModal = () => {
  showAddModelModal.value = false;
  showApiKey.value = false; // Reset password visibility
  showAdvancedProviderSettings.value = false; // Reset advanced settings toggle
  editingModelIndex.value = -1;
  newModel.value = {
    name: "",
    provider: "openai",
    url: "https://api.openai.com/v1",
    modelsUrl: "",
    apiKey: "",
    defaultModel: "",
    deployment: "",
    baseUrl: "",
    port: null,
    chatEndpoint: "",
    modelsEndpoint: "",
  };
};

const saveServerConfig = async () => {
  const serverName = newServer.value.name.trim();
  if (!serverName) {
    mainStore.addNotification({
      title: "Server name required",
      message: "Enter a stable, unique name for this MCP server.",
      type: "error",
      autoClose: true,
    });
    return;
  }

  const currentConfig = parseMcpConfig(mcpServerConfig.value);
  const duplicateName = currentConfig.mcp.server.some(
    (server, index) =>
      index !== editingServerIndex.value &&
      server.name.trim().toLocaleLowerCase() === serverName.toLocaleLowerCase(),
  );
  if (duplicateName) {
    mainStore.addNotification({
      title: "Server name already used",
      message: `An MCP server named ${serverName} already exists.`,
      type: "error",
      autoClose: true,
    });
    return;
  }
  newServer.value.name = serverName;

  // Convert environment variables arrays to object
  const envs: Record<string, string> = {};
  newServerEnvKeys.value.forEach((key, index) => {
    if (key && newServerEnvValues.value[index]) {
      envs[key] = newServerEnvValues.value[index];
    }
  });

  newServer.value.envs = envs as any;

  if (editingServerIndex.value === -1) {
    // Adding new server
    const serverEntry = await addServerToConfig();
    if (serverEntry && settings.value.ai.enabled) {
      try {
        const result = await invoke<{ success: boolean; message: string; tools_count: number }>("connect_mcp_server", { server: serverEntry });
        mainStore.addNotification({
          title: "MCP Server Connected",
          message: result.message || `Connected to ${serverEntry.name}`,
          type: "success",
          autoClose: true,
        });
      } catch (e: any) {
        console.error("Failed to connect new MCP server:", e);
        mainStore.addNotification({
          title: "MCP Server Connection Failed",
          message: e?.message || e || "Could not connect to server",
          type: "error",
          autoClose: true,
        });
      }
    }
  } else {
    // Editing existing server - disconnect old, connect new
    const oldServerName = getServerNameAtIndex(editingServerIndex.value);
    const serverEntry = await updateServerConfig(editingServerIndex.value);
    if (settings.value.ai.enabled) {
      try {
        if (oldServerName) {
          await invoke("disconnect_mcp_server", { serverName: oldServerName });
        }
        if (serverEntry) {
          const result = await invoke<{ success: boolean; message: string; tools_count: number }>("connect_mcp_server", { server: serverEntry });
          mainStore.addNotification({
            title: "MCP Server Updated",
            message: result.message || `Reconnected to ${serverEntry.name}`,
            type: "success",
            autoClose: true,
          });
        }
      } catch (e: any) {
        console.error("Failed to reconnect MCP server:", e);
        mainStore.addNotification({
          title: "MCP Server Reconnection Failed",
          message: e?.message || e || "Could not reconnect to server",
          type: "error",
          autoClose: true,
        });
      }
    }
  }
};

const saveModelConfig = async () => {
  if (editingModelIndex.value === -1) {
    await addModelToConfig();
    useMcpSession(settings.value.ai.enabled);
  } else {
    await updateModelConfig(editingModelIndex.value);
    useMcpSession(settings.value.ai.enabled);
  }
};

const updateServerConfig = async (index: number) => {
  try {
    // Parse config
    const config = parseMcpConfig(mcpServerConfig.value);

    if (
      config.mcp &&
      config.mcp.server &&
      Array.isArray(config.mcp.server) &&
      index >= 0 &&
      index < config.mcp.server.length
    ) {
      // Create updated server entry
      const updatedServer: McpServerEntry = {
        name: newServer.value.name,
        protocol: newServer.value.protocol,
      };

      // Add protocol-specific fields
      if (newServer.value.protocol === "stdio") {
        updatedServer.command = newServer.value.command;
        if (newServer.value.args.length > 0) {
          updatedServer.args = [...newServer.value.args];
        }
      } else {
        updatedServer.url = newServer.value.url;
      }

      // Add OAuth configuration only when authType is 'oauth'
      if (newServer.value.authType === "oauth") {
        updatedServer.oauth = {
          redirect_uri: newServer.value.oauth.redirect_uri,
          callback_port: newServer.value.oauth.callback_port,
        };
      }
      if (newServer.value.authType === "apiKey" && serverApiKey.value.trim()) {
        updatedServer.api_key = serverApiKey.value.trim();
      }

      // Add environment variables if any
      if (Object.keys(newServer.value.envs).length > 0) {
        updatedServer.envs = { ...newServer.value.envs };
      }

      // Update server in config
      config.mcp.server[index] = updatedServer;

      // Save updated config
      const updatedConfig = JSON.stringify(config, null, 2);
      mcpServerConfig.value = updatedConfig;

      await writeTextFile("resources/mcp/config.json", updatedConfig, {
        baseDir: BaseDirectory.AppConfig,
      });

      // Reset form and close modal
      closeServerModal();

      return updatedServer;
    }
    return null;
  } catch (error) {
    console.error("Failed to update server:", error);
    return null;
  }
};

const updateModelConfig = async (index: number) => {
  try {
    // Parse config
    const config = parseMcpConfig(mcpServerConfig.value);

    if (
      config.models &&
      config.models.model &&
      Array.isArray(config.models.model) &&
      index >= 0 &&
      index < config.models.model.length
    ) {
      // Fallback to default URL if empty or whitespace
      const finalUrl =
        (newModel.value.url || "").trim() ||
        providerPresets[newModel.value.provider]?.url ||
        "https://api.openai.com/v1";

      // Create updated model entry with new structure
      const updatedModel: McpModelEntry = {
        name: newModel.value.name,
        provider: newModel.value.provider,
        url: finalUrl,
        apiKey: newModel.value.apiKey,
      };

      // Add optional fields
      if (newModel.value.modelsUrl && newModel.value.modelsUrl.trim()) {
        updatedModel.modelsUrl = newModel.value.modelsUrl.trim();
      }
      if (newModel.value.defaultModel) {
        updatedModel.default_model = newModel.value.defaultModel;
      }
      if (newModel.value.deployment) {
        updatedModel.deployment = newModel.value.deployment;
      }

      // Update model in config
      config.models.model[index] = updatedModel;

      // Save updated config
      const updatedConfig = JSON.stringify(config, null, 2);
      mcpServerConfig.value = updatedConfig;

      await writeTextFile("resources/mcp/config.json", updatedConfig, {
        baseDir: BaseDirectory.AppConfig,
      });

      // Reset form and close modal
      closeModelModal();
    }
  } catch (error) {
    console.error("Failed to update model:", error);
  }
};

// Add reset functions with confirmation alert
const resetServerConfig = async () => {
  if (
    confirm(
      "Are you sure you want to reset all server configurations? This action cannot be undone."
    )
  ) {
    try {
      // Parse config
      let config: McpConfig;
      try {
        config = parseMcpConfig(mcpServerConfig.value);
      } catch (error) {
        config = emptyMcpConfig();
      }

      // Reset servers to empty array
      config.mcp = { server: [] };

      // Save updated config
      const updatedConfig = JSON.stringify(config, null, 2);
      mcpServerConfig.value = updatedConfig;

      await writeTextFile("resources/mcp/config.json", updatedConfig, {
        baseDir: BaseDirectory.AppConfig,
      });
    } catch (error) {
      console.error("Failed to reset server config:", error);
    }
  }
};

const resetModels = async () => {
  if (
    confirm(
      "Are you sure you want to reset all model configurations? This action cannot be undone."
    )
  ) {
    try {
      // Parse config
      let config: McpConfig;
      try {
        config = parseMcpConfig(mcpServerConfig.value);
      } catch (error) {
        config = emptyMcpConfig();
      }

      // Reset models to empty array
      config.models = { model: [] };

      // Save updated config
      const updatedConfig = JSON.stringify(config, null, 2);
      mcpServerConfig.value = updatedConfig;

      await writeTextFile("resources/mcp/config.json", updatedConfig, {
        baseDir: BaseDirectory.AppConfig,
      });
    } catch (error) {
      console.error("Failed to reset models config:", error);
    }
  }
};

// import { Command } from "@tauri-apps/plugin-shell";
// onMounted(async () => {
//   const command = Command.sidecar("binaries/node-runner/node-runner", [
//     "-y",
//     "@smithery/cli@latest",
//     "run",
//     "@smithery-ai/server-sequential-thinking",
//   ]);

//   command.stdout.on("data", (event) => {
//     console.log(event);
//   });

//   command.stderr.on("data", (event) => {
//     console.log(event);
//   });

//   const output = await command.spawn();
// Hermes-Style Theme Palettes
const paletteSearch = ref('');
const availablePalettes = ref([
  {
    id: 'vortideck',
    name: 'VortiDeck',
    description: 'Midnight obsidian canvas, Electric blue accent',
    lightPreview: { bg: '#f8fafc', surface: '#ffffff', sidebar: '#e2e8f0', line: '#0f172a', accent: '#3b82f6', border: '#cbd5e1' },
    darkPreview: { bg: '#030712', surface: '#0f172a', sidebar: '#111827', line: '#f8fafc', accent: '#3b82f6', border: '#1e293b' }
  },
  {
    id: 'github',
    name: 'GitHub',
    description: 'GitHub Light Default and Dark Default',
    lightPreview: { bg: '#f6f8fa', surface: '#ffffff', sidebar: '#eaeef2', line: '#1f2328', accent: '#0969da', border: '#d0d7de' },
    darkPreview: { bg: '#0d1117', surface: '#161b22', sidebar: '#21262d', line: '#f0f6fc', accent: '#2f81f7', border: '#30363d' }
  },
  {
    id: 'catppuccin',
    name: 'Catppuccin',
    description: 'Soothing pastels — Latte and Mocha',
    lightPreview: { bg: '#eff1f5', surface: '#ffffff', sidebar: '#ccd0da', line: '#4c4f69', accent: '#8839ef', border: '#bcc0cc' },
    darkPreview: { bg: '#1e1e2e', surface: '#24273a', sidebar: '#181825', line: '#cdd6f4', accent: '#cba6f7', border: '#363a4f' }
  },
  {
    id: 'everforest',
    name: 'Everforest',
    description: 'Warm, low-contrast forest greens',
    lightPreview: { bg: '#f3efda', surface: '#ffffff', sidebar: '#e6e2cc', line: '#5c6a72', accent: '#8da101', border: '#dfdbbf' },
    darkPreview: { bg: '#27342b', surface: '#343f36', sidebar: '#2d353b', line: '#d3c6aa', accent: '#a7c080', border: '#3e483e' }
  }
]);

const filteredPalettes = computed(() => {
  if (!paletteSearch.value.trim()) return availablePalettes.value;
  const q = paletteSearch.value.toLowerCase();
  return availablePalettes.value.filter(p => p.name.toLowerCase().includes(q) || p.description.toLowerCase().includes(q));
});

//   console.log(output);
// });
</script>

<style lang="scss" scoped>
@use "./modalTheme.scss" as *;

.settings-content-panel {
  &::-webkit-scrollbar {
    display: block !important;
    width: 6px !important;
    height: 6px !important;
  }

  &::-webkit-scrollbar-button,
  &::-webkit-scrollbar-button:single-button,
  &::-webkit-scrollbar-button:vertical:decrement,
  &::-webkit-scrollbar-button:vertical:increment {
    display: none !important;
    width: 0 !important;
    height: 0 !important;
  }

  &::-webkit-scrollbar-track {
    background: transparent !important;
  }

  &::-webkit-scrollbar-thumb {
    background: hsl(var(--border2) / 0.8) !important;
    border-radius: 9999px !important;

    &:hover {
      background: hsl(var(--primary)) !important;
    }
  }

  &::-webkit-scrollbar-corner {
    background: transparent !important;
  }
}

.settings-section {
  transition: all 0.3s ease;

  &:hover {
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  }
}

.setting-item {
  transition: all 0.2s ease;
  padding-inline: 0.5rem;
  padding-block: 0.4rem;
  border-radius: 0.375rem;
}

.shortcut-key {
  font-family: monospace;
}

.settings-toggle-track {
  width: 2.6rem;
  height: 1.45rem;
  border-radius: 9999px;
  background-color: hsl(var(--border2) / 0.5);
  border: 1px solid hsl(var(--border2) / 0.8);
  position: relative;
  transition: all 0.2s ease;
  cursor: pointer;

  &::after {
    content: '';
    position: absolute;
    top: 2px;
    left: 2px;
    width: 1.05rem;
    height: 1.05rem;
    border-radius: 9999px;
    background-color: #ffffff;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25);
    transition: transform 0.2s ease;
  }
}

input:checked + .settings-toggle-track,
.peer:checked ~ .settings-toggle-track {
  background-color: hsl(var(--button)) !important;
  border-color: hsl(var(--button)) !important;

  &::after {
    transform: translateX(1.15rem);
  }
}

/* Hide browser's native password reveal button */
.hide-password-reveal::-ms-reveal,
.hide-password-reveal::-ms-clear,
.hide-password-reveal::-webkit-credentials-auto-fill-button,
.hide-password-reveal::-webkit-textfield-decoration-container {
  display: none !important;
}
</style>
