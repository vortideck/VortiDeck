<template lang="pug">
Transition(name="fade")
  div(v-if="show" @click="$emit('close')" class="fixed inset-0 z-50" style="pointer-events: none;")
    div(
      @click.stop
      :style="{ left: position.x + 'px', top: position.y + 'px' }"
      class="absolute bg-white rounded-lg shadow-xl border border-neutral-200 py-2 min-w-48"
      style="pointer-events: auto;"
    )
      button(
        v-for="item in menuItems"
        :key="item.id"
        @click="$emit('menu-action', item.action)"
        class="w-full px-4 py-2 text-left text-sm text-neutral-700 hover:bg-neutral-50 flex items-center gap-2"
      )
        component(:is="item.icon" class="w-4 h-4")
        span {{ item.label }}
</template>

<script setup lang="ts">
interface MenuItem {
  id: string
  label: string
  action: string
  icon: any
}

interface Position {
  x: number
  y: number
}

const props = withDefaults(defineProps<{
  show?: boolean
  position?: Position
  menuItems?: MenuItem[]
}>(), {
  show: false,
  position: () => ({ x: 0, y: 0 }),
  menuItems: () => []
})

const emit = defineEmits<{
  close: []
  'menu-action': [action: string]
}>()
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