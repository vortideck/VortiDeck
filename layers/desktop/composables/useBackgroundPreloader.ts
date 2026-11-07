import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

type LoadingStatus = 'pending' | 'loading' | 'loaded' | 'error'
type PreloadedItem = Record<string, any>

interface PreloadedData {
  programs: PreloadedItem[] | null
  macros: PreloadedItem[] | null
  flows: PreloadedItem[] | null
  savedActions: Record<string, PreloadedItem[]> | null
  systemControls: PreloadedItem[] | null
  httpRequests: PreloadedItem[] | null
  focusWindows: PreloadedItem[] | null
}

type DataKey = keyof PreloadedData
type MenuKey = 'programs' | 'macros' | 'flows' | 'actions' | 'utilities'
type LoadingProgress = Record<DataKey, LoadingStatus>
type Loader = () => Promise<void>

// Global singleton state that persists across navigation
const globalLoadingProgress = ref<LoadingProgress>({
  programs: 'pending',
  macros: 'pending',
  flows: 'pending',
  savedActions: 'pending',
  systemControls: 'pending',
  httpRequests: 'pending',
  focusWindows: 'pending'
})

const globalPreloadedData = ref<PreloadedData>({
  programs: null,
  macros: null,
  flows: null,
  savedActions: null,
  systemControls: null,
  httpRequests: null,
  focusWindows: null
})

export const useBackgroundPreloader = () => {
  const loadingProgress = globalLoadingProgress
  const preloadedData = globalPreloadedData

  // Programs with icons loaded directly
  const loadProgramsInBackground = async () => {
    try {
      loadingProgress.value.programs = 'loading'

      // Load programs with icons included
      const programs = await invoke<PreloadedItem[]>("get_programs_with_meta", {
        includeIcons: true
      })

      preloadedData.value.programs = programs
      loadingProgress.value.programs = 'loaded'

    } catch (error) {
      loadingProgress.value.programs = 'error'
      console.error('Background programs load failed:', error)
    }
  }

  // Macros loading
  const loadMacrosInBackground = async () => {
    try {
      loadingProgress.value.macros = 'loading'
      const macros = await invoke<PreloadedItem[]>("get_all_macros")
      preloadedData.value.macros = macros
      loadingProgress.value.macros = 'loaded'
    } catch (error) {
      loadingProgress.value.macros = 'error'
      console.error('Background macros load failed:', error)
    }
  }

  // Flows loading
  const loadFlowsInBackground = async () => {
    try {
      loadingProgress.value.flows = 'loading'
      const flows = await invoke<PreloadedItem[]>("list_flows")
      preloadedData.value.flows = flows
      loadingProgress.value.flows = 'loaded'
    } catch (error) {
      loadingProgress.value.flows = 'error'
      console.error('Background flows load failed:', error)
    }
  }

  // Provider-neutral saved integration actions loading
  const loadSavedActionsInBackground = async () => {
    try {
      loadingProgress.value.savedActions = 'loading'
      const grouped = await invoke<Record<string, PreloadedItem[]>>("get_all_saved_integration_actions")
      const entries = await Promise.all(Object.entries(grouped).map(async ([integrationId, actions]) => {
        const health = await invoke<import("@base/types/actionReference").SavedActionPreflight[]>(
          "preflight_saved_integration_actions",
          { integrationId },
        )
        const healthById = new Map(health.map((item) => [item.stableId, item.preflight]))
        return [integrationId, actions.map((action) => ({ ...action, preflight: healthById.get(action.stable_id) }))] as const
      }))
      preloadedData.value.savedActions = Object.fromEntries(entries)
      loadingProgress.value.savedActions = 'loaded'
    } catch (error) {
      loadingProgress.value.savedActions = 'error'
      console.error('Background integration actions load failed:', error)
    }
  }

  // System Controls loading
  const loadSystemControlsInBackground = async () => {
    try {
      loadingProgress.value.systemControls = 'loading'
      const controls = await invoke<PreloadedItem[]>("get_available_actions")
      preloadedData.value.systemControls = controls
      loadingProgress.value.systemControls = 'loaded'
    } catch (error) {
      loadingProgress.value.systemControls = 'error'
      console.error('Background system controls load failed:', error)
    }
  }

  // HTTP Requests loading
  const loadHttpRequestsInBackground = async () => {
    try {
      loadingProgress.value.httpRequests = 'loading'
      const requests = await invoke<PreloadedItem[]>("list_http_requests")
      preloadedData.value.httpRequests = requests
      loadingProgress.value.httpRequests = 'loaded'
    } catch (error) {
      loadingProgress.value.httpRequests = 'error'
      console.error('Background HTTP requests load failed:', error)
    }
  }

  // Focus Windows loading
  const loadFocusWindowsInBackground = async () => {
    try {
      loadingProgress.value.focusWindows = 'loading'
      const windows = await invoke<PreloadedItem[]>("get_focus_windows")
      preloadedData.value.focusWindows = windows
      loadingProgress.value.focusWindows = 'loaded'
    } catch (error) {
      loadingProgress.value.focusWindows = 'error'
      console.error('Background focus windows load failed:', error)
    }
  }

  // Priority-based background preloading with current menu priority
  const startBackgroundPreload = async (currentMenu?: string) => {
    console.log('🚀 Starting background preload...', currentMenu ? `Current menu: ${currentMenu}` : '')

    // Map menu names to loader functions and data keys
    const loaderMap: Record<MenuKey, { loader: Loader; dataKey: DataKey }> = {
      'programs': { loader: loadProgramsInBackground, dataKey: 'programs' },
      'macros': { loader: loadMacrosInBackground, dataKey: 'macros' },
      'flows': { loader: loadFlowsInBackground, dataKey: 'flows' },
      'actions': {
        loader: async () => {
          await Promise.all([
            loadingProgress.value.savedActions !== 'loaded' ? loadSavedActionsInBackground() : undefined,
            loadingProgress.value.systemControls !== 'loaded' ? loadSystemControlsInBackground() : undefined,
          ])
        },
        dataKey: 'savedActions'
      },
      'utilities': {
        loader: async () => {
          await Promise.all([
            loadingProgress.value.httpRequests !== 'loaded' ? loadHttpRequestsInBackground() : undefined,
            loadingProgress.value.focusWindows !== 'loaded' ? loadFocusWindowsInBackground() : undefined,
          ])
        },
        dataKey: 'httpRequests' // Use httpRequests as primary check for utilities menu
      }
    }

    // Default priority order (fallback)
    const defaultOrder: MenuKey[] = ['programs', 'macros', 'flows', 'actions', 'utilities']
    const isMenuKey = (value: string): value is MenuKey => value in loaderMap

    // If current menu is specified and not already loaded, load it first
    if (currentMenu && isMenuKey(currentMenu)) {
      const { loader, dataKey } = loaderMap[currentMenu]
      const currentStatus = loadingProgress.value[dataKey]

      if (currentStatus !== 'loaded' && currentStatus !== 'loading') {
        console.log(`🎯 Priority loading for current menu: ${currentMenu}`)
        void loader()
        // Short delay before loading others
        await new Promise(resolve => setTimeout(resolve, 50))
      } else {
        console.log(`✅ Data for ${currentMenu} already available`)
      }
    }

    // Load remaining items in priority order (only if not already loaded)
    const remainingItems = defaultOrder.filter(item => item !== currentMenu)

    for (let i = 0; i < remainingItems.length; i++) {
      const menuType = remainingItems[i]
      if (!menuType) continue
      const { loader, dataKey } = loaderMap[menuType]
      const status = loadingProgress.value[dataKey]

      if (status !== 'loaded' && status !== 'loading') {
        void loader()
        // Staggered loading to prevent overwhelming
        if (i < remainingItems.length - 1) {
          await new Promise(resolve => setTimeout(resolve, 100))
        }
      }
    }
  }

  // Helper to get menu data status
  const getMenuData = (menuType: string) => {
    const loaderMap: Record<MenuKey, DataKey> = {
      programs: 'programs',
      macros: 'macros',
      flows: 'flows',
      actions: 'savedActions',
      utilities: 'httpRequests',
    }
    const dataKey = menuType in loaderMap
      ? loaderMap[menuType as MenuKey]
      : menuType as DataKey
    const data = preloadedData.value[dataKey]
    const status = loadingProgress.value[dataKey]

    return {
      data: data || [],
      isLoading: status === 'loading',
      hasError: status === 'error',
      isLoaded: status === 'loaded',
      isEmpty: status === 'loaded' && (!data || (Array.isArray(data) && data.length === 0))
    }
  }

  // Retry functions for error states
  const retryLoad = (menuType: string) => {
    switch (menuType) {
      case 'programs':
        loadProgramsInBackground()
        break
      case 'macros':
        loadMacrosInBackground()
        break
      case 'flows':
        loadFlowsInBackground()
        break
      case 'savedActions':
        loadSavedActionsInBackground()
        break
      case 'systemControls':
        loadSystemControlsInBackground()
        break
      case 'httpRequests':
        loadHttpRequestsInBackground()
        break
      case 'focusWindows':
        loadFocusWindowsInBackground()
        break
      case 'actions':
        void Promise.all([loadSavedActionsInBackground(), loadSystemControlsInBackground()])
        break
      case 'utilities':
        void Promise.all([loadHttpRequestsInBackground(), loadFocusWindowsInBackground()])
        break
    }
  }

  // Cache invalidation methods
  const invalidateCache = (dataType: string) => {
    if (!(dataType in globalLoadingProgress.value)) return
    const dataKey = dataType as DataKey
    console.log(`🗑️ Invalidating cache for: ${dataType}`)
    globalLoadingProgress.value[dataKey] = 'pending'
    globalPreloadedData.value[dataKey] = null

    // Immediately reload the data
    retryLoad(dataType)
  }

  const invalidateAll = () => {
    console.log('🗑️ Invalidating all cache')
    ;(Object.keys(globalLoadingProgress.value) as DataKey[]).forEach((key) => {
      globalLoadingProgress.value[key] = 'pending'
      globalPreloadedData.value[key] = null
    })
    startBackgroundPreload()
  }

  return {
    loadingProgress,
    preloadedData,
    startBackgroundPreload,
    getMenuData,
    retryLoad,
    invalidateCache,
    invalidateAll,
    // Individual loaders for manual retry
    loadProgramsInBackground,
    loadMacrosInBackground,
    loadFlowsInBackground,
    loadSavedActionsInBackground,
    loadSystemControlsInBackground
  }
}
