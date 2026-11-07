<template lang="pug">
  div.plugins-dropdown(
    @click.stop=""
    class="bg-themeBackground2 rounded-lg shadow-lg border border-themeBorder2 w-64 p-4"
  )
    // Search
    div.search-container(class="flex items-center gap-2 px-3 py-2 bg-themeBackground rounded-lg border border-themeBorder2 mb-4")
      PixelarticonsSearch(class="w-4 h-4 text-gray-400")
      input(
        v-model="searchQuery"
        placeholder="Search plugins..."
        class="bg-transparent outline-none flex-1 text-sm"
        @input="handleSearch"
      )
    
    // Plugin list
    div.plugin-list(class="max-h-64 overflow-y-auto space-y-1")
      div(
        v-for="plugin in filteredPlugins"
        :key="plugin.id"
        @click="selectPlugin(plugin)"
        class="flex items-center gap-3 px-3 py-2 rounded-lg hover:bg-themeBackground cursor-pointer transition-colors"
      )
        component(:is="plugin.icon" class="w-5 h-5")
        span.plugin-name(class="text-sm flex-1") {{ plugin.name }}
        span.plugin-badge(
          v-if="plugin.badge"
          class="text-xs bg-themePrimary text-white px-2 py-0.5 rounded"
        ) {{ plugin.badge }}
    
    // Manage plugins link
    div.manage-plugins(class="mt-4 pt-4 border-t border-themeBorder2")
      NuxtLink(
        to="/deck/store/plugins"
        @click="closeDropdown"
        class="flex items-center justify-center gap-2 px-3 py-2 text-sm text-themePrimary hover:bg-themeBackground rounded-lg transition-colors"
      )
        IcBaselineSettings(class="w-4 h-4")
        span Manage Plugins
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import PixelarticonsSearch from "~icons/pixelarticons/search?width=16px&height=16px";
import IcBaselineSettings from "~icons/ic/baseline-settings?width=16px&height=16px";
import CarbonOverlay from "~icons/carbon/overlay";
import IcBaselineExtension from "~icons/ic/baseline-extension?width=20px&height=20px";
import { invoke } from "@tauri-apps/api/core";
import { usePluginsStore } from "@desktop/stores/plugins";

const emit = defineEmits(["close"]);
const router = useRouter();
const pluginsStore = usePluginsStore();

interface DropdownPlugin {
  id: string;
  name: string;
  icon: any;
  route?: string;
  badge?: string;
  isWebExtension?: boolean;
}

const searchQuery = ref("");

// Combine built-in plugins with web extension plugins from store
const allPlugins = computed((): DropdownPlugin[] => {
  const builtInPlugins: DropdownPlugin[] = [
    {
      id: "vorti-overlay",
      name: "Vorti Overlay",
      icon: CarbonOverlay,
      route: "overlay",
    },
  ];

  // Add web extension plugins from store
  const webExtensionPlugins: DropdownPlugin[] = pluginsStore.webExtensionPlugins.map(
    (p) => ({
      id: p.id,
      name: p.name,
      icon: IcBaselineExtension,

      isWebExtension: true,
    })
  );

  return [...builtInPlugins, ...webExtensionPlugins];
});

const filteredPlugins = computed(() => {
  if (!searchQuery.value) return allPlugins.value;
  return allPlugins.value.filter((plugin) =>
    plugin.name.toLowerCase().includes(searchQuery.value.toLowerCase())
  );
});

function handleSearch() {
  // Search is handled by the computed property
}

async function selectPlugin(plugin: DropdownPlugin) {
  try {
    if (plugin.id === "vorti-overlay") {
      await invoke("open_overlay_window");
      closeDropdown();
    } else if (plugin.isWebExtension) {
      await pluginsStore.openPluginWindow(plugin.id);
      closeDropdown();
    } else if (plugin.route) {
      router.push(plugin.route);
      closeDropdown();
    }
  } catch (error) {
    console.error(`Failed to open ${plugin.name}:`, error);
  }
}

function closeDropdown() {
  emit("close");
}

// Close on click outside
function handleClickOutside(event: MouseEvent) {
  const dropdown = event.currentTarget as HTMLElement;
  if (!dropdown.contains(event.target as Node)) {
    closeDropdown();
  }
}

// Note: Plugins are fetched globally in app.vue during startup
// The store provides cached data with automatic refresh every 5 minutes
onMounted(async () => {
  document.addEventListener("click", handleClickOutside);

  // Fallback: If store is empty, fetch plugins
  // This handles cases where hot-reload skips the splashscreen initialization

  console.log("[Dropdown] Store empty, fetching plugins...");
  await pluginsStore.fetchPlugins();
});

onUnmounted(() => {
  document.removeEventListener("click", handleClickOutside);
});
</script>

<style lang="scss" scoped>
.plugins-dropdown {
  animation: fadeIn 0.2s ease-out;
}

@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(-10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.plugin-list {
  &::-webkit-scrollbar {
    width: 6px;
  }

  &::-webkit-scrollbar-track {
    background: transparent;
  }

  &::-webkit-scrollbar-thumb {
    background-color: rgba(0, 0, 0, 0.2);
    border-radius: 3px;
  }
}
</style>
