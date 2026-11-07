<template lang="pug">
div(class="desktop-picker w-full h-full flex flex-col bg-gray-900 text-white p-4")
  // Header
  div(class="mb-6")
    h2(class="text-2xl font-bold mb-2") Select Desktop
    p(class="text-gray-400") Choose which desktop to control

  // Loading state
  div(v-if="loading" class="flex items-center justify-center flex-1")
    div(class="text-center")
      div(class="w-12 h-12 border-4 border-blue-500/30 border-t-blue-500 rounded-full animate-spin mx-auto mb-4")
      p(class="text-gray-400") Loading desktops...

  // Desktop list
  div(v-else-if="desktops.length > 0" class="flex-1 overflow-y-auto space-y-3")
    div(
      v-for="desktop in desktops"
      :key="desktop.desktop_id"
      @click="selectDesktop(desktop)"
      class="desktop-card bg-gray-800 rounded-lg p-4 border-2 transition-all cursor-pointer"
      :class="[ desktop.online ? 'border-green-500/30 hover:border-green-500 hover:bg-gray-750' : 'border-gray-700 opacity-60 cursor-not-allowed']"
    )
      div(class="flex items-start justify-between")
        // Desktop info
        div(class="flex items-center gap-3 flex-1")
          // Status indicator
          span(class="text-2xl") {{ desktop.online ? '🟢' : '🔴' }}

          // Details
          div(class="flex-1")
            h3(class="font-semibold text-lg") {{ desktop.desktop_name }}
            p(class="text-sm text-gray-400")
              | {{ desktop.metadata?.os || 'Unknown OS' }} • {{ desktop.metadata?.hostname || 'Unknown Host' }}
            p(class="text-xs text-gray-500 mt-1")
              | Last seen: {{ formatTime(desktop.last_seen) }}

        // Connect button
        button(
          v-if="desktop.online"
          class="px-4 py-2 bg-green-600 hover:bg-green-700 rounded-lg text-sm font-medium transition-colors"
        ) Connect
        button(
          v-else
          disabled
          class="px-4 py-2 bg-gray-700 rounded-lg text-sm font-medium cursor-not-allowed"
        ) Offline

  // Empty state
  div(v-else class="flex items-center justify-center flex-1")
    div(class="text-center")
      span(class="text-6xl mb-4 block") 💻
      p(class="text-gray-400 mb-2") No desktops available
      p(class="text-sm text-gray-500") Make sure your desktop app is running

  // Refresh button
  div(class="mt-4 pt-4 border-t border-gray-800")
    button(
      @click="$emit('refresh')"
      class="w-full py-3 bg-blue-600 hover:bg-blue-700 rounded-lg font-medium transition-colors"
    ) 🔄 Refresh List
</template>

<script setup lang="ts">
interface Desktop {
  desktop_id: string;
  desktop_name: string;
  online: boolean;
  last_seen: string;
  metadata?: {
    os?: string;
    hostname?: string;
  };
}

const props = defineProps<{
  desktops: Desktop[];
  loading?: boolean;
}>();

const emit = defineEmits<{
  select: [desktop: Desktop];
  refresh: [];
}>();

const selectDesktop = (desktop: Desktop) => {
  if (desktop.online) {
    emit("select", desktop);
  }
};

const formatTime = (timestamp: string) => {
  try {
    const date = new Date(timestamp);
    const now = new Date();
    const diff = now.getTime() - date.getTime();

    if (diff < 60000) return "Just now";
    if (diff < 3600000) return `${Math.floor(diff / 60000)} min ago`;
    if (diff < 86400000) return `${Math.floor(diff / 3600000)} hours ago`;
    return `${Math.floor(diff / 86400000)} days ago`;
  } catch (error) {
    return "Unknown";
  }
};
</script>

<style scoped>
.desktop-card:active {
  transform: scale(0.98);
}
</style>
