<template lang="pug">
div(class="w-full h-full flex flex-col")
  div(v-if="!authStore.getAccessToken" class="flex flex-col items-center justify-center h-full p-8 bg-themeBackground rounded-lg shadow-sm")
    div(class="text-center max-w-md")
      h2(class="text-2xl font-bold text-themeText mb-4") {{ t("store.themes.loginRequired") }}
      p(class="text-themeText2 mb-6") {{ t("store.themes.loginRequiredDescription") }}
      button(
        @click="openLoginModal" 
        class="px-6 py-2 bg-violet-600 text-white rounded-md hover:bg-violet-700 transition-colors"
      ) Login
  div(v-else class="w-full h-full p-6 pt-0 overflow-scroll relative")
    // Search and filter bar
    div(class="mb-6 flex flex-col md:flex-row gap-4 items-center sticky py-2 w-full top-0 left-0 right-0 bg-themeBackground z-10")
      slot(name="search-filters")
    
    // Pagination top
    slot(name="pagination-top")
    
    // Loading state
    div(v-if="loading" class="flex justify-center items-center py-12")
      div(class="animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-themePrimary")
    
    // Error state
    div(v-else-if="error" class="bg-red-50 p-4 rounded-lg text-red-600")
      p {{ error }}
      button(@click="refreshData" class="mt-2 px-4 py-2 bg-red-100 hover:bg-red-200 rounded-md") {{ t("store.themes.tryAgain") }}
    
    // Empty state
    div(v-else-if="isEmpty" class="flex flex-col items-center justify-center py-12 text-center")
      slot(name="empty-state")
    
    // Content grid
    div(v-else)
      slot(name="content-grid")
    
    // Pagination bottom
    slot(name="pagination-bottom")
</template>

<script setup lang="ts">
import { useAuthStore } from "@/stores/Auth";
import { useMainStore } from "@/stores/Main";

const { t } = useI18n();
const authStore = useAuthStore();
const mainStore = useMainStore();

const props = defineProps({
  loading: {
    type: Boolean,
    default: false,
  },
  error: {
    type: String,
    default: "",
  },
  isEmpty: {
    type: Boolean,
    default: false,
  },
});

const emit = defineEmits(["refresh"]);

const openLoginModal = () => {
  mainStore.openModal("login");
};

const refreshData = () => {
  emit("refresh");
};
</script>
