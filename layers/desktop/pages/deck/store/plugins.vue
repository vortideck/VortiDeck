<template lang="pug">
div(class="deck-store-page deck-store-plugins w-full h-full flex flex-col")
  // Tab navigation - sticky header
  div(class="store-header sticky top-0 z-10 px-6 pt-3 pb-3 flex items-center justify-between border-b border-themeBorder")
    div(class="flex items-center gap-1 bg-themeBackground2 p-1 rounded-xl border border-themeBorder shadow-2xs")
      button(
        v-for="tab in tabList"
        :key="tab.key"
        @click="currentTab = tab.key"
        :class="currentTab === tab.key ? 'bg-themeBackground text-themeText shadow-xs font-semibold' : 'text-themeText2 hover:text-themeText'"
        class="px-3.5 py-1.5 rounded-lg text-xs transition-all cursor-pointer"
      )
        span {{ tab.label }}

  // Main content area with proper scroll
  div(class="plugins-content-scroll scrollbar-visible flex-1 overflow-y-auto")
    div(class="p-6 pt-4")
      div(v-if="catalogWarning" class="mb-4 rounded-xl border border-themeBorder bg-themeBackground2 p-3 text-xs text-themeText2")
        div(class="flex items-start gap-2")
          Icon(icon="heroicons:cloud-slash" class="mt-0.5 h-4 w-4 shrink-0 text-themeText3")
          div
            p(class="font-semibold text-themeText") Marketplace is temporarily unavailable
            p(class="mt-0.5") Local and already installed packages remain available. {{ catalogWarning }}
      div(v-if="packageDiscoveryErrors.length" class="mb-4 rounded-xl border border-amber-500/30 bg-amber-500/10 p-4 text-xs text-amber-300")
        div(class="flex items-start gap-2")
          Icon(icon="heroicons:exclamation-triangle" class="mt-0.5 h-4 w-4 shrink-0")
          div
            p(class="font-semibold") {{ packageDiscoveryErrors.length }} package folder{{ packageDiscoveryErrors.length === 1 ? '' : 's' }} could not be loaded
            p(class="mt-1 text-amber-200/80") Fix the manifest and refresh. Broken packages remain disabled.
            details(v-for="item in packageDiscoveryErrors" :key="item.folder" class="mt-2")
              summary(class="cursor-pointer font-medium") {{ item.folder }}
              p(class="mt-1 break-all font-mono text-[10px]") {{ item.error }}
      // Workshop tab content
      div(v-if="currentTab === 'workshop'" class="space-y-6 pb-12")
        // Header
        div(class="flex flex-col md:flex-row md:items-center justify-between gap-4 pb-2 border-b border-themeBorder/40")
          div
            h2(class="text-xl font-bold text-themeText tracking-tight") Plugin Workshop
            p(class="text-xs text-themeText3 mt-0.5") Scaffold custom plugins and widgets with production-ready templates
          div(class="flex items-center gap-2")
            button(
              @click="currentTab = 'workshop'"
              class="px-3.5 py-1.5 bg-themeBackground border border-themeBorder hover:bg-themeBackground2 text-themeText rounded-xl text-xs font-medium transition-colors flex items-center gap-1.5"
            )
              Icon(icon="heroicons:arrow-up-tray" class="w-3.5 h-3.5 text-themeText2")
              span Import Repository
            button(
              @click="currentTab = 'workshop'"
              class="px-3.5 py-1.5 bg-themeBackground border border-themeBorder hover:bg-themeBackground2 text-themeText rounded-xl text-xs font-medium transition-colors flex items-center gap-1.5"
            )
              Icon(icon="heroicons:book-open" class="w-3.5 h-3.5 text-themeText2")
              span API Reference

        // Template grid
        div(class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4")
          div(
            v-for="template in pluginTemplates"
            :key="template.id"
            @click="selectTemplate(template)"
            class="bg-themeBackground2 rounded-2xl p-4 border border-themeBorder hover:border-themePrimary/40 transition-all cursor-pointer shadow-xs flex flex-col justify-between group"
          )
            div
              // Card Header with Icon & Badges
              div(class="flex items-start justify-between gap-3 mb-3")
                div(class="w-10 h-10 rounded-xl bg-themePrimary/10 border border-themePrimary/20 flex items-center justify-center text-themePrimary shrink-0 group-hover:scale-105 transition-transform")
                  Icon(:icon="template.icon" class="w-5 h-5")
                div(class="flex flex-wrap gap-1.5 justify-end")
                  span(
                    :class="[ 'px-2 py-0.5 rounded-full text-[10px] font-semibold border', template.difficulty === 'beginner' ? 'bg-emerald-500/12 text-emerald-500 border-emerald-500/25' : template.difficulty === 'intermediate' ? 'bg-amber-500/12 text-amber-500 border-amber-500/25' : 'bg-purple-500/12 text-purple-400 border-purple-500/25' ]"
                  ) {{ template.difficulty }}
                  span(class="px-2 py-0.5 rounded-full text-[10px] font-medium bg-themeBackground border border-themeBorder text-themeText3") {{ template.estimatedTime }}

              // Title & Description
              h3(class="font-bold text-sm text-themeText group-hover:text-themePrimary transition-colors truncate") {{ template.name }}
              p(class="text-xs text-themeText2 mt-1 mb-3 line-clamp-2 leading-relaxed") {{ template.description }}

              // Feature Tags
              div(class="flex flex-wrap gap-1.5 mb-4")
                span(
                  v-for="feature in template.features"
                  :key="feature"
                  class="px-2 py-0.5 rounded-md text-[10px] font-medium bg-themeBackground border border-themeBorder text-themeText3"
                ) {{ feature }}

            // Action button
            button(class="w-full py-2 px-3 bg-themeBackground border border-themeBorder group-hover:bg-themeButton group-hover:text-white group-hover:border-transparent text-themeText rounded-xl text-xs font-semibold transition-all flex items-center justify-center gap-1.5")
              span Use Template
              Icon(icon="heroicons:arrow-right" class="w-3.5 h-3.5")

      // Browse and Installed tabs content
      div(v-else)
        // Search and filter bar (for browse tab)
        div(v-if="currentTab === 'browse'" class="mb-6 flex flex-col md:flex-row gap-3 items-center")
          div(class="relative w-full md:w-1/3")
            div(class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none")
              Icon(icon="heroicons:magnifying-glass" class="w-4 h-4 text-themeText3")
            input(
              v-model="searchQuery"
              type="text"
              placeholder="Search plugins & extensions..."
              class="w-full bg-themeBackground border border-themeBorder text-themeText placeholder-themeText3 text-xs rounded-xl pl-9 pr-4 py-2 focus:outline-none focus:ring-1 focus:ring-themeButton focus:border-themeButton transition-all"
            )
          
          div(class="flex items-center gap-2.5 w-full md:w-auto")
            // Category filter
            select(
              v-model="categoryFilter"
              class="bg-themeBackground border border-themeBorder text-themeText text-xs rounded-xl px-3 py-2 focus:outline-none focus:ring-1 focus:ring-themeButton cursor-pointer"
            )
              option(value="all") All Categories
              option(value="integration") Integration
              option(value="productivity") Productivity
              option(value="utility") Utility
              option(value="widget") Widgets
              option(value="automation") Automation
              option(value="finance") Finance
            
            // Price filter
            select(
              v-model="priceFilter"
              class="bg-themeBackground border border-themeBorder text-themeText text-xs rounded-xl px-3 py-2 focus:outline-none focus:ring-1 focus:ring-themeButton cursor-pointer"
            )
              option(value="all") All Prices
              option(value="free") Free Only
              option(value="paid") Paid Only
              option(value="installed") Installed Only

            button(
              @click="fetchPlugins"
              class="p-2 bg-themeBackground border border-themeBorder hover:bg-themeBackground2 text-themeText rounded-xl transition-colors cursor-pointer"
              title="Refresh plugins"
            )
              IcBaselineRefresh(class="w-4 h-4 text-themeText2")

        // Loading state
        div(v-if="loading" class="flex justify-center items-center py-12")
          div(class="animate-spin rounded-full h-8 w-8 border-2 border-themePrimary border-t-transparent")

        // Error state
        div(v-else-if="error" class="bg-red-500/10 border border-red-500/30 p-4 rounded-xl text-red-400 text-xs")
          p {{ error }}
          button(@click="fetchPlugins" class="mt-2 px-3 py-1.5 bg-red-500/20 hover:bg-red-500/30 text-red-300 rounded-lg text-xs font-medium") Try Again

        // Browse Tab - Plugin Store
        div(v-else-if="currentTab === 'browse'")
          // Empty state
          div(v-if="storePlugins.length === 0" class="flex flex-col items-center justify-center py-12 text-center")
            Icon(icon="heroicons:puzzle-piece" class="w-12 h-12 text-themeText3 mb-3")
            h3(class="text-base font-semibold text-themeText mb-1") No plugins available
            p(class="text-xs text-themeText3") Check back later for new plugins

          // No results after filtering
          div(v-else-if="filteredPlugins.length === 0" class="flex flex-col items-center justify-center py-12 text-center")
            Icon(icon="heroicons:magnifying-glass" class="w-12 h-12 text-themeText3 mb-3")
            h3(class="text-base font-semibold text-themeText mb-1") No plugins found
            p(class="text-xs text-themeText3") Try adjusting your search or filters

          // Plugin grid
          div(v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5 pb-16")
            template(v-for="plugin in filteredPlugins" :key="plugin.id")
              div(
                v-if="plugin && plugin.icon && plugin.name"
                class="store-card rounded-2xl overflow-hidden border border-themeBorder bg-themeBackground2 hover:border-themePrimary/40 transition-all cursor-pointer flex flex-col p-4 shadow-xs"
                @click="openPluginDetails(plugin)"
              )
                div(class="flex flex-col h-full")
                  div(class="flex items-start gap-3.5 mb-3")
                    img(
                      :src="plugin.icon"
                      :alt="plugin.name"
                      class="w-12 h-12 rounded-xl object-cover border border-themeBorder shadow-2xs"
                      @error="handleIconError($event, plugin)"
                    )
                    div(class="flex-1 min-w-0")
                      div(class="flex items-center justify-between mb-1")
                        h3(class="font-bold text-sm text-themeText truncate") {{ plugin.name }}
                        span(class="px-2 py-0.5 rounded-full text-[10px] font-semibold bg-themeBackground text-themeText2 border border-themeBorder")
                          span(v-if="plugin.price && plugin.price !== 'Free' && plugin.price !== 0") {{ typeof plugin.price === 'string' ? plugin.price : '$' + plugin.price.toFixed(2) }}
                          span(v-else) Free
                      
                      div(class="flex flex-wrap gap-1.5 mb-1")
                        span(class="px-2 py-0.5 rounded-md text-[10px] uppercase font-semibold tracking-wider bg-themeBackground text-themeText3 border border-themeBorder") {{ plugin.category }}
                        span(v-if="packageInfoById[plugin.id]?.is_granted && !packageInfoById[plugin.id]?.needs_reconsent" class="px-2 py-0.5 rounded-md text-[10px] font-semibold bg-themePrimary/12 text-themePrimary border border-themePrimary/20") ✓ Installed
                        span(v-if="packageInfoById[plugin.id]?.needs_reconsent" class="px-2 py-0.5 rounded-md text-[10px] bg-amber-500/15 text-amber-400 font-semibold border border-amber-500/25" title="This package changed since you granted access — review it") ⚠ Update review
                        span(v-else-if="packageInfoById[plugin.id] && !plugin.installed" class="px-2 py-0.5 rounded-md text-[10px] bg-purple-500/15 text-purple-400 font-semibold border border-purple-500/25") Requires access

                  p(class="text-xs text-themeText3 mb-3.5 line-clamp-2 leading-relaxed") {{ plugin.description }}

                  div(class="flex items-center justify-between text-xs text-themeText3 mb-3.5 mt-auto")
                    div(class="flex items-center gap-2")
                      div(class="flex items-center gap-1 text-amber-400 font-medium bg-themeBackground px-2 py-0.5 rounded-full border border-themeBorder text-[11px]")
                        Icon(icon="heroicons:star" class="w-3 h-3")
                        span {{ plugin.rating }}
                      div(class="flex items-center gap-1 text-themeText3 bg-themeBackground px-2 py-0.5 rounded-full border border-themeBorder text-[11px]")
                        Icon(icon="heroicons:arrow-down-tray" class="w-3 h-3")
                        span {{ formatDownloads(plugin.downloads) }}

                  div(class="flex gap-2")
                    button(
                      v-if="isPackageBusy(plugin.id)"
                      disabled
                      class="flex-1 py-2 px-4 bg-themeButton/70 text-white rounded-xl text-xs font-semibold cursor-wait inline-flex items-center justify-center gap-2"
                    )
                      span(class="h-3.5 w-3.5 rounded-full border-2 border-white border-t-transparent animate-spin")
                      span Working…
                    button(
                      v-else-if="packageInfoById[plugin.id]?.needs_reconsent"
                      @click.stop="installPlugin(plugin)"
                      class="flex-1 py-2 px-4 bg-amber-600 hover:bg-amber-700 text-white rounded-xl text-xs font-semibold transition-colors"
                      title="Re-review the changed access"
                    )
                      span ⚠ Review update
                    button(
                      v-else-if="plugin.updateAvailable"
                      @click.stop="installPlugin(plugin)"
                      class="store-primary-btn flex-1 py-2 px-4 text-white rounded-xl text-xs font-semibold transition-colors"
                      title="Install the latest verified marketplace release"
                    )
                      span Update to v{{ plugin.version }}
                    button(
                      v-else-if="packageInfoById[plugin.id] && !packageInfoById[plugin.id]?.is_granted"
                      @click.stop="installPlugin(plugin)"
                      class="flex-1 py-2 px-4 bg-themeButton hover:bg-themeButtonHover text-white rounded-xl text-xs font-semibold transition-colors"
                    )
                      span Review access
                    button(
                      v-else-if="packageInfoById[plugin.id]?.has_runtime && packageInfoById[plugin.id]?.runtime_active === false && plugin.installed"
                      @click.stop="retryPackageActivation(plugin)"
                      class="flex-1 py-2 px-4 bg-themeWarningBg text-themeWarningIcon border border-themeWarningIcon/25 rounded-xl text-xs font-semibold transition-colors"
                    )
                      span Retry activation
                    button(
                      v-else-if="!plugin.installed"
                      :disabled="!packageInfoById[plugin.id] && !catalogById[plugin.id]?.latest_release"
                      @click.stop="installPlugin(plugin)"
                      class="store-primary-btn flex-1 py-2 px-4 text-white rounded-xl text-xs font-semibold transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                    )
                      span {{ (!packageInfoById[plugin.id] && !catalogById[plugin.id]?.latest_release) ? 'Unavailable' : ((plugin.price && plugin.price !== 'Free' && plugin.price !== 0) ? 'Buy & Install' : 'Install') }}
                    button(
                      v-else
                      disabled
                      class="flex-1 py-2 px-4 bg-themePrimary/10 border border-themePrimary/20 text-themePrimary rounded-xl opacity-90 cursor-default text-xs font-semibold flex items-center justify-center gap-1.5"
                    )
                      span ✓ Installed
                    button(
                      @click.stop="openPluginDetails(plugin)"
                      class="px-3.5 py-2 bg-themeBackground border border-themeBorder hover:bg-themeBackground2 text-themeText rounded-xl text-xs font-medium transition-colors"
                    )
                      span Details

        // Installed Tab - Plugin Management
        div(v-else-if="currentTab === 'installed'")
          // Empty state
          div(v-if="installedPlugins.length === 0" class="flex flex-col items-center justify-center py-12 text-center")
            div(class="text-5xl mb-3") 🔌
            h3(class="text-base font-semibold text-themeText mb-1") No plugins installed
            p(class="text-xs text-themeText3 mb-5") Install plugins from the Marketplace tab to get started
            button(
              @click="currentTab = 'browse'"
              class="px-5 py-2 bg-themeButton hover:bg-themeButtonHover text-white rounded-xl text-xs font-semibold transition-colors"
            )
              span Browse Marketplace

          // Installed plugins list
          div(v-else class="space-y-3.5")
            template(v-for="plugin in installedPlugins" :key="plugin.id")
              div(
                v-if="plugin && plugin.icon && plugin.name"
                class="bg-themeBackground2 rounded-2xl p-5 border border-themeBorder shadow-xs hover:border-themeBorder2 transition-all flex items-start gap-4"
              )
                img(
                  :src="plugin.icon"
                  :alt="plugin.name"
                  class="w-14 h-14 rounded-xl object-cover border border-themeBorder shrink-0"
                  @error="handleIconError($event, plugin)"
                )
                div(class="flex-1 min-w-0")
                  div(class="flex items-center justify-between mb-1.5")
                    h3(class="font-bold text-base text-themeText truncate") {{ plugin.name }}
                    div(class="flex items-center gap-2")
                      span(v-if="packageInfoById[plugin.id]?.trust === 'builtin'" class="px-2 py-0.5 rounded-full text-[11px] font-semibold bg-themePrimary/12 text-themePrimary border border-themePrimary/20") Built in
                      span(v-else-if="packageInfoById[plugin.id]?.needs_reconsent" class="px-2 py-0.5 rounded-full text-[11px] font-semibold bg-amber-500/15 text-amber-400 border border-amber-500/25" title="This package changed since you granted access — review it") ⚠ Update review
                      span(v-else-if="packageInfoById[plugin.id]?.has_runtime && packageInfoById[plugin.id]?.runtime_active === false" class="px-2 py-0.5 rounded-full text-[11px] font-semibold bg-amber-500/15 text-amber-400 border border-amber-500/25" title="Access is granted, but this runtime is not loaded") Granted · inactive
                      span(v-else-if="packageInfoById[plugin.id]?.is_granted" class="px-2 py-0.5 rounded-full text-[11px] font-semibold bg-themePrimary/12 text-themePrimary border border-themePrimary/20") ✓ Active
                      span(v-else class="px-2 py-0.5 rounded-full text-[11px] font-semibold bg-purple-500/15 text-purple-400 border border-purple-500/25") Access review required
                      span(v-if="plugin.version" class="px-2 py-0.5 rounded-full text-[11px] font-medium bg-themeBackground border border-themeBorder text-themeText3") v{{ plugin.version }}

                  p(class="text-xs text-themeText2 mb-3 line-clamp-2 leading-relaxed") {{ plugin.description }}

                  div(class="flex items-center justify-between pt-2 border-t border-themeBorder/40")
                    div(class="flex items-center gap-4 text-xs text-themeText3")
                      span Author: {{ plugin.developer || plugin.author || 'Unknown' }}
                      span Category: {{ plugin.category }}

                    div(class="flex gap-2")
                      button(
                        v-if="packageInfoById[plugin.id]?.needs_reconsent"
                        @click="installPlugin(plugin)"
                        class="px-3.5 py-1.5 bg-amber-600 hover:bg-amber-700 text-white rounded-lg text-xs font-semibold transition-colors"
                        title="Re-review the changed access"
                      )
                        span ⚠ Review update
                      button(
                        v-else-if="plugin.updateAvailable"
                        @click="installPlugin(plugin)"
                        class="px-3.5 py-1.5 bg-themeButton hover:bg-themeButtonHover text-white rounded-lg text-xs font-semibold transition-colors"
                      )
                        span Update to v{{ plugin.version }}
                      button(
                        v-else-if="packageInfoById[plugin.id] && !packageInfoById[plugin.id]?.is_granted"
                        @click="installPlugin(plugin)"
                        class="px-3.5 py-1.5 bg-themeButton hover:bg-themeButtonHover text-white rounded-lg text-xs font-semibold transition-colors"
                      )
                        span Review access
                      button(
                        v-else-if="packageInfoById[plugin.id]?.has_runtime && packageInfoById[plugin.id]?.runtime_active === false"
                        @click="retryPackageActivation(plugin)"
                        class="px-3.5 py-1.5 bg-themeWarningBg text-themeWarningIcon border border-themeWarningIcon/25 rounded-lg text-xs font-semibold transition-colors"
                      )
                        span Retry activation
                      button(
                        @click="openPluginSettings(plugin)"
                        class="px-3.5 py-1.5 bg-themeBackground border border-themeBorder hover:bg-themeBackground2 text-themeText rounded-lg text-xs font-medium transition-colors"
                      )
                        span Settings
                      button(
                        v-if="packageInfoById[plugin.id]?.trust !== 'builtin'"
                        @click="requestPluginUninstall(plugin)"
                        class="px-3.5 py-1.5 bg-red-500/10 hover:bg-red-500/20 text-red-500 border border-red-500/20 rounded-lg text-xs font-semibold transition-colors"
                      )
                        span Uninstall

  // Plugin details modal
  ModalPluginDetails(
    :isOpen="showDetailsModal"
    :plugin="selectedPlugin"
    :pkg="packageInfoById[selectedPlugin?.id]"
    @close="closeDetailsModal"
    @install="handleInstallPlugin"
    @configure="handleConfigurePlugin"
    @purchase="handlePurchasePlugin"
  )

  // Plugin settings modal
  ModalPluginSettings(
    :isOpen="showSettingsModal"
    :plugin="selectedPlugin"
    :pkg="packageInfoById[selectedPlugin?.id]"
    @close="closeSettingsModal"
    @uninstall="handleSettingsUninstall"
  )

  // Uninstall confirmation and transactional lifecycle progress
  BaseModal(:is-open="!!pendingUninstall" :z-index="12080" @close="closeUninstallModal")
    div(class="vd-modal-card w-full max-w-md")
      div(class="flex items-start justify-between gap-4 p-6 border-b border-themeBorder")
        div(class="min-w-0")
          p(class="text-xs font-semibold uppercase tracking-wider text-themeError") Remove package
          h2(class="mt-1 text-lg font-semibold text-themeText truncate") Uninstall {{ pendingUninstall?.name }}?
        button(
          @click="closeUninstallModal"
          :disabled="uninstalling"
          class="vd-modal-close-btn disabled:opacity-40 disabled:cursor-not-allowed"
          aria-label="Close"
        ) ×

      div(class="p-6")
        p(class="text-sm leading-relaxed text-themeText2")
          | This removes the package, stops all of its runtime contributions, and revokes its saved access. If any step fails, VortiDeck restores the previous package state.

        div(v-if="uninstalling" class="mt-4 rounded-xl border border-themeButton/25 bg-themeButton/10 p-3")
          div(class="flex items-center gap-3")
            span(class="h-4 w-4 shrink-0 rounded-full border-2 border-themeButton border-t-transparent animate-spin")
            div(class="min-w-0")
              p(class="text-sm font-medium text-themeText") Uninstalling package…
              p(class="mt-0.5 text-xs text-themeText2") {{ uninstallLifecycle?.message || 'Preparing transactional removal…' }}
          div(class="mt-3 h-1.5 overflow-hidden rounded-full bg-themeBackground3")
            div(class="h-full w-full rounded-full bg-themeButton animate-pulse")

        div(v-else-if="uninstallError" class="mt-4 rounded-xl border border-themeError/25 bg-themeError/10 p-3")
          p(class="text-sm font-medium text-themeError") Uninstall failed
          p(class="mt-1 text-xs leading-relaxed text-themeText2 break-words") {{ uninstallError }}

      div(class="vd-modal-footer flex items-center justify-end gap-3 p-6")
        button(
          @click="closeUninstallModal"
          :disabled="uninstalling"
          class="px-4 py-2 rounded-lg text-sm font-medium text-themeText2 hover:text-themeText disabled:opacity-40 disabled:cursor-not-allowed"
        ) Cancel
        button(
          @click="confirmPluginUninstall"
          :disabled="uninstalling"
          class="inline-flex items-center gap-2 rounded-lg border border-themeError/30 bg-themeError/15 px-4 py-2 text-sm font-semibold text-themeError hover:bg-themeError/20 disabled:opacity-60 disabled:cursor-wait"
        )
          span(v-if="uninstalling" class="h-3.5 w-3.5 rounded-full border-2 border-themeError border-t-transparent animate-spin")
          span {{ uninstalling ? 'Uninstalling…' : (uninstallError ? 'Try again' : 'Uninstall package') }}

  // Plugin creation modal
  BaseModal(:is-open="!!selectedTemplate" :z-index="12070" @close="closeModal")
    div(class="vd-modal-card w-full max-w-2xl max-h-[90vh] overflow-y-auto")
      div(class="flex justify-between items-center p-6 border-b border-themeBorder")
        h2(class="text-xl font-semibold text-themeText") Create Plugin: {{ selectedTemplate.name }}
        button(@click="closeModal" class="vd-modal-close-btn") ×

      div(class="p-6")
        div(class="flex items-start gap-4 mb-6")
          div(class="w-12 h-12 rounded-xl bg-themePrimary/10 border border-themePrimary/20 flex items-center justify-center text-themePrimary shrink-0")
            Icon(:icon="selectedTemplate.icon" class="w-6 h-6")
          div(class="flex-1 min-w-0")
            p(class="text-xs text-themeText2 mb-3 leading-relaxed") {{ selectedTemplate.description }}
            
            div(class="grid grid-cols-2 gap-4 text-sm")
              div
                span(class="text-themeText3") Difficulty: 
                span(class="text-themeText") {{ selectedTemplate.difficulty }}
              div
                span(class="text-themeText3") Time: 
                span(class="text-themeText") {{ selectedTemplate.estimatedTime }}

        div(class="mb-6")
          h3(class="font-semibold text-themeText mb-3") Features:
          ul(class="space-y-1")
            li(v-for="feature in selectedTemplate.features" :key="feature" class="text-sm text-themeText3") • {{ feature }}

      div(class="p-6 border-t border-themeBorder flex justify-end")
        button(@click="generatePlugin(selectedTemplate)" class="px-6 py-2 bg-themeButton hover:bg-themeButtonHover text-white rounded-xl text-xs font-semibold transition-colors")
          span Generate Plugin
</template>

<script setup lang="ts">
import BaseModal from "../../../components/Modal/BaseModal.vue";
import IcBaselineRefresh from "~icons/ic/baseline-refresh?width=24px&height=24px";
import PixelarticonsSearch from "~icons/pixelarticons/search?width=24px&height=24px";
import { Icon } from "@iconify/vue";

import { ref, computed, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useI18n } from "vue-i18n";
import { useCapabilityDialog } from "../../../composables/useCapabilityDialog";
import {
  type PackageInfo,
  widgetContributions,
} from "../../../../desktop-streaming/composables/overlay/packageManifest";

const { t } = useI18n();

// Unified package surface: the global capability dialog (consent gate) + a map
// of packages from `list_packages` (widget packages, etc.) keyed by id, so
// install/uninstall drive the real grant/revoke instead of the legacy mock.
const capabilityDialog = useCapabilityDialog();
const packageInfoById = ref<Record<string, PackageInfo>>({});
interface PackageLifecycleUi {
  package_id: string;
  phase: string;
  progress: number;
  message: string;
  error?: string | null;
  needs_review: boolean;
  runtime_active: boolean;
}
const packageLifecycleById = ref<Record<string, PackageLifecycleUi>>({});
const pendingUninstall = ref<Plugin | null>(null);
const uninstalling = ref(false);
const uninstallError = ref<string | null>(null);
const uninstallLifecycle = computed(() => {
  const id = pendingUninstall.value?.id;
  return id ? packageLifecycleById.value[canonicalPackageId(id)] : undefined;
});
interface PackageDiscoveryError {
  folder: string;
  manifest_path: string;
  error: string;
}
const packageDiscoveryErrors = ref<PackageDiscoveryError[]>([]);

interface MarketplaceReleaseSummary {
  release_id: string;
  version: string;
  artifact_sha256: string;
  artifact_size: number;
  published_at?: string | null;
}

interface MarketplaceCatalogPackage {
  package_id: string;
  slug: string;
  name: string;
  summary: string;
  description: string;
  category: string;
  author: string;
  trust: string;
  is_free: boolean;
  icon_url: string;
  support_url: string;
  website_url: string;
  rating: number | null;
  reviews: number;
  downloads: number;
  latest_release: MarketplaceReleaseSummary | null;
  updated_at: string;
}

interface PluginLibraryItem {
  package_id: string;
  catalog: MarketplaceCatalogPackage | null;
  local: PackageInfo | null;
  installation: {
    installed_version: string;
    source: "builtin" | "marketplace" | "local";
    release_id?: string | null;
    lifecycle_state: string;
  } | null;
}

interface PluginLibraryResponse {
  items: PluginLibraryItem[];
  catalog_error: string | null;
}

const catalogById = ref<Record<string, MarketplaceCatalogPackage>>({});
const catalogWarning = ref<string | null>(null);

// Runtime registries namespace executable instances as `plugin.<package-id>`.
// Marketplace, consent, and settings always use the manifest's package id.
const canonicalPackageId = (id: string) => id.replace(/^plugin\./, "");
const fallbackPluginIcon = (name: string) => {
  const initials = name.trim().slice(0, 3).toUpperCase() || "VD";
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="128" height="128" viewBox="0 0 128 128"><rect width="128" height="128" rx="28" fill="#6d3df5"/><text x="64" y="73" text-anchor="middle" font-family="system-ui,sans-serif" font-size="34" font-weight="700" fill="white">${initials.replace(/[<>&"']/g, "")}</text></svg>`;
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
};
const terminalLifecyclePhases = new Set([
  "active",
  "awaiting_review",
  "granted_inactive",
  "rolled_back",
  "unavailable",
  "uninstalled",
  "failed",
]);
const isPackageBusy = (id: string) => {
  const lifecycle = packageLifecycleById.value[canonicalPackageId(id)];
  return !!lifecycle && !terminalLifecyclePhases.has(lifecycle.phase);
};

// Plugin interfaces
interface Plugin {
  id: string;
  name: string;
  description: string;
  fullDescription?: string;
  icon: string;
  category: string;
  price: number | string;
  originalPrice?: string;
  rating: number;
  reviews: number;
  downloads: number;
  installed: boolean;
  updateAvailable?: boolean;
  version: string;
  author?: string;
  developer?: string;
  lastUpdated: string;
  screenshots: string[];
  featured?: boolean;
  verified?: boolean;
  trialAvailable?: boolean;
  features?: string[];
  changelog?: string;
  supportUrl?: string;
  websiteUrl?: string;
}

interface PluginTemplate {
  id: string;
  name: string;
  description: string;
  category: string;
  difficulty: "beginner" | "intermediate" | "advanced";
  icon: string;
  estimatedTime: string;
  features: string[];
}

const currentTab = ref("browse");
const currentPage = ref(1);
const pageSize = 12;

// Tab configuration
const tabList = ref([
  { key: "browse", label: "Marketplace" },
  { key: "installed", label: "Installed" },
  { key: "workshop", label: "Workshop" },
]);

// Plugin state
const plugins = ref<Plugin[]>([]);
const loading = ref(true);
const error = ref<string | null>(null);
const selectedPlugin = ref<Plugin | null>(null);
const showDetailsModal = ref(false);

// Workshop state
const showCreateWorkshop = ref(false);
const selectedTemplate = ref<PluginTemplate | null>(null);
const workshopStep = ref(1);
const pluginCreationStep = ref("config");
const pluginConfig = ref({
  name: "",
  description: "",
  category: "productivity",
  author: "",
  version: "1.0.0",
});

// Filter state for search/filtering functionality
const searchQuery = ref("");
const categoryFilter = ref("all");
const priceFilter = ref("all");

// Computed properties for filtering browse tab plugins
const filteredPlugins = computed(() => {
  let filtered = storePlugins.value.filter(
    (plugin) => plugin && plugin.name && plugin.icon,
  );

  if (searchQuery.value) {
    filtered = filtered.filter(
      (plugin) =>
        plugin &&
        plugin.name &&
        plugin.description &&
        (plugin.name.toLowerCase().includes(searchQuery.value.toLowerCase()) ||
          plugin.description
            .toLowerCase()
            .includes(searchQuery.value.toLowerCase())),
    );
  }

  if (categoryFilter.value !== "all") {
    filtered = filtered.filter(
      (plugin) => plugin && plugin.category === categoryFilter.value,
    );
  }

  if (priceFilter.value === "free") {
    filtered = filtered.filter(
      (plugin) => plugin && (plugin.price === 0 || plugin.price === "Free"),
    );
  } else if (priceFilter.value === "paid") {
    filtered = filtered.filter(
      (plugin) =>
        plugin && plugin.price && plugin.price !== "Free" && plugin.price !== 0,
    );
  } else if (priceFilter.value === "installed") {
    filtered = filtered.filter((plugin) => plugin && plugin.installed);
  }

  return filtered;
});

// Separate computed properties for Browse and Installed tabs
const storePlugins = computed(() => {
  // For Browse tab - show all available plugins with preference for uninstalled
  return plugins.value
    .filter((plugin) => plugin && plugin.name && plugin.icon)
    .sort((a, b) => {
      // Uninstalled plugins first
      if (a.installed && !b.installed) return 1;
      if (!a.installed && b.installed) return -1;
      return 0;
    });
});

const installedPlugins = computed(() => {
  // For Installed tab - only show installed plugins
  return plugins.value.filter(
    (plugin) => plugin && plugin.installed && plugin.name && plugin.icon,
  );
});

// Pagination computed properties
const totalPages = computed(() => {
  const currentData =
    currentTab.value === "browse" ? storePlugins.value : installedPlugins.value;
  return Math.ceil(currentData.length / pageSize);
});

// Helper functions
const formatDownloads = (downloads: number) => {
  if (downloads >= 1000000) {
    return (downloads / 1000000).toFixed(1) + "M";
  } else if (downloads >= 1000) {
    return (downloads / 1000).toFixed(1) + "K";
  }
  return downloads.toString();
};

const changePage = (page: number) => {
  currentPage.value = page;
};

// Plugin templates
const pluginTemplates = ref<PluginTemplate[]>([
  {
    id: "text-processor",
    name: "Text Processor",
    description: "Process and transform text data with various operations",
    category: "TEXT_PROCESSING",
    difficulty: "beginner",
    icon: "heroicons:document-text",
    estimatedTime: "10 min",
    features: ["Transformation", "Regex", "Format conversion"],
  },
  {
    id: "api-connector",
    name: "API Connector",
    description: "Connect to external REST APIs and cloud services",
    category: "INTEGRATIONS",
    difficulty: "intermediate",
    icon: "heroicons:globe-alt",
    estimatedTime: "20 min",
    features: ["HTTP Requests", "Bearer Auth", "Data Mapping"],
  },
  {
    id: "data-transformer",
    name: "Data Transformer",
    description: "Transform data between JSON, CSV, and XML payloads",
    category: "DATA_PROCESSING",
    difficulty: "beginner",
    icon: "heroicons:arrow-path",
    estimatedTime: "15 min",
    features: ["JSON/CSV/XML", "Validation", "Schema Mapping"],
  },
  {
    id: "file-processor",
    name: "File Processor",
    description:
      "Process files, handle watch events, and manage local filesystem operations",
    category: "FILE_OPERATIONS",
    difficulty: "intermediate",
    icon: "heroicons:folder",
    estimatedTime: "25 min",
    features: ["File I/O", "Watchers", "Path Routing"],
  },
  {
    id: "webhook-listener",
    name: "Webhook Listener",
    description:
      "Listen for incoming webhook events and trigger automation pipelines",
    category: "AUTOMATION",
    difficulty: "beginner",
    icon: "heroicons:bolt",
    estimatedTime: "10 min",
    features: ["HTTP Payloads", "Secret Verify", "Event Triggers"],
  },
  {
    id: "system-monitor",
    name: "System Monitor",
    description: "Read real-time CPU/GPU telemetry and hardware sensors",
    category: "HARDWARE",
    difficulty: "advanced",
    icon: "heroicons:cpu-chip",
    estimatedTime: "30 min",
    features: ["Hardware Stats", "Watchdog", "Gauges & Alerts"],
  },
]);

const pluginCardFromLibraryItem = (item: PluginLibraryItem): Plugin | null => {
  const local = item.local;
  const catalog = item.catalog;
  if (!local && !catalog) return null;
  const id = canonicalPackageId(item.package_id);
  const name = catalog?.name || local?.name || id;
  const isWidget = local ? widgetContributions(local).length > 0 : false;
  let icon = catalog?.icon_url || fallbackPluginIcon(name);
  if (local?.icon_path) {
    try {
      icon = convertFileSrc(local.icon_path.replace(/\\/g, "/"));
    } catch (iconError) {
      console.warn(`Could not load icon for package '${id}'`, iconError);
    }
  }
  const installation = item.installation;
  return {
    id,
    name,
    description:
      catalog?.summary || local?.description || `${name} package for VortiDeck`,
    fullDescription:
      catalog?.description ||
      local?.description ||
      `${name} package for VortiDeck`,
    icon,
    category:
      catalog?.category?.toLowerCase() ||
      (isWidget ? "widget" : local?.has_runtime ? "plugin" : "package"),
    price: catalog?.is_free === false ? "Unavailable" : 0,
    rating: catalog?.rating || 0,
    reviews: catalog?.reviews || 0,
    downloads: catalog?.downloads || 0,
    installed: !!installation || (local?.is_granted ?? false),
    updateAvailable:
      installation?.source === "marketplace" &&
      !!catalog?.latest_release &&
      installation.release_id !== catalog.latest_release.release_id,
    version: catalog?.latest_release?.version || local?.version || "Unreleased",
    author: catalog?.author || local?.author || "Unknown",
    lastUpdated:
      catalog?.updated_at?.split("T")[0] || "Local development package",
    screenshots: [],
    verified: catalog?.trust === "verified" || catalog?.trust === "builtin",
    supportUrl: catalog?.support_url,
    websiteUrl: catalog?.website_url,
  };
};

let pluginRefreshGeneration = 0;
let pluginRefreshTimer: ReturnType<typeof setTimeout> | null = null;

// Fetch the single backend projection. Background lifecycle refreshes keep the
// current cards visible so install/uninstall completion never flashes the grid.
const fetchPlugins = async (
  showInitialLoading = plugins.value.length === 0,
) => {
  const generation = ++pluginRefreshGeneration;
  if (showInitialLoading) loading.value = true;
  error.value = null;

  try {
    const library = await invoke<PluginLibraryResponse>("list_plugin_library");
    if (generation !== pluginRefreshGeneration) return;
    if (library.catalog_error) {
      console.warn("Marketplace catalog unavailable", library.catalog_error);
      catalogWarning.value =
        "The marketplace is temporarily unavailable. Local and installed packages remain usable.";
    } else {
      catalogWarning.value = null;
    }
    catalogById.value = {};
    packageInfoById.value = {};
    for (const item of library.items) {
      const id = canonicalPackageId(item.package_id);
      if (item.catalog) catalogById.value[id] = item.catalog;
      if (item.local) packageInfoById.value[id] = item.local;
    }
    plugins.value = library.items
      .map(pluginCardFromLibraryItem)
      .filter((plugin): plugin is Plugin => plugin !== null);
    packageDiscoveryErrors.value = await invoke(
      "list_package_discovery_errors",
    );
  } catch (err) {
    if (generation !== pluginRefreshGeneration) return;
    console.error("Failed to fetch plugins:", err);
    error.value = "Failed to load plugins. Please try again.";
  } finally {
    if (generation === pluginRefreshGeneration && showInitialLoading) {
      loading.value = false;
    }
  }
};

const schedulePluginRefresh = () => {
  if (pluginRefreshTimer) clearTimeout(pluginRefreshTimer);
  pluginRefreshTimer = setTimeout(() => {
    pluginRefreshTimer = null;
    void fetchPlugins(false);
  }, 150);
};

let unlistenPluginChanges: (() => void) | null = null;
let unlistenPackageLifecycle: (() => void) | null = null;
const { $posthog } = useNuxtApp() as ReturnType<typeof useNuxtApp> & {
  $posthog?: {
    capture: (event: string, properties?: Record<string, unknown>) => void;
  };
};

// Initialize plugins on mount
onMounted(async () => {
  // 📊 Track store page view
  $posthog?.capture("store_page_viewed", { section: "plugins" });

  // First, let's see what directory the backend thinks is the plugins directory
  try {
    const pluginsDir = await invoke("get_plugins_directory");
    console.log("🔍 Backend plugins directory:", pluginsDir);
  } catch (err) {
    console.error("Failed to get plugins directory:", err);
  }

  await fetchPlugins();

  // Listen for plugin changes (hot reloading)
  unlistenPluginChanges = await listen("plugin-changed", (event) => {
    console.log("🔄 Plugin changed:", event.payload);

    // The lifecycle event is the commit boundary and owns list refreshes.
    // Raw watcher events are intentionally diagnostic only.
    const changeEvent = event.payload as {
      event_type: string;
      plugin_id?: string;
    };
    if (changeEvent.event_type === "added") {
      console.log("✅ Plugin added automatically");
    } else if (changeEvent.event_type === "removed") {
      console.log("❌ Plugin removed automatically");
    } else if (changeEvent.event_type === "modified") {
      console.log("🔄 Plugin reloaded automatically");
    }
  });
  unlistenPackageLifecycle = await listen<PackageLifecycleUi>(
    "package-lifecycle",
    async ({ payload }) => {
      const packageId = canonicalPackageId(payload.package_id);
      packageLifecycleById.value[packageId] = {
        ...payload,
        package_id: packageId,
      };
      if (terminalLifecyclePhases.has(payload.phase)) {
        schedulePluginRefresh();
      }
    },
  );
});

onUnmounted(() => {
  if (unlistenPluginChanges) {
    unlistenPluginChanges();
  }
  if (unlistenPackageLifecycle) {
    unlistenPackageLifecycle();
  }
  if (pluginRefreshTimer) {
    clearTimeout(pluginRefreshTimer);
    pluginRefreshTimer = null;
  }
});

// Plugin actions
const installPlugin = async (plugin: Plugin) => {
  const catalog = catalogById.value[plugin.id];
  const release = catalog?.latest_release;
  if (plugin.updateAvailable) {
    if (!release) {
      error.value = `${plugin.name} has no published compatible update.`;
      return;
    }
    try {
      await invoke<string>("install_marketplace_release", {
        releaseId: release.release_id,
        packageId: plugin.id,
      });
      await fetchPlugins();
      const updated = packageInfoById.value[plugin.id];
      if (!updated) {
        throw new Error("The verified update could not be discovered.");
      }
      if (updated.needs_reconsent) {
        capabilityDialog.open(updated, async () => {
          $posthog?.capture("plugin_updated", {
            plugin_name: plugin.name,
            version: release.version,
          });
          await fetchPlugins();
        });
      } else {
        $posthog?.capture("plugin_updated", {
          plugin_name: plugin.name,
          version: release.version,
        });
      }
    } catch (err) {
      console.error("Marketplace package update failed:", err);
      error.value = err instanceof Error ? err.message : String(err);
    }
    return;
  }

  // Real packages (from list_packages) flow through the unified capability
  // dialog — the single consent gate. The backend persists one package grant;
  // we then refresh so the row flips to "installed".
  const pkg = packageInfoById.value[plugin.id];
  if (pkg) {
    capabilityDialog.open(pkg, async () => {
      plugin.installed = true;
      $posthog?.capture("plugin_installed", { plugin_name: plugin.name });
      await fetchPlugins();
    });
    return;
  }

  if (!catalog || !release) {
    error.value = `${plugin.name} has no published compatible release.`;
    return;
  }
  if (!catalog.is_free) {
    error.value = `${plugin.name} is not available for purchase yet.`;
    return;
  }
  try {
    await invoke<string>("install_marketplace_release", {
      releaseId: release.release_id,
      packageId: plugin.id,
    });
    await fetchPlugins();
    const staged = packageInfoById.value[plugin.id];
    if (!staged) {
      throw new Error(
        "The verified package was staged but could not be discovered.",
      );
    }
    capabilityDialog.open(staged, async () => {
      plugin.installed = true;
      $posthog?.capture("plugin_installed", {
        plugin_name: plugin.name,
        source: "marketplace",
        version: release.version,
      });
      await fetchPlugins();
    });
  } catch (err) {
    console.error("Marketplace package installation failed:", err);
    error.value = err instanceof Error ? err.message : String(err);
  }
};

const retryPackageActivation = async (plugin: Plugin) => {
  if (isPackageBusy(plugin.id)) return;
  try {
    await invoke("grant_package_permissions", { packageId: plugin.id });
    await fetchPlugins();
  } catch (err) {
    console.error("Failed to activate package runtime:", err);
    error.value = err instanceof Error ? err.message : String(err);
  }
};

const requestPluginUninstall = (plugin: Plugin) => {
  const packageInfo = packageInfoById.value[plugin.id];
  if (packageInfo?.trust === "builtin") return;
  pendingUninstall.value = plugin;
  uninstallError.value = null;
};

const closeUninstallModal = () => {
  if (uninstalling.value) return;
  const packageId = pendingUninstall.value?.id;
  if (packageId)
    delete packageLifecycleById.value[canonicalPackageId(packageId)];
  pendingUninstall.value = null;
  uninstallError.value = null;
};

const confirmPluginUninstall = async () => {
  const plugin = pendingUninstall.value;
  if (!plugin || uninstalling.value) return;

  uninstalling.value = true;
  uninstallError.value = null;
  try {
    // One transactional backend path removes every contribution, stops all
    // runtimes, revokes durable consent, and rolls back on partial failure.
    await invoke("uninstall_plugin", { id: plugin.id });
    await fetchPlugins();
    delete packageLifecycleById.value[canonicalPackageId(plugin.id)];
    pendingUninstall.value = null;
    console.log(`Uninstalled plugin: ${plugin.name}`);
  } catch (err) {
    console.error("Failed to uninstall plugin:", err);
    uninstallError.value = err instanceof Error ? err.message : String(err);
    await fetchPlugins();
  } finally {
    uninstalling.value = false;
  }
};

const closeModal = () => {
  selectedPlugin.value = null;
  selectedTemplate.value = null;
};

const showSettingsModal = ref(false);

// Plugin details modal handlers
const openPluginSettings = (plugin: Plugin) => {
  selectedPlugin.value = plugin;
  showSettingsModal.value = true;
};

const closeSettingsModal = () => {
  selectedPlugin.value = null;
  showSettingsModal.value = false;
};

const handleSettingsUninstall = (plugin: Plugin) => {
  showSettingsModal.value = false;
  selectedPlugin.value = null;
  requestPluginUninstall(plugin);
};

const openPluginDetails = (plugin: Plugin) => {
  selectedPlugin.value = plugin;
  showDetailsModal.value = true;
};

const closeDetailsModal = () => {
  selectedPlugin.value = null;
  showDetailsModal.value = false;
};

const handleInstallPlugin = async () => {
  if (selectedPlugin.value) {
    await installPlugin(selectedPlugin.value);
    closeDetailsModal();
  }
};

const handleConfigurePlugin = () => {
  // Navigate to plugin configuration or show configuration modal
  console.log("Configure plugin:", selectedPlugin.value?.name);
  closeDetailsModal();
};

const handlePurchasePlugin = () => {
  // Do not simulate a completed purchase or invoke a native confirmation.
  // Paid delivery will use the future marketplace checkout/provenance service.
  if (selectedPlugin.value) {
    error.value = `${selectedPlugin.value.name} is not available for purchase yet.`;
  }
  closeDetailsModal();
};

const openScreenshot = (screenshot: string) => {
  // Open screenshot in full screen or larger view
  window.open(screenshot, "_blank");
};

// Handle icon loading errors - fallback to 3-letter theme avatar
const handleIconError = (event: Event, plugin: Plugin) => {
  const img = event.target as HTMLImageElement;
  const initials = encodeURIComponent(
    plugin.name.substring(0, 3).toUpperCase(),
  );
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" viewBox="0 0 100 100"><rect width="100" height="100" rx="20" fill="%232c333f"/><text x="50" y="58" font-family="system-ui, sans-serif" font-size="28" font-weight="bold" fill="%23e6edf3" text-anchor="middle">${initials}</text></svg>`;
  img.src = `data:image/svg+xml;utf8,${svg}`;
};

// Plugin creation functions
const startPluginCreation = (template: PluginTemplate) => {
  selectedTemplate.value = template;
  pluginCreationStep.value = "config";
};

const selectTemplate = (template: PluginTemplate) => {
  selectedTemplate.value = template;
};

const generatePlugin = async (config: any) => {
  try {
    loading.value = true;

    // This would call a backend service to generate the plugin
    const result = await invoke("generate_plugin_from_template", {
      templateId: selectedTemplate.value?.id,
      config: config,
    });

    console.log("Generated plugin:", result);
    pluginCreationStep.value = "test";
  } catch (err) {
    console.error("Failed to generate plugin:", err);
    alert("Failed to generate plugin. Please try again.");
  } finally {
    loading.value = false;
  }
};
</script>

<style lang="scss" scoped>
@use "../../../components/Modal/modalTheme.scss" as *;

.deck-store-page {
  background: hsl(var(--background));
  color: hsl(var(--text));
}

.store-header {
  background: hsl(var(--background) / 0.95);
  border-bottom: 1px solid hsl(var(--border));
  backdrop-filter: blur(8px);
}

.store-tab {
  background: transparent;
}

.store-tab--idle {
  color: hsl(var(--text2));
  border-bottom-color: transparent;
}

.store-tab--idle:hover {
  color: hsl(var(--text));
  border-bottom-color: hsl(var(--border2));
}

.store-tab--active {
  color: hsl(var(--primary));
  border-bottom-color: hsl(var(--primary));
  font-weight: 600;
}

.store-input {
  background: hsl(var(--background2));
  border: 1px solid hsl(var(--border));
  color: hsl(var(--text));
  transition:
    border-color 0.2s ease,
    box-shadow 0.2s ease;
}

.store-input:focus {
  border-color: hsl(var(--button));
  box-shadow: 0 0 0 2px hsl(var(--button) / 0.16);
}

.store-soft-btn {
  border: 1px solid hsl(var(--border));
  background: hsl(var(--background));
  color: hsl(var(--text));
  transition:
    background 0.2s ease,
    border-color 0.2s ease;
}

.store-soft-btn:hover {
  background: hsl(var(--background2));
  border-color: hsl(var(--border2));
}

.store-primary-btn {
  background: hsl(var(--button));
  color: #ffffff;
  transition:
    background 0.2s ease,
    filter 0.2s ease;
}

.store-primary-btn:hover {
  background: hsl(var(--buttonHover));
}

.store-card {
  background: hsl(var(--background2));
  border-color: hsl(var(--border));
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.04);
}

.store-card:hover {
  box-shadow: 0 8px 20px rgba(0, 0, 0, 0.08);
  border-color: hsl(var(--primary) / 0.4);
}

.store-pill {
  background: hsl(var(--background3));
}

.plugins-content-scroll {
  scrollbar-gutter: stable;

  &::-webkit-scrollbar {
    width: 6px;
    height: 6px;
    display: block;
  }
  &::-webkit-scrollbar-track {
    background: transparent;
  }
  &::-webkit-scrollbar-thumb {
    background: hsl(var(--border2));
    border-radius: 999px;
  }
  &::-webkit-scrollbar-thumb:hover {
    background: hsl(var(--text3) / 0.5);
  }
  &::-webkit-scrollbar-button {
    display: none !important;
    width: 0 !important;
    height: 0 !important;
  }
}
</style>
