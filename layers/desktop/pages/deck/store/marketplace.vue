<template lang="pug">
div(class="deck-store-page deck-store-marketplace w-full h-full flex flex-col px-6")
  div(class="store-header flex flex-wrap gap-x-2")
    button(
      class="store-tab flex items-center gap-2 px-4 py-2 text-sm font-medium rounded-t-lg transition-colors border-b-2 border-transparent hover:border-[#ffd0bb]"
      :class="activeTab === 'iconPacks' ? 'store-tab--active' : 'store-tab--idle'"
      @click="activeTab = 'iconPacks'"
    )
      span Icon Packs
    button(
      class="store-tab flex items-center gap-2 px-4 py-2 text-sm font-medium rounded-t-lg transition-colors border-b-2 border-transparent hover:border-[#ffd0bb]"
      :class="activeTab === 'buttons' ? 'store-tab--active' : 'store-tab--idle'"
      @click="activeTab = 'buttons'"
    )
      span Buttons
    button(
      class="store-tab flex items-center gap-2 px-4 py-2 text-sm font-medium rounded-t-lg transition-colors border-b-2 border-transparent hover:border-[#ffd0bb]"
      :class="activeTab === 'decks' ? 'store-tab--active' : 'store-tab--idle'"
      @click="activeTab = 'decks'"
    )
      span Decks
    button(
      class="store-tab flex items-center gap-2 px-4 py-2 text-sm font-medium rounded-t-lg transition-colors border-b-2 border-transparent hover:border-[#ffd0bb]"
      :class="activeTab === 'actions' ? 'store-tab--active' : 'store-tab--idle'"
      @click="activeTab = 'actions'"
    )
      span Actions
    button(
      class="store-tab flex items-center gap-2 px-4 py-2 text-sm font-medium rounded-t-lg transition-colors border-b-2 border-transparent hover:border-[#ffd0bb]"
      :class="activeTab === 'themes' ? 'store-tab--active' : 'store-tab--idle'"
      @click="activeTab = 'themes'"
    )
      span Themes
    button(
      class="store-tab flex items-center gap-2 px-4 py-2 text-sm font-medium rounded-t-lg transition-colors border-b-2 border-transparent hover:border-[#ffd0bb]"
      :class="activeTab === 'plugins' ? 'store-tab--active' : 'store-tab--idle'"
      @click="activeTab = 'plugins'"
    )
      span Plugins
  
  div(class="flex-1 overflow-auto")
    div(v-if="!authStore.getAccessToken" class="store-empty-state flex flex-col items-center justify-center h-full p-8")
      div(class="text-center max-w-md")
        h2(class="text-2xl font-bold text-themeText mb-4") Login Required
        p(class="text-themeText2 mb-6") You need to be logged in to download themes from the marketplace.
        button(
          @click="openLoginModal" 
          class="store-primary-btn px-6 py-2 text-white rounded-md transition-colors"
        ) Login
    
    div(v-else-if="activeTab === 'themes'" class="w-full h-full p-6 overflow-scroll")
      // Loading state
      div(v-if="loading" class="flex justify-center items-center py-12")
        div(class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-themePrimary")
      
      // Error state
      div(v-else-if="error" class="bg-red-50 p-4 rounded-lg text-red-600")
        p {{ error }}
        button(@click="fetchThemes" class="mt-2 px-4 py-2 bg-red-100 hover:bg-red-200 rounded-md") Try Again
      
      // Themes grid
      div(v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6")
        div(v-for="theme in computedThemes" :key="theme.id" class="store-card rounded-lg shadow-md overflow-hidden border transition-shadow")
          // Theme preview with actual CSS styling
          div(class="h-40 relative overflow-hidden")
            div(
              class="absolute inset-0 theme-preview-background"
              :style="theme.backgroundCss"
            )
            // Rotary button preview
            div(class="absolute w-10 top-1/2 left-1/4 transform -translate-x-1/2 -translate-y-1/2")
              DeckButtonsRotary(:displayValue="false" :isReactive="true" :theme="theme.components.rotaryButton" behavior="free" class="h-full w-full")
          
            // Click button preview
            div(
              class="absolute top-1/2 right-1/4 transform translate-x-1/2 -translate-y-1/2 w-10 flex items-center justify-center"
            )
              DeckDynamicButton(:theme="theme.components.clickButton") 
            // Theme name overlay
            div(class="absolute bottom-0 left-0 right-0 bg-black/50 text-white p-2 text-center font-medium") 
              p {{ theme.name }}
            
            // Active theme indicator
            div(
              v-if="deckStore.getActiveTheme.id === theme.id" 
              class="absolute top-4 right-0 bg-gradient-to-r from-green-500 to-emerald-600 text-white py-1 px-12 transform rotate-45 translate-x-[30%] translate-y-[-10%] shadow-md text-xs font-bold tracking-wide"
            )
              span.flex.items-center.justify-center
                i.ri-check-line.mr-1
                | Active
          div(class="p-4")
            div(class="flex justify-between items-start mb-3")
              h3(class="font-semibold text-lg text-themeText") {{ theme.name }}
              span(class="store-pill px-2 py-1 rounded-full text-xs font-medium text-themeText3") {{ theme.price > 0 ? `$${theme.price}` : 'Free' }}
            
            p(class="text-sm text-themeText3 mb-4 line-clamp-2") {{ theme.description }}
            
            div(class="flex justify-between items-center")
              div(class="flex items-center")
                span(class="text-yellow-500 mr-1") ★
                span(class="text-sm text-themeText3") {{ theme.rating || '4.5' }}
              
              div(class="flex space-x-2")
                button(
                  v-if="isThemeDownloaded(theme)"
                  @click="deckStore.uninstallTheme(theme.id)"
                  class="px-4 py-1.5 bg-red-500 text-white rounded-md hover:bg-red-600 text-sm font-medium"
                ) Uninstall
                button(
                  v-if="deckStore.getActiveTheme.id !== theme.id"
                  @click="downloadTheme(theme)"
                  class="store-primary-btn px-4 py-1.5 text-white rounded-md text-sm font-medium"
                ) {{ isThemeDownloaded(theme) ? 'Apply' : 'Download' }}
    
    // Other tab content placeholders
    div(v-else class="flex flex-col items-center justify-center h-full p-8")
      div(class="text-center max-w-md")
        h2(class="text-xl font-bold text-themeText mb-4") {{ getTabTitle }}
        p(class="text-themeText2") This section is coming soon.
</template>

<script setup lang="ts">
import { useAuthStore } from "@/stores/Auth";
import { useDesktopUtils } from "@desktop/stores/desktopUtils";
import { useDeckStore } from "@/stores/Deck";
import { useMainStore } from "@/stores/Main";

definePageMeta({
  title: "Marketplace",
  description: "Explore and download extensions for your deck",
});

const config = useRuntimeConfig();
const authStore = useAuthStore();
const desktopUtils = useDesktopUtils();
const deckStore = useDeckStore();
const mainStore = useMainStore();

// Tab navigation
const activeTab = ref("themes");

// Computed property for tab title
const getTabTitle = computed(() => {
  const titles = {
    iconPacks: "Icon Packs",
    buttons: "Buttons",
    decks: "Decks",
    actions: "Actions",
    themes: "Themes",
    plugins: "Plugins",
  };
  return titles[activeTab.value] || "Marketplace";
});

// State variables
const loading = ref(true);
const error = ref("");
const themes = ref([]);
const downloadedThemes = ref([]);

// Watch for changes in the store themes
watch(
  () => deckStore.getThemes,
  (newThemes) => {
    downloadedThemes.value = newThemes || [];
  },
  { immediate: true, deep: true }
);

// Function to check if a theme is downloaded
const isThemeDownloaded = (theme) => {
  return downloadedThemes.value.some((storeTheme) => storeTheme.id === theme.id);
};

// Computed property for themes with download status
const computedThemes = computed(() => {
  return themes.value.map((theme) => ({
    ...theme,
    downloaded: isThemeDownloaded(theme),
  }));
});

// Function to open login modal
const openLoginModal = () => {
  desktopUtils.setUserModal(true);
  mainStore.removeAllNotificationsOfType("logout");
};

// Download theme function
const downloadTheme = async (theme) => {
  if (isThemeDownloaded(theme)) {
    deckStore.setActiveTheme(theme);
  } else {
    try {
      await deckStore.downloadTheme(theme);
      // Force refresh of downloaded themes
      downloadedThemes.value = deckStore.getThemes || [];
    } catch (err) {
      console.error("Error downloading theme:", err);
    }
  }
};

// Fetch themes data from API
const fetchThemes = async () => {
  loading.value = true;
  error.value = "";

  try {
    const result = await useFetchInterceptor(
      `${config.public.API_URL}/api/v1/store/deck-themes/`,
      {
        method: "GET",
        headers: {
          "Content-Type": "application/json",
          "x-client-Type": "native",
          accept: "application/json",
        },
      }
    );

    if (result.ok) {
      themes.value = (await result.json()) || [];
    } else {
      error.value = "Failed to load themes. Please try again.";
    }
  } catch (err) {
    console.error("Error fetching themes:", err);
    error.value = "Failed to load themes. Please try again.";
  } finally {
    loading.value = false;
  }
};

// Call fetchThemes when component is mounted
onMounted(fetchThemes);

const { isOnline } = useNetworkStatus();
watch(isOnline, (online) => {
  if (online && (error.value || !themes.value.length)) {
    fetchThemes();
  }
});
</script>

<style lang="scss" scoped>
.deck-store-page {
  --background: 220 34% 98%;
  --background2: 220 24% 96%;
  --background3: 220 24% 92%;
  --text: 220 26% 20%;
  --text2: 220 14% 42%;
  --text3: 220 12% 57%;
  --primary: 20 96% 55%;
  --secondary: 20 96% 55%;
  --border: 220 22% 86%;
  --border2: 220 20% 82%;
  --hover: 220 26% 93%;
  --active: 20 96% 55%;
  background: hsl(var(--background));
  color: hsl(var(--text));
}

.store-header {
  position: sticky;
  top: 0;
  z-index: 8;
  background: hsl(var(--background));
  border-bottom: 1px solid hsl(var(--border));
  padding-top: 1rem;
}

.store-tab--idle {
  color: hsl(var(--text2));
}

.store-tab--active {
  color: #ff6a3a;
  border-bottom-color: #ff6a3a;
}

.store-empty-state {
  background: linear-gradient(180deg, #fbfcff 0%, #f3f6fb 100%);
  border: 1px solid hsl(var(--border));
  border-radius: 0.9rem;
}

.store-primary-btn {
  background: linear-gradient(90deg, #ff5d23, #ff7b45);
}

.store-card {
  background: #fbfcff;
  border-color: hsl(var(--border));
  box-shadow: 0 8px 20px rgba(33, 46, 76, 0.06);
}

.store-card:hover {
  box-shadow: 0 12px 28px rgba(33, 46, 76, 0.1);
}

.store-pill {
  background: hsl(var(--background));
}

.theme-preview-background {
  transition: all 0.3s ease;
}
</style>
