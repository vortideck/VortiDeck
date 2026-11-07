import { ref, readonly, type Component } from 'vue'

export interface ContextMenuItem {
  id: string
  label: string
  action: () => void
  icon?: Component | string | Function
  danger?: boolean
  disabled?: boolean
  shortcut?: string
  divider?: boolean
}

export interface ContextMenuState {
  isVisible: boolean
  position: { x: number; y: number }
  items: ContextMenuItem[]
  closeCallback?: () => void
}

// Global state for context menu
const globalContextMenu = ref<ContextMenuState>({
  isVisible: false,
  position: { x: 0, y: 0 },
  items: [],
  closeCallback: undefined
})

export const useGlobalContextMenu = () => {
  const MENU_VIEWPORT_MARGIN = 8
  const MENU_ESTIMATED_WIDTH = 220
  const MENU_ITEM_HEIGHT = 36
  const MENU_VERTICAL_PADDING = 8

  const getSafePosition = (clickX: number, clickY: number, itemCount: number) => {
    const viewportWidth = typeof window !== 'undefined' ? window.innerWidth : 1200
    const viewportHeight = typeof window !== 'undefined' ? window.innerHeight : 800
    const estimatedHeight = itemCount * MENU_ITEM_HEIGHT + MENU_VERTICAL_PADDING

    let x = clickX
    let y = clickY

    // Keep menu fully inside viewport horizontally
    if (x + MENU_ESTIMATED_WIDTH > viewportWidth - MENU_VIEWPORT_MARGIN) {
      x = clickX - MENU_ESTIMATED_WIDTH
    }
    if (x < MENU_VIEWPORT_MARGIN) {
      x = MENU_VIEWPORT_MARGIN
    }

    // Flip upward if there is not enough space below
    if (y + estimatedHeight > viewportHeight - MENU_VIEWPORT_MARGIN) {
      y = clickY - estimatedHeight
    }
    if (y < MENU_VIEWPORT_MARGIN) {
      y = MENU_VIEWPORT_MARGIN
    }

    return { x, y }
  }

  const showContextMenu = (
    event: MouseEvent,
    items: ContextMenuItem[],
    closeCallback?: () => void
  ) => {
    // Close any existing context menu first
    closeContextMenu()

    const safePosition = getSafePosition(event.clientX, event.clientY, items.length)

    globalContextMenu.value = {
      isVisible: true,
      position: safePosition,
      items,
      closeCallback
    }
  }

  const closeContextMenu = () => {
    if (!globalContextMenu.value.isVisible && !globalContextMenu.value.closeCallback) {
      return
    }

    if (globalContextMenu.value.closeCallback) {
      globalContextMenu.value.closeCallback()
    }

    globalContextMenu.value = {
      isVisible: false,
      position: { x: 0, y: 0 },
      items: [],
      closeCallback: undefined
    }
  }

  const handleItemClick = (item: ContextMenuItem) => {
    if (item.disabled) return
    item.action()
    closeContextMenu()
  }

  // Handle click outside and escape key with visibility guards
  const handleClickOutside = () => {
    if (globalContextMenu.value.isVisible) {
      closeContextMenu()
    }
  }

  const handleEscape = (event: KeyboardEvent) => {
    if (event.key === 'Escape' && globalContextMenu.value.isVisible) {
      closeContextMenu()
    }
  }

  return {
    globalContextMenu: readonly(globalContextMenu),
    showContextMenu,
    closeContextMenu,
    handleItemClick,
    handleClickOutside,
    handleEscape
  }
}
