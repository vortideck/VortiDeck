<template lang="pug">
Teleport(to="body")
  Transition(name="ctx-menu-fade")
    div(
      v-if="isVisible"
      ref="contextMenu"
      :style="{ top: position.y + 'px', left: position.x + 'px' }"
      class="custom-ctx-menu"
      @click.stop
    )
      div(
        v-for="item in items"
        :key="item.id"
        @click="handleItemClick(item)"
        class="custom-ctx-item group"
        :class="{ 'custom-ctx-item--danger': item.id === 'delete' || item.id === 'remove' }"
      )
        div(class="custom-ctx-icon-wrap" :class="{ 'custom-ctx-icon-wrap--danger': item.id === 'delete' || item.id === 'remove' }")
          SolarPen2Linear(v-if="item.id === 'edit'" class="w-4 h-4 text-blue-400 group-hover:scale-110 transition-transform")
          SolarCopyLinear(v-else-if="item.id === 'copy' || item.id === 'duplicate'" class="w-4 h-4 text-purple-400 group-hover:scale-110 transition-transform")
          SolarTrashBinTrashLinear(v-else-if="item.id === 'delete' || item.id === 'remove'" class="w-4 h-4 text-red-400 group-hover:scale-110 transition-transform")
          SolarSquareShareLineLinear(v-else-if="item.id === 'open-in-new-window'" class="w-4 h-4 text-emerald-400 group-hover:scale-110 transition-transform")
          SolarBoltLinear(v-else class="w-4 h-4 text-slate-400 group-hover:scale-110 transition-transform")

        span(class="custom-ctx-label") {{ item.label }}
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import SolarPen2Linear from "~icons/solar/pen-2-linear?width=20px&height=20px";
import SolarCopyLinear from "~icons/solar/copy-linear?width=20px&height=20px";
import SolarTrashBinTrashLinear from "~icons/solar/trash-bin-trash-linear?width=20px&height=20px";
import SolarSquareShareLineLinear from "~icons/solar/square-share-line-linear?width=20px&height=20px";
import SolarBoltLinear from "~icons/solar/bolt-linear?width=20px&height=20px";

interface ContextMenuItem {
  id: string;
  label: string;
  action: () => void;
}

interface Props {
  items: ContextMenuItem[];
  isVisible: boolean;
  position: { x: number; y: number };
}

const props = defineProps<Props>();
const emit = defineEmits(['close']);

const contextMenu = ref<HTMLElement>();

const handleItemClick = (item: ContextMenuItem) => {
  item.action();
  emit('close');
};

const handleClickOutside = (event: MouseEvent) => {
  if (contextMenu.value && !contextMenu.value.contains(event.target as Node)) {
    emit('close');
  }
};

const handleEscape = (event: KeyboardEvent) => {
  if (event.key === 'Escape') {
    emit('close');
  }
};

onMounted(() => {
  document.addEventListener('click', handleClickOutside);
  document.addEventListener('keydown', handleEscape);
});

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside);
  document.removeEventListener('keydown', handleEscape);
});
</script>

<style scoped lang="scss">
.custom-ctx-menu {
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

.custom-ctx-item {
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
    }
  }
}

.custom-ctx-icon-wrap {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 1.5rem;
  height: 1.5rem;
  flex-shrink: 0;
}

.custom-ctx-label {
  flex: 1 1 auto;
  white-space: nowrap;
  line-height: 1.2;
}

.ctx-menu-fade-enter-active,
.ctx-menu-fade-leave-active {
  transition: opacity 0.14s cubic-bezier(0.16, 1, 0.3, 1), transform 0.14s cubic-bezier(0.16, 1, 0.3, 1);
}

.ctx-menu-fade-enter-from,
.ctx-menu-fade-leave-to {
  opacity: 0;
  transform: scale(0.96) translateY(-4px);
}
</style>