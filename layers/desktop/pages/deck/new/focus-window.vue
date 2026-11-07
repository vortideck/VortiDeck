<template lang="pug">
div(class="h-screen flex flex-col overflow-hidden")
  div(class="flex-1 w-full max-w-[46rem] mx-auto px-6 pb-6 overflow-y-auto")

    DeckEditorActionPageHeader(
      class="focus-action-header"
      eyebrow="Utilities"
      :title="`${windowId ? 'Update' : 'Create'} focus window action`"
      description="Bring a matching application window to the foreground when triggered."
    )
      template(v-slot:icon)
        WindowIcon(class="w-6 h-6")

    // Form
    div(class="focus-action-stack")

      // Window Selection Card
      div(class="focus-action-body bg-themeBackground2 p-5 border border-themeBorder2")
        div(class="flex items-center justify-between mb-4")
          h3(class="text-lg font-semibold text-themeText flex items-center gap-2")
            CursorArrowRaysIcon(class="w-5 h-5 text-themeButton")
            | Window selection
          div(class="flex items-center gap-2")
            label(class="inline-flex items-center gap-2 text-xs text-themeText2 cursor-pointer")
              input(type="checkbox" v-model="autoRefresh" class="h-4 w-4 rounded border-themeBorder2 bg-themeBackground3")
              span Auto-refresh
            button(
              @click="loadWindows"
              :disabled="loadingWindows"
              class="px-3 py-1.5 text-sm bg-themeButton/20 text-themeButton rounded-lg hover:bg-themeButton/30 disabled:opacity-50 transition-all"
            ) {{ loadingWindows ? 'Loading...' : 'Refresh Windows' }}

        div(class="space-y-4")
          // Action Name
          div
            label(class="block text-sm font-medium text-themeText2 mb-2") Action Name
            input(
              v-model="actionName"
              class="w-full px-4 py-3 bg-themeBackground border border-themeBorder2 rounded-lg text-themeText focus:outline-none focus:ring-2 focus:ring-themeButton"
              placeholder="Focus Chrome Browser"
            )

          // Matching mode
          div(class="rounded-lg border border-themeBorder2 bg-themeBackground/40 p-4 space-y-3")
            div(class="flex items-center justify-between gap-4")
              p(class="text-sm font-medium text-themeText") Matching
              select(
                v-model="matchMode"
                class="px-3 py-1.5 text-sm bg-themeBackground border border-themeBorder2 rounded-lg text-themeText focus:outline-none focus:ring-2 focus:ring-themeButton"
              )
                option(v-for="option in matchModeOptions" :key="option.value" :value="option.value") {{ option.label }}

            div(class="grid grid-cols-1 md:grid-cols-2 gap-3")
              div
                label(class="block text-xs uppercase tracking-wide text-themeText3 mb-1") Window title hint
                input(
                  v-model="windowTitle"
                  :disabled="matchMode === 'process_only'"
                  class="w-full px-3 py-2 bg-themeBackground border border-themeBorder2 rounded-lg text-themeText disabled:opacity-50 disabled:cursor-not-allowed focus:outline-none focus:ring-2 focus:ring-themeButton"
                  :placeholder="matchMode === 'regex' ? 'e.g. ^Discord(?!.*Settings).*' : 'e.g. Discord or OBS'"
                )
              div
                label(class="block text-xs uppercase tracking-wide text-themeText3 mb-1") Process name
                input(
                  v-model="processName"
                  class="w-full px-3 py-2 bg-themeBackground border border-themeBorder2 rounded-lg text-themeText focus:outline-none focus:ring-2 focus:ring-themeButton"
                  placeholder="e.g. discord or obs64.exe"
                )

            p(class="text-xs text-themeText3") {{ selectedModeDescription }}

          // Window picker dropdown
          div(ref="dropdownContainerRef" class="relative")
            label(class="block text-xs uppercase tracking-wide text-themeText3 mb-1") Pick from running windows
            div(
              @click="toggleDropdown"
              :class="['w-full flex items-center gap-2 px-3 py-2 bg-themeBackground border rounded-lg cursor-pointer transition-all', dropdownOpen ? 'border-themeButton ring-2 ring-themeButton/30' : 'border-themeBorder2 hover:border-themeBorder']"
            )
              svg(class="w-4 h-4 shrink-0" :class="selectedWindow && !dropdownOpen ? 'text-themeButton' : 'text-themeText3'" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                path(v-if="selectedWindow && !dropdownOpen" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7")
                path(v-else stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z")
              //- When dropdown is open: show search input
              input(
                v-show="dropdownOpen"
                ref="searchInputRef"
                v-model="windowSearch"
                type="text"
                placeholder="Search by title or process..."
                class="flex-1 bg-transparent text-themeText text-sm outline-none placeholder-themeText3"
                @focus="dropdownOpen = true"
                @keydown.escape="dropdownOpen = false"
              )
              //- When dropdown is closed: show selected window or placeholder
              div(v-show="!dropdownOpen" class="flex-1 min-w-0")
                p(v-if="selectedWindow" class="text-sm text-themeButton truncate") {{ selectedWindow.title || selectedWindow.process_name }}
                p(v-else class="text-sm text-themeText3") Search by title or process...
              button(v-if="selectedWindow && !dropdownOpen" @click.stop="clearSelection" class="text-themeText3 hover:text-themeText transition-colors shrink-0")
                svg(class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                  path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")
              span(class="text-xs text-themeText3 shrink-0") {{ availableWindows.length }}
              svg(:class="['w-4 h-4 text-themeText3 shrink-0 transition-transform', dropdownOpen ? 'rotate-180' : '']" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7")

            // Dropdown panel (opens upward)
            Transition(name="dropdown")
              div(v-if="dropdownOpen" class="absolute z-50 left-0 right-0 bottom-full mb-1 bg-themeBackground2 border border-themeBorder2 rounded-lg shadow-xl overflow-hidden")
                div(v-if="lastRefreshAt" class="px-3 py-1.5 border-b border-themeBorder2 flex items-center justify-between")
                  p(class="text-xs text-themeText3") {{ filteredWindows.length }} of {{ availableWindows.length }} windows
                  p(class="text-xs text-themeText3") Updated {{ lastRefreshAt }}

                div(class="max-h-64 overflow-y-auto")
                  div(
                    v-for="(win, index) in filteredWindows"
                    :key="`${win.process_id}-${win.title}-${index}`"
                    @click="selectWindowFromDropdown(win)"
                    :class="['px-3 py-2.5 cursor-pointer transition-all border-b border-themeBorder2/50 last:border-b-0', isSelectedWindow(win) ? 'bg-themeButton/15' : 'hover:bg-themeBackground']"
                  )
                    div(class="flex items-center gap-2")
                      div(class="flex-1 min-w-0")
                        p(class="text-sm font-medium text-themeText truncate") {{ win.title || '(untitled)' }}
                        p(class="text-xs text-themeText3") {{ win.process_name }}
                      div(v-if="isSelectedWindow(win)")
                        svg(class="w-4 h-4 text-themeButton" fill="currentColor" viewBox="0 0 20 20")
                          path(fill-rule="evenodd" d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z" clip-rule="evenodd")

                div(v-if="filteredWindows.length === 0 && availableWindows.length > 0" class="px-3 py-6 text-center")
                  p(class="text-sm text-themeText3") No windows match your search

                div(v-else-if="availableWindows.length === 0 && !loadingWindows" class="px-3 py-6 text-center")
                  p(class="text-sm text-themeText3") No windows loaded
                  button(@click="loadWindows" class="mt-1 text-xs text-themeButton hover:text-themeButton") Refresh now

    // Action Buttons
    div(class="focus-action-footer flex gap-3")
      button(
        @click="saveWindow"
        :disabled="!isFormValid"
        class="flex-1 px-6 py-3 bg-themeButton text-white rounded-lg font-medium hover:brightness-105 disabled:opacity-50 disabled:cursor-not-allowed transition-all shadow-lg"
      ) {{ windowId ? 'Update' : 'Save' }} Action

      button(
        @click="$router.back()"
        class="px-6 py-3 bg-themeBackground2 text-themeText2 rounded-lg font-medium hover:bg-themeBackground border border-themeBorder2 transition-all"
      ) Cancel
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, watch, nextTick } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import WindowIcon from '~icons/heroicons/window'
import CursorArrowRaysIcon from '~icons/heroicons/cursor-arrow-rays'

const route = useRoute()
const router = useRouter()

const windowId = ref<number | null>(null)
const copyFromId = ref<number | null>(null)
const actionName = ref('')
const windowTitle = ref('')
const processName = ref('')
const availableWindows = ref<any[]>([])
const selectedWindow = ref<any>(null)
const loadingWindows = ref(false)
const matchMode = ref<'smart' | 'exact' | 'contains' | 'regex' | 'process_only'>('smart')
const windowSearch = ref('')
const autoRefresh = ref(true)
const lastRefreshAt = ref<string | null>(null)
let refreshTimer: ReturnType<typeof setInterval> | null = null

const dropdownOpen = ref(false)
const dropdownContainerRef = ref<HTMLElement | null>(null)
const searchInputRef = ref<HTMLInputElement | null>(null)

const AUTO_REFRESH_MS = 3500

const matchModeOptions = [
  { value: 'smart', label: 'Smart (recommended)', description: 'Best-match scoring using title + process.' },
  { value: 'contains', label: 'Contains', description: 'Title contains hint (case-insensitive).' },
  { value: 'exact', label: 'Exact', description: 'Exact title match (strict).' },
  { value: 'regex', label: 'Regex', description: 'Pattern match for advanced dynamic titles.' },
  { value: 'process_only', label: 'Process only', description: 'Ignore title and match only process.' }
] as const

const selectedModeDescription = computed(() => {
  return matchModeOptions.find((option) => option.value === matchMode.value)?.description || ''
})

const filteredWindows = computed(() => {
  const raw = windowSearch.value.trim().toLowerCase()
  if (!raw) return availableWindows.value

  const tokens = raw.split(/\s+/).filter(Boolean)

  // Score each window: every token must match somewhere in title or process
  const scored: { window: any; score: number }[] = []
  for (const w of availableWindows.value) {
    const title = String(w.title || '').toLowerCase()
    const process = String(w.process_name || '').toLowerCase()
    const combined = title + ' ' + process

    let totalScore = 0
    let allMatch = true
    for (const token of tokens) {
      if (combined.startsWith(token)) {
        totalScore += 3
      } else if (title.startsWith(token) || process.startsWith(token)) {
        totalScore += 2
      } else if (combined.includes(token)) {
        totalScore += 1
      } else {
        allMatch = false
        break
      }
    }

    if (allMatch) {
      scored.push({ window: w, score: totalScore })
    }
  }

  return scored.sort((a, b) => b.score - a.score).map((s) => s.window)
})

const isFormValid = computed(() => {
  const hasActionName = actionName.value.trim() !== ''
  const hasProcess = processName.value.trim() !== ''
  const hasTitle = windowTitle.value.trim() !== ''

  if (!hasActionName) {
    return false
  }

  if (matchMode.value === 'process_only') {
    return hasProcess
  }

  if (matchMode.value === 'regex') {
    return hasTitle
  }

  return hasTitle || hasProcess
})

async function loadWindows() {
  if (loadingWindows.value) {
    return
  }

  try {
    loadingWindows.value = true
    const windows = await invoke<any[]>('get_visible_windows')
    availableWindows.value = windows

    if (selectedWindow.value) {
      const previous = selectedWindow.value
      const nextSelected = matchMode.value === 'process_only'
        ? windows.find((window) => window.process_name === previous.process_name)
        : windows.find((window) => window.process_id === previous.process_id && window.title === previous.title) ||
          windows.find((window) => window.process_name === previous.process_name)

      if (nextSelected) {
        selectedWindow.value = nextSelected
      }
    }

    lastRefreshAt.value = new Date().toLocaleTimeString()
  } catch (error) {
    console.error('Failed to load windows:', error)
  } finally {
    loadingWindows.value = false
  }
}

function selectWindow(window: any) {
  selectedWindow.value = window
  windowTitle.value = window.title
  processName.value = window.process_name
  if (matchMode.value === 'process_only') {
    windowTitle.value = ''
  }
  if (!actionName.value) {
    actionName.value = `Focus ${window.process_name}`
  }
}

function selectWindowFromDropdown(window: any) {
  selectWindow(window)
  dropdownOpen.value = false
  windowSearch.value = ''
}

function clearSelection() {
  selectedWindow.value = null
  windowTitle.value = ''
  processName.value = ''
}

function toggleDropdown() {
  dropdownOpen.value = !dropdownOpen.value
  if (dropdownOpen.value) {
    nextTick(() => searchInputRef.value?.focus())
  }
}

function handleClickOutside(event: MouseEvent) {
  if (!dropdownOpen.value) return
  if (dropdownContainerRef.value?.contains(event.target as Node)) return
  dropdownOpen.value = false
}

function isSelectedWindow(window: any) {
  return selectedWindow.value?.process_id === window.process_id &&
         selectedWindow.value?.title === window.title
}

function startAutoRefresh() {
  stopAutoRefresh()
  refreshTimer = setInterval(() => {
    loadWindows()
  }, AUTO_REFRESH_MS)
}

function stopAutoRefresh() {
  if (refreshTimer) {
    clearInterval(refreshTimer)
    refreshTimer = null
  }
}

watch(autoRefresh, (enabled) => {
  if (enabled) {
    startAutoRefresh()
  } else {
    stopAutoRefresh()
  }
})

watch(matchMode, (mode) => {
  if (mode === 'process_only') {
    windowTitle.value = ''
  }
})

onMounted(async () => {
  // Check for edit mode
  if (route.query.actionId) {
    windowId.value = parseInt(route.query.actionId as string)
    await loadWindow()
  }
  // Check for copy mode
  else if (route.query.copyFrom) {
    copyFromId.value = parseInt(route.query.copyFrom as string)
    windowId.value = copyFromId.value
    await loadWindow()
  }

  await loadWindows()
  if (autoRefresh.value) {
    startAutoRefresh()
  }

  document.addEventListener('mousedown', handleClickOutside, true)
})

onBeforeUnmount(() => {
  stopAutoRefresh()
  document.removeEventListener('mousedown', handleClickOutside, true)
})

async function loadWindow() {
  try {
    const window: any = await invoke('get_focus_window_by_id', { id: windowId.value })
    // If copying, append " (Copy)" to the name
    actionName.value = copyFromId.value ? `${window.name} (Copy)` : window.name
    windowTitle.value = window.window_title
    processName.value = window.process_name
    if (window.match_mode) {
      matchMode.value = window.match_mode
    } else {
      matchMode.value = !window.window_title?.trim() && !!window.process_name?.trim() ? 'process_only' : 'smart'
    }

    // If copying, clear the windowId so it creates a new entry
    if (copyFromId.value) {
      windowId.value = null
    }
  } catch (error) {
    console.error('Failed to load focus window:', error)
  }
}

async function saveWindow() {
  try {
    const command = windowId.value ? 'update_focus_window' : 'create_focus_window'
    const normalizedTitle = matchMode.value === 'process_only' ? '' : windowTitle.value.trim()
    const params: any = {
      name: actionName.value.trim(),
      windowTitle: normalizedTitle,
      processName: processName.value.trim(),
      matchMode: matchMode.value
    }

    if (windowId.value) {
      params.id = windowId.value
    }

    const response: any = await invoke(command, params)

    if (response.status === 200) {
      router.back()
    }
  } catch (error) {
    console.error('Failed to save focus window:', error)
  }
}
</script>

<style scoped>
:deep(.focus-action-header) {
  border-radius: 0.9rem 0.9rem 0 0;
  box-shadow: none;
}

.focus-action-body {
  border-top: 0;
  border-radius: 0;
}

.focus-action-footer {
  justify-content: flex-end;
  padding: 1rem 1.25rem 1.25rem;
  border: 1px solid hsl(var(--border2));
  border-top: 0;
  border-radius: 0 0 0.9rem 0.9rem;
  background: hsl(var(--background2));
}

.dropdown-enter-active,
.dropdown-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}
.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: translateY(4px);
}
</style>
