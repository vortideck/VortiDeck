<template lang="pug">
div(
  ref="triggerElement"
  class="context-submenu-trigger"
  @pointerenter="openMenu"
  @pointerleave="scheduleClose"
  @focusin="openMenu"
)
  slot(name="trigger" :open="isOpen" :show="openMenu" :toggle="toggleMenu" :close="closeMenu")

Teleport(to="body")
  div(
    v-if="isOpen && bridgeStyle"
    class="context-submenu-bridge"
    :style="bridgeStyle"
    aria-hidden="true"
    @pointerenter="cancelClose"
    @pointerleave="scheduleClose"
  )
  Transition(name="context-submenu")
    div(
      v-if="isOpen"
      ref="menuElement"
      class="context-submenu scrollbar-visible"
      :class="[`context-submenu--${placement.side}`, menuClass]"
      :style="menuStyle"
      data-menu="context-submenu"
      role="menu"
      @pointerenter="cancelClose"
      @pointerleave="scheduleClose"
      @focusin="cancelClose"
      @pointerdown.stop
    )
      slot(:close="closeMenu")
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref } from "vue";
import {
  resolveContextMenuPlacement,
  type ContextMenuPlacement,
  type ContextMenuSide,
} from "@desktop/utils/contextMenuPosition";

const props = withDefaults(defineProps<{
  width?: number;
  gap?: number;
  closeDelay?: number;
  preferredSide?: ContextMenuSide;
  menuClass?: string;
}>(), {
  width: 176,
  gap: 6,
  closeDelay: 140,
  preferredSide: "right",
  menuClass: "",
});

const isOpen = ref(false);
const triggerElement = ref<HTMLElement | null>(null);
const menuElement = ref<HTMLElement | null>(null);
const placement = ref<ContextMenuPlacement>({
  left: 0,
  top: 0,
  maxHeight: 0,
  side: props.preferredSide,
});
const anchorRect = ref<DOMRect | null>(null);
const visibleMenuHeight = ref(0);
let closeTimer: ReturnType<typeof setTimeout> | null = null;

const menuStyle = computed(() => ({
  left: `${placement.value.left}px`,
  top: `${placement.value.top}px`,
  width: `${Math.min(props.width, Math.max(0, typeof window === "undefined" ? props.width : window.innerWidth - 16))}px`,
  maxHeight: `${placement.value.maxHeight}px`,
}));

const bridgeStyle = computed(() => {
  const anchor = anchorRect.value;
  if (!anchor || typeof window === "undefined") return null;
  const menuWidth = Math.min(props.width, Math.max(0, window.innerWidth - 16));
  const menuRight = placement.value.left + menuWidth;
  const left = placement.value.side === "right" ? anchor.right : menuRight;
  const right = placement.value.side === "right" ? placement.value.left : anchor.left;
  const width = Math.max(0, right - left);
  if (!width) return null;
  const top = Math.min(anchor.top, placement.value.top);
  const bottom = Math.max(anchor.bottom, placement.value.top + visibleMenuHeight.value);
  return {
    left: `${left}px`,
    top: `${top}px`,
    width: `${width}px`,
    height: `${Math.max(1, bottom - top)}px`,
  };
});

const cancelClose = () => {
  if (closeTimer) clearTimeout(closeTimer);
  closeTimer = null;
};

const updatePlacement = () => {
  if (!triggerElement.value || !menuElement.value) return;
  const anchor = triggerElement.value.getBoundingClientRect();
  anchorRect.value = anchor;
  visibleMenuHeight.value = Math.min(menuElement.value.scrollHeight, Math.max(0, window.innerHeight - 16));
  placement.value = resolveContextMenuPlacement({
    anchor,
    menuWidth: Math.min(props.width, Math.max(0, window.innerWidth - 16)),
    menuHeight: menuElement.value.scrollHeight,
    viewportWidth: window.innerWidth,
    viewportHeight: window.innerHeight,
    preferredSide: props.preferredSide,
    gap: props.gap,
  });
};

const handleViewportChange = (event?: Event) => {
  const target = event?.target;
  if (
    event?.type === "scroll"
    && target instanceof Element
    && target.closest(".context-menu-floating")
  ) return;
  if (isOpen.value) updatePlacement();
};

const isWithinInteractionTree = (target: EventTarget | null) => (
  target instanceof Node
  && (
    triggerElement.value?.contains(target)
    || menuElement.value?.contains(target)
    || (target instanceof Element && Boolean(target.closest(".context-menu-floating")))
  )
);

const handleDocumentPointerDown = (event: PointerEvent) => {
  if (isWithinInteractionTree(event.target)) return;
  closeMenu();
};

const handleDocumentFocusIn = (event: FocusEvent) => {
  if (isWithinInteractionTree(event.target)) {
    cancelClose();
    return;
  }
  closeMenu();
};

const handleDocumentPointerOver = (event: PointerEvent) => {
  const target = event.target;
  if (target instanceof Element && target.closest(".context-menu-floating")) {
    cancelClose();
  }
};

const handleDocumentKeyDown = (event: KeyboardEvent) => {
  if (event.key === "Escape") closeMenu();
};

const attachListeners = () => {
  window.addEventListener("resize", handleViewportChange);
  window.addEventListener("scroll", handleViewportChange, true);
  document.addEventListener("pointerdown", handleDocumentPointerDown, true);
  document.addEventListener("pointerover", handleDocumentPointerOver, true);
  document.addEventListener("focusin", handleDocumentFocusIn, true);
  document.addEventListener("keydown", handleDocumentKeyDown);
};

const detachListeners = () => {
  window.removeEventListener("resize", handleViewportChange);
  window.removeEventListener("scroll", handleViewportChange, true);
  document.removeEventListener("pointerdown", handleDocumentPointerDown, true);
  document.removeEventListener("pointerover", handleDocumentPointerOver, true);
  document.removeEventListener("focusin", handleDocumentFocusIn, true);
  document.removeEventListener("keydown", handleDocumentKeyDown);
};

const openMenu = async () => {
  cancelClose();
  if (!isOpen.value) {
    isOpen.value = true;
    attachListeners();
  }
  await nextTick();
  updatePlacement();
};

const closeMenu = () => {
  cancelClose();
  if (!isOpen.value) return;
  isOpen.value = false;
  detachListeners();
};

const scheduleClose = () => {
  cancelClose();
  // Portalled controls are outside this component's DOM subtree even though
  // they are part of the same interaction. Keep the parent menu stable for
  // the entire lifetime of an open floating control; scrolling a list can
  // otherwise emit transient pointerout events with no relatedTarget.
  if (document.querySelector(".context-menu-floating")) return;
  closeTimer = setTimeout(() => {
    closeTimer = null;
    // A floating child can mount after focus has already left the trigger.
    // Re-check at execution time so focusing a teleported search input does
    // not close its parent submenu during the mount tick.
    if (document.querySelector(".context-menu-floating")) return;
    closeMenu();
  }, props.closeDelay);
};

const toggleMenu = () => {
  if (isOpen.value) closeMenu();
  else void openMenu();
};

onBeforeUnmount(() => {
  cancelClose();
  detachListeners();
});
</script>

<style scoped>
.context-submenu-trigger {
  position: relative;
  width: 100%;
}

.context-submenu-bridge {
  position: fixed;
  z-index: 99999;
}

.context-submenu {
  position: fixed;
  z-index: 100000;
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: .15rem;
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
  border: 1px solid hsl(var(--border2) / .85);
  border-radius: .8rem;
  background: hsl(var(--background) / .98);
  padding: .32rem;
  color: hsl(var(--text));
  box-shadow: 0 20px 48px -8px hsl(var(--text) / .25), 0 0 0 1px hsl(var(--border2) / .4), inset 0 1px 0 hsl(var(--text) / .08);
  backdrop-filter: blur(24px);
  -webkit-backdrop-filter: blur(24px);
  scrollbar-color: hsl(var(--border2)) transparent;
  scrollbar-width: thin;
}

.context-submenu::-webkit-scrollbar {
  display: block;
  width: 8px;
  height: 8px;
  background: transparent;
}

.context-submenu::-webkit-scrollbar-track,
.context-submenu::-webkit-scrollbar-corner {
  background: transparent;
}

.context-submenu::-webkit-scrollbar-thumb {
  border: 2px solid transparent;
  border-radius: 8px;
  background: hsl(var(--border2));
  background-clip: padding-box;
}

.context-submenu::-webkit-scrollbar-thumb:hover {
  background: hsl(var(--text3) / .6);
  background-clip: padding-box;
}

.context-submenu::-webkit-scrollbar-button,
.context-submenu::-webkit-scrollbar-button:single-button,
.context-submenu::-webkit-scrollbar-button:vertical:decrement,
.context-submenu::-webkit-scrollbar-button:vertical:increment,
.context-submenu::-webkit-scrollbar-button:horizontal:decrement,
.context-submenu::-webkit-scrollbar-button:horizontal:increment {
  display: none;
  width: 0;
  height: 0;
}

.context-submenu-enter-active,
.context-submenu-leave-active {
  transition: opacity .12s ease, transform .12s ease;
  transform-origin: left top;
}

.context-submenu--left.context-submenu-enter-active,
.context-submenu--left.context-submenu-leave-active {
  transform-origin: right top;
}

.context-submenu-enter-from,
.context-submenu-leave-to {
  opacity: 0;
  transform: translateX(.25rem) scale(.98);
}

.context-submenu--left.context-submenu-enter-from,
.context-submenu--left.context-submenu-leave-to {
  transform: translateX(-.25rem) scale(.98);
}
</style>
