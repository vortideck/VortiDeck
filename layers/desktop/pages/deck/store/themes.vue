<template lang="pug">
    div(class="deck-store-page deck-store-themes w-full h-full flex flex-col")
      div(v-if="!authStore.getAccessToken" class="store-empty-state flex flex-col items-center justify-center h-full p-8")
        div(class="text-center max-w-md")
          h2(class="text-2xl font-bold  text-themeText mb-4") {{ t("store.themes.loginRequired") }}
          p(class="text-themeText2 mb-6") {{ t("store.themes.loginRequiredDescription") }}
          button(
            @click="openLoginModal" 
            class="store-primary-btn px-6 py-2 text-white rounded-md transition-colors"
          ) Login
      div(v-else class="w-full h-full flex flex-col")
        // Search and filter bar - sticky header
        div(class="store-header sticky top-0 z-10 p-6 pb-4")
          div(class="flex flex-col md:flex-row gap-4 items-center")
            div(class="relative w-full md:w-1/2")
              div(class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none")
                PixelarticonsSearch(class="w-5 h-5 text-themeText3")
              input(
                v-model="searchQuery"
                @click="searchQuery = ''"
                type="text"
                placeholder="Search themes..."
                class="store-input pl-10 pr-4 py-2 w-full rounded-lg focus:outline-none"
              )
            div(class="flex items-center gap-4 w-full")
              button(
                @click="showFavorites = !showFavorites"
                class="store-soft-btn flex items-center gap-2 px-4 py-2 rounded-lg transition-colors"
                :class="showFavorites ? 'bg-themeError/10 border-themeError/40 text-themeError' : 'hover:bg-themeBackground3'"
              )
                IcBaselineFavorite(class="w-5 h-5" :class="showFavorites ? 'text-themeError' : 'text-themeText3'")
                span(class="text-sm font-medium") {{ showFavorites ? 'Showing Favorites Only' : 'Show Favorites' }}

              button(
                @click="fetchThemes"
                class="store-soft-btn p-2 ml-auto rounded-lg transition-colors"
                title="Refresh themes"
              )
                IcBaselineRefresh(class="w-5 h-5 text-themeText3")

        // Main content area with proper scroll
        div(class="flex-1 overflow-y-auto")
          div(class="p-6 pt-4")
            // Loading state
            UtilsPagination(v-if="totalPages > 1" :pages="totalPages" :page="currentPage"  @onPageChange="changePage" class="mb-8")
            div(v-if="loading" class="flex justify-center items-center py-12")
              div(class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-themePrimary")
            
            // Error state
            div(v-else-if="error" class="bg-themeError/10 border border-themeError/25 p-4 rounded-lg text-themeError")
              p {{ error }}
              button(@click="fetchThemes" class="mt-2 px-4 py-2 bg-themeBackground2 hover:bg-themeBackground3 border border-themeBorder2 text-themeText rounded-md transition-colors") {{ t("store.themes.tryAgain") }}
            
            // Empty state for search results
            div(v-else-if="filteredThemes.length === 0" class="flex flex-col items-center justify-center py-12 text-center")
              IcOutlineSearchOff(class="w-16 h-16 text-themeText3 mb-4")
              h3(class="text-xl font-medium text-themeText mb-2") No themes found
              p(class="text-themeText3") Try adjusting your search or filters
            
            // Themes grid
            div(v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 px-1")
              div(v-for="theme in filteredThemes" :key="theme.id" class="store-card theme-card rounded-lg overflow-hidden transition-shadow")
                // Theme preview with actual CSS styling
                div(class="h-40 relative overflow-hidden")
                  div(
                    class="absolute inset-0 theme-preview-background"
                    :style="theme.backgroundCss"
                  )
                  // Rotary button preview
                  div(class="absolute w-10 top-1/2 left-1/4 transform -translate-x-1/2 -translate-y-1/2")
                    DeckButtonsRotary(:displayValue="false" :isReactive="true"  :theme="theme.components.rotaryButton" behavior="free" class="h-full w-full")

                  // Click button preview
                  div(
                    class="absolute top-1/2 right-1/4 transform translate-x-1/2 -translate-y-1/2 w-10 flex items-center justify-center"
                    
                  )
                    DeckDynamicButton(:theme="theme.components.clickButton" ) 
                  // Theme name overlay
                  div(class="absolute bottom-0 left-0 right-0 bg-black/50 text-white p-2 text-center font-medium") 
                    p {{ theme.name }}
                  
                  // Favorite button
                  button(
                    @click="toggleFavorite(theme)"
                    class="absolute top-2 left-2 p-1.5 bg-black/40 hover:bg-black/60 rounded-full transition-colors"
                  )
                    IcBaselineFavorite(
                      class="w-5 h-5"
                      :class="isFavorite(theme) ? 'text-themeError' : 'text-white/70'"
                    )
                  
                  // Active theme indicator
                  div(
                    v-if="deckStore.getActiveTheme.id === theme.id" 
                    class="absolute top-4 right-0 bg-themeSuccess text-white py-1 px-12 transform rotate-45 translate-x-[30%] translate-y-[-10%] shadow-md text-xs font-bold tracking-wide"
                  )
                    span.flex.items-center.justify-center
                      i.ri-check-line.mr-1
                      | {{ t("store.themes.active") }}
                div(class="p-4")
                  div(class="flex justify-between items-start mb-3")
                    h3(class="font-semibold text-lg text-themeText") {{ theme.name }}
                    span(class="store-pill px-2 py-1 rounded-full text-xs font-medium text-themeText3") {{ theme.price > 0 ? `$${theme.price}` : t("store.themes.free") }}
                  
                  p(class="text-sm text-themeText3 mb-4 line-clamp-2") {{ theme.description }}
                  
                  div(class="flex justify-between items-center")
                    div(class="flex items-center")
                      span(class="text-yellow-500 mr-1") ★
                      span(class="text-sm text-themeText3") {{ theme.rating || '4.5' }}
                    
                    div(class="flex space-x-2")
                      button(
                        v-if="isThemeDownloaded(theme)"
                        @click="deckStore.uninstallTheme(theme.id)"
                        class="px-4 py-1.5 bg-themeError text-white rounded-md hover:brightness-90 text-sm font-medium transition-all"
                      ) {{ t("store.themes.uninstall") }}
                      button(
                        v-if="deckStore.getActiveTheme.id !== theme.id "
                        @click="downloadTheme(theme)"
                        class="store-primary-btn px-4 py-1.5 text-white rounded-md text-sm font-medium"
                      ) {{ isThemeDownloaded(theme) ? t("store.themes.apply") : t("store.themes.download") }}

            UtilsPagination(v-if="totalPages > 0" :pages="totalPages" :page="currentPage"  @onPageChange="changePage" class="py-8")
        
</template>

<script setup lang="ts">
import IcBaselineFavorite from "~icons/ic/baseline-favorite?width=24px&height=24px";
import IcBaselineRefresh from "~icons/ic/baseline-refresh?width=24px&height=24px";
import PixelarticonsSearch from "~icons/pixelarticons/search?width=24px&height=24px";
import IcOutlineSearchOff from "~icons/ic/outline-search-off?width=24px&height=24px";

import { useAuthStore } from "@/stores/Auth";
import { useDesktopUtils } from "@desktop/stores/desktopUtils";
import { useDeckStore } from "@/stores/Deck";
import { useMainStore } from "@/stores/Main";
import type { DeckTheme } from "@base/stores/Deck";

import { ref, onMounted, computed, watch } from "vue";

interface MarketplaceTheme extends DeckTheme {
  description?: string;
  price?: number;
  rating?: number | string;
}

interface ThemesResponse {
  results: MarketplaceTheme[];
  count: number;
}

const { t } = useI18n();

const title = ref(t("store.themes.title"));
const description = ref(t("store.themes.description"));

const route = useRoute();
definePageMeta({
  title: "",
  description: "",
});

route.meta.title = title.value;
route.meta.description = description.value;

const currentPage = ref(1);
const totalPages = ref(1);
const perPage = ref(12);

const changePage = (page: number) => {
  currentPage.value = page;
  fetchThemes();
};

// Lazy load the ThemeCard component
// const ThemeCard = defineAsyncComponent(() => import('@/components/deck/ThemeCard.vue'));

const config = useRuntimeConfig();
const authStore = useAuthStore();
const desktopUtils = useDesktopUtils();
const deckStore = useDeckStore();
const mainStore = useMainStore();

// State variables
const loading = ref(true);
const error = ref("");
const themes = ref<MarketplaceTheme[]>([]);
const downloadedThemes = ref<DeckTheme[]>([]);
const searchQuery = ref("");
const showFavorites = ref(false);
const favoriteThemes = ref<number[]>([]);

// Load favorites from localStorage on mount
onMounted(() => {
  const savedFavorites = localStorage.getItem("favoriteThemes");
  if (savedFavorites) {
    try {
      const parsed: unknown = JSON.parse(savedFavorites);
      favoriteThemes.value = Array.isArray(parsed)
        ? parsed.filter((id): id is number => typeof id === "number" && Number.isInteger(id))
        : [];
    } catch (e) {
      console.error("Error loading favorite themes:", e);
      favoriteThemes.value = [];
    }
  }
});

// Save favorites to localStorage when changed
watch(
  favoriteThemes,
  (newValue) => {
    localStorage.setItem("favoriteThemes", JSON.stringify(newValue));
  },
  { deep: true }
);

// Watch for changes in the store themes
watch(
  () => deckStore.getThemes,
  (newThemes) => {
    downloadedThemes.value = newThemes || [];
  },
  { immediate: true, deep: true }
);

// Function to check if a theme is downloaded
const isThemeDownloaded = (theme: MarketplaceTheme) => {
  return downloadedThemes.value.some((storeTheme) => storeTheme.id === theme.id);
};

// Function to check if a theme is favorited
const isFavorite = (theme: MarketplaceTheme) => {
  return favoriteThemes.value.includes(theme.id);
};

// Function to toggle favorite status
const toggleFavorite = (theme: MarketplaceTheme) => {
  if (isFavorite(theme)) {
    favoriteThemes.value = favoriteThemes.value.filter((id) => id !== theme.id);
  } else {
    favoriteThemes.value.push(theme.id);
  }
};

// Computed property for filtered themes
const filteredThemes = computed(() => {
  let result = themes.value;

  // Filter by search query
  if (searchQuery.value.trim()) {
    const query = searchQuery.value.toLowerCase();
    result = result.filter(
      (theme) =>
        theme.name.toLowerCase().includes(query) ||
        (theme.description && theme.description.toLowerCase().includes(query))
    );
  }

  // Filter by favorites
  if (showFavorites.value) {
    result = result.filter((theme) => isFavorite(theme));
  }

  return result.map((theme) => ({
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
const downloadTheme = async (theme: MarketplaceTheme) => {
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
      `${config.public.API_URL}/api/v1/store/deck-themes/?page=${currentPage.value}&page_size=${perPage.value}`,
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
      const data = await result.json() as ThemesResponse;
      themes.value = data.results || [];
      totalPages.value = Math.ceil(data.count / perPage.value);
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
</script>

<style lang="scss" scoped>
.deck-store-page {
  background: hsl(var(--background));
  color: hsl(var(--text));
}

.store-header {
  background: hsl(var(--background) / 0.96);
  border-bottom: 1px solid hsl(var(--border) / 0.65);
  backdrop-filter: blur(10px);
}

.store-empty-state {
  background: hsl(var(--background2));
  border: 1px solid hsl(var(--border));
  border-radius: 0.9rem;
}

.store-input {
  background: hsl(var(--background2));
  border: 1px solid hsl(var(--border));
  color: hsl(var(--text));
}

.store-input:focus {
  border-color: hsl(var(--button));
  box-shadow: 0 0 0 2px hsl(var(--button) / 0.18);
}

.store-soft-btn {
  border: 1px solid hsl(var(--border));
  background: hsl(var(--background2));
  color: hsl(var(--text2));
}

.store-primary-btn {
  background: hsl(var(--button));
}

.store-primary-btn:hover {
  filter: brightness(1.05);
}

.store-card {
  background: hsl(var(--background2));
  border: 1px solid hsl(var(--border));
  box-shadow: 0 8px 20px hsl(var(--text) / 0.05);
}

.store-card:hover {
  border-color: hsl(var(--button) / 0.4);
  box-shadow: 0 12px 28px hsl(var(--text) / 0.09);
}

.store-pill {
  background: hsl(var(--background3));
  border: 1px solid hsl(var(--border));
}

.theme-preview-background {
  transition: all 0.3s ease;
}
</style>
