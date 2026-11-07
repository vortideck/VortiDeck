<template lang="pug">
Teleport(to="body")
  Transition(name="global-ctx-menu")
    div(
      v-if="globalContextMenu.isVisible"
      ref="contextMenu"
      :style="{ top: globalContextMenu.position.y + 'px', left: globalContextMenu.position.x + 'px' }"
      class="global-ctx-menu"
      @click.stop
    )
      template(v-for="item in globalContextMenu.items" :key="item.id")
        div(v-if="item.divider" class="global-ctx-divider")
        div(
          v-else
          @click="handleItemClick(item)"
          class="global-ctx-item group"
          :class="{ 'global-ctx-item--danger': isDangerItem(item), 'global-ctx-item--disabled': item.disabled }"
        )
          div(class="global-ctx-icon-wrap" :class="{ 'global-ctx-icon-wrap--danger': isDangerItem(item) }")
            component(v-if="item.icon" :is="item.icon" class="w-4 h-4")
            SolarPen2Linear(v-else-if="item.id === 'edit'" class="w-4 h-4 text-blue-400 group-hover:scale-110 transition-transform")
            SolarCopyLinear(v-else-if="item.id === 'copy' || item.id === 'duplicate'" class="w-4 h-4 text-purple-400 group-hover:scale-110 transition-transform")
            SolarTrashBinTrashLinear(v-else-if="item.id === 'delete' || item.id === 'remove'" class="w-4 h-4 text-red-400 group-hover:scale-110 transition-transform")
            SolarSquareShareLineLinear(v-else-if="item.id === 'open-in-new-window'" class="w-4 h-4 text-emerald-400 group-hover:scale-110 transition-transform")
            SolarBoltLinear(v-else class="w-4 h-4 text-slate-400 group-hover:scale-110 transition-transform")

          span(class="global-ctx-label") {{ item.label }}
          span(v-if="item.shortcut" class="global-ctx-shortcut") {{ item.shortcut }}
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useGlobalContextMenu, type ContextMenuItem } from '../composables/useGlobalContextMenu'
import SolarPen2Linear from "~icons/solar/pen-2-linear?width=20px&height=20px"
import SolarCopyLinear from "~icons/solar/copy-linear?width=20px&height=20px"
import SolarTrashBinTrashLinear from "~icons/solar/trash-bin-trash-linear?width=20px&height=20px"
import SolarSquareShareLineLinear from "~icons/solar/square-share-line-linear?width=20px&height=20px"
import SolarBoltLinear from "~icons/solar/bolt-linear?width=20px&height=20px"

const { globalContextMenu, handleItemClick, handleClickOutside, handleEscape } = useGlobalContextMenu()

const contextMenu = ref<HTMLElement>()

const isDangerItem = (item: ContextMenuItem) => {
  return item.danger === true || item.id === 'delete' || item.id === 'remove'
}

onMounted(() => {
  document.addEventListener('click', handleClickOutside)
  document.addEventListener('keydown', handleEscape)
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
  document.removeEventListener('keydown', handleEscape)
})
</script>

<style lang="scss" scoped>
.global-ctx-menu {
  position: fixed;
  z-index: 99999;
  min-width: 13rem;
  padding: 0.32rem;
  border-radius: 0.85rem;
  border: 1px solid hsl(var(--border2) / 0.85);
  background: hsl(var(--background) / 0.95);
  box-shadow: 0 20px 48px -8px hsl(var(--text) / 0.25), 0 0 0 1px hsl(var(--border2) / 0.4), inset 0 1px 0 hsl(var(--text) / 0.08);
  backdrop-filter: blur(24px);
  -webkit-backdrop-filter: blur(24px);
  display: flex;
  flex-direction: column;
  gap: 0.12rem;
  user-select: none;
  color: hsl(var(--text));
}

.global-ctx-item {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  padding: 0.45rem 0.6rem;
  border-radius: 0.55rem;
  font-size: 0.8rem;
  font-weight: 550;
  color: hsl(var(--text));
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease;

  &:hover {
    background: hsl(var(--hover));
    color: hsl(var(--text));
  }

  &--danger {
    &:hover {
      background: hsl(var(--error) / 0.14);
      color: hsl(var(--error));

      .global-ctx-icon-wrap {
        color: hsl(var(--error));
      }
    }
  }

  &--disabled {
    opacity: 0.45;
    pointer-events: none;
  }
}

.global-ctx-icon-wrap {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 1.5rem;
  height: 1.5rem;
  flex-shrink: 0;
}

.global-ctx-label {
  flex: 1 1 auto;
  white-space: nowrap;
  line-height: 1.2;
}

.global-ctx-shortcut {
  font-size: 0.68rem;
  font-weight: 600;
  color: hsl(var(--text3));
  margin-left: auto;
}

.global-ctx-divider {
  height: 1px;
  background: hsl(var(--border2) / 0.6);
  margin: 0.18rem 0.2rem;
}

.global-ctx-menu-enter-active,
.global-ctx-menu-leave-active {
  transition: opacity 0.14s cubic-bezier(0.16, 1, 0.3, 1), transform 0.14s cubic-bezier(0.16, 1, 0.3, 1);
}

.global-ctx-menu-enter-from,
.global-ctx-menu-leave-to {
  opacity: 0;
  transform: scale(0.96) translateY(-4px);
}
</style>