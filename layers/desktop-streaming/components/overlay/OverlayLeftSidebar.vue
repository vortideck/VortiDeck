<template lang="pug">
div(class="relative flex-shrink-0 h-full" style="width: 72px")
  //- Icon Rail (always visible with labels)
  div(class="w-[72px] h-full flex flex-col items-center py-3 gap-2 bg-themeBackground border-r border-themeBorder relative z-20 shadow-sm")
    //- Tab icons from libraryTabs
    button(
      v-for="tab in libraryTabs"
      :key="tab.id"
      @click="handleIconClick(tab.id)"
      :class="activeLibraryTab === tab.id && isExpanded ? 'bg-blue-500/15 text-blue-500 font-bold' : 'text-themeText2 hover:text-themeText hover:bg-themeBackground2'"
      class="w-14 py-2 px-1 rounded-xl flex flex-col items-center justify-center gap-1 transition-colors relative text-center group cursor-pointer"
      :title="tab.label"
    )
      Icon(:icon="tab.icon" class="w-5 h-5")
      span(class="text-[10px] leading-tight select-none font-medium truncate max-w-full") {{ tab.label }}
      span(v-if="tab.count" class="absolute top-1 right-1 min-w-[16px] h-4 bg-blue-500 text-white text-[10px] rounded-full flex items-center justify-center px-1 font-bold") {{ tab.count }}

    //- Separator
    div(class="w-10 h-px bg-themeBorder my-1")

    //- Action buttons (open installed component library)
    button(
      @click="$emit('show-marketplace')"
      class="w-14 py-2 px-1 rounded-xl flex flex-col items-center justify-center gap-1 text-themeText2 hover:text-themeText hover:bg-themeBackground2 transition-colors text-center cursor-pointer"
      title="Overlay component library"
    )
      Icon(icon="heroicons:shopping-bag" class="w-5 h-5")
      span(class="text-[10px] leading-tight font-medium select-none") Library

    //- Spacer
    div(class="flex-1")

  //- Floating Panel (overlays canvas when expanded)
  Transition(name="slide-panel")
    div(
      v-if="isExpanded"
      ref="leftPanel"
      class="absolute top-0 left-[72px] h-full bg-themeBackground2/95 backdrop-blur-xl border-r border-themeBorder shadow-xl z-10 flex flex-col"
      :style="{ width: expandedWidth + 'px' }"
    )
      //- Panel header with close button
      div(class="flex items-center justify-between px-3 py-2.5 border-b border-themeBorder")
        h3(class="text-sm font-semibold text-themeText") {{ activeTabLabel }}
        button(@click="isExpanded = false" class="p-1 text-themeText2 hover:text-themeText hover:bg-themeBackground3 rounded-md transition-colors cursor-pointer")
          svg(class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")

      //- Search
      div(class="p-3 border-b border-themeBorder")
        div(class="relative")
          input(
            v-model="searchQuery"
            type="text"
            placeholder="Search elements..."
            class="w-full pl-8 pr-3 py-2 bg-themeBackground border border-themeBorder rounded-lg text-sm text-themeText placeholder-themeText3 focus:outline-none focus:ring-1 focus:ring-themePrimary focus:border-themePrimary"
          )
          PixelarticonsSearch(class="absolute left-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-themeText3 pointer-events-none")

      //- Tab Content (scrollable)
      div(class="flex-1 overflow-y-auto")
        //- Theme Selector Tab (Compact 2-Column Grid with 1-Click Apply)
        div(v-if="activeLibraryTab === 'theme'" class="p-3 space-y-3")
          //- Active Theme Compact Card
          div(v-if="activeOverlayTheme" class="p-2.5 bg-themeBackground text-themeText rounded-xl shadow-md border border-themeBorder flex flex-col gap-2")
            div(class="flex items-center justify-between")
              div(class="flex items-center gap-2 min-w-0")
                span(class="w-2 h-2 rounded-full bg-emerald-400 animate-pulse flex-shrink-0")
                span(class="text-xs font-bold text-themeText truncate") {{ activeOverlayTheme.name }}
              span(class="text-[10px] text-themePrimary font-bold uppercase tracking-wider flex-shrink-0") Active Theme
            div(class="flex h-2.5 rounded-md overflow-hidden border border-themeBorder shadow-inner")
              div(v-for="(c, i) in activeOverlayTheme.colors" :key="i" :style="`background-color: ${c}`" class="flex-1" :title="c")

          //- Category Filter Pills
          div
            div(class="flex items-center justify-between mb-2")
              h4(class="text-xs font-bold text-themeText2 uppercase tracking-wider") Theme Catalog
              span(class="text-[10px] text-themeText3 font-medium") {{ filteredThemeCatalog.length }} themes
            div(class="flex flex-wrap gap-1")
              button(
                v-for="cat in themeFilterCategories"
                :key="cat.id"
                @click="selectedThemeCategory = cat.id"
                :class="selectedThemeCategory === cat.id ? 'bg-themePrimary text-white font-bold' : 'bg-themeBackground border border-themeBorder text-themeText2 hover:text-themeText hover:bg-themeBackground3'"
                class="px-2.5 py-1 rounded-full text-xs font-medium transition-colors cursor-pointer"
              ) {{ cat.name }}

          //- Available Themes 2-Column Grid (Ultra-clean & 1-Click Apply)
          div(class="grid grid-cols-2 gap-2")
            div(
              v-for="theme in filteredThemeCatalog"
              :key="theme.id"
              @click="applyThemeDirect(theme)"
              @mouseenter="setHoveredTheme(theme, $event)"
              @mouseleave="clearHoveredTheme"
              :class="activeOverlayTheme && activeOverlayTheme.id === theme.id ? 'border-themePrimary ring-2 ring-themePrimary/30 bg-themePrimary/15 shadow-sm' : 'border-themeBorder bg-themeBackground hover:bg-themeBackground3 hover:border-themePrimary/40'"
              class="p-2.5 rounded-xl border transition-all flex items-center justify-between gap-1.5 cursor-pointer group relative"
            )
              div(class="flex items-center gap-2 min-w-0")
                div(:style="`background: linear-gradient(135deg, ${theme.colors[0]}, ${theme.colors[2] || theme.colors[1]})`" class="w-3.5 h-3.5 rounded-full border border-themeBorder shadow-xs flex-shrink-0")
                h5(class="text-xs font-bold text-themeText truncate group-hover:text-themePrimary transition-colors") {{ theme.name }}
              span(v-if="activeOverlayTheme && activeOverlayTheme.id === theme.id" class="w-2 h-2 rounded-full bg-themePrimary flex-shrink-0" title="Active Theme")
              span(v-else class="text-[10px] text-themeText3 group-hover:text-themePrimary font-medium transition-colors opacity-0 group-hover:opacity-100 flex-shrink-0") →

        //- Add to Canvas Tab (Unified Library)
        div(v-if="activeLibraryTab === 'elements'" class="p-3")
          //- Collapsible categories
          div(v-if="filteredCategories.length === 0" class="text-center py-8 text-themeText3 text-xs")
            p No matching elements or widgets found
          div(v-for="category in filteredCategories" :key="category.id" class="mb-3")
            button(
              @click="toggleBuiltinCategory(category.id, category.expanded)"
              class="w-full flex items-center justify-between p-2 hover:bg-themeBackground3 rounded-lg transition-colors font-medium text-themeText cursor-pointer"
            )
              div(class="flex items-center gap-2")
                Icon(:icon="category.icon" class="w-4 h-4 text-themePrimary")
                span(class="text-xs font-semibold text-themeText2 uppercase tracking-wider") {{ category.name }}
                span(class="text-[11px] text-themeText3 font-normal") ({{ category.items.length }})
              svg(
                :class="isCategoryExpanded(category.id, category.expanded) ? 'rotate-90' : ''"
                class="w-4 h-4 text-themeText3 transition-transform duration-200"
                fill="none" stroke="currentColor" viewBox="0 0 24 24"
              )
                path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7")

            Transition(name="expand")
              div(v-if="isCategoryExpanded(category.id, category.expanded)" class="mt-1 space-y-1")
                button(
                  v-for="item in category.items"
                  :key="item.id"
                  draggable="true"
                  @dragstart="handleItemDragStart($event, item)"
                  @dragend="handleItemDragEnd"
                  @click="$emit('add-element', item)"
                  @mouseenter="setHoveredItem(item, category.name, $event)"
                  @mouseleave="clearHoveredItem"
                  class="w-full p-2 bg-themeBackground hover:bg-themeBackground3 border border-themeBorder hover:border-themePrimary/40 rounded-xl transition-all flex items-center gap-2 group relative text-left cursor-grab active:cursor-grabbing shadow-2xs"
                )
                  Icon(:icon="item.icon" class="w-4 h-4 text-themeText2 group-hover:text-themePrimary transition-colors")
                  div(class="flex-1 min-w-0")
                    p(class="text-xs font-semibold text-themeText truncate") {{ item.name }}
                    p(class="text-[11px] text-themeText2 truncate") {{ item.description }}

        //- Favorites Tab
        div(v-if="activeLibraryTab === 'favorites'" class="p-3")
          div(v-if="favoriteElements.length === 0" class="text-center py-8")
            Icon(icon="heroicons:star" class="w-10 h-10 text-themeText3")
            p(class="text-sm text-themeText2 mt-2") No favorites yet
            p(class="text-xs text-themeText3") Right-click elements to add to favorites
          div(v-else class="space-y-2")
            div(
              v-for="fav in favoriteElements"
              :key="fav.id"
              draggable="true"
              @dragstart="handleItemDragStart($event, fav)"
              @dragend="handleItemDragEnd"
              class="w-full p-3 bg-themeBackground hover:bg-themeBackground3 border border-themeBorder hover:border-themePrimary/40 rounded-xl transition-all flex items-center gap-3 group cursor-grab active:cursor-grabbing shadow-2xs"
            )
              Icon(:icon="fav.icon" class="w-5 h-5 text-themePrimary")
              div(class="flex-1 text-left")
                p(class="text-sm font-semibold text-themeText group-hover:text-themePrimary transition-colors") {{ fav.name }}
                p(class="text-xs text-themeText2") {{ fav.usageCount || 0 }} times used
              button(@click.stop="removeFavorite(fav.id)" class="text-themeText3 hover:text-red-500 transition-colors")
                svg(class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24")
                  path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")

      //- Resize handle (on floating panel)
      div(
        ref="resizeHandle"
        @mousedown="startPanelResize"
        class="absolute top-0 right-0 w-2 h-full hover:bg-themePrimary/20 cursor-col-resize z-20 panel-resize-handle group transition-colors duration-150"
        title="Drag to resize panel"
      )
        div(class="absolute top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 opacity-0 group-hover:opacity-100 transition-opacity duration-200")
          svg(class="w-4 h-4 text-themePrimary" fill="none" stroke="currentColor" viewBox="0 0 24 24")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 9l4-4 4 4m0 6l-4 4-4-4")

  //- Teleported Hover Preview Card
  Teleport(to="body")
    Transition(name="fade")
      div(
        v-if="hoveredItem"
        :style="{ top: hoverPos.top + 'px', left: hoverPos.left + 'px' }"
        class="fixed p-3.5 bg-themeBackground2/95 backdrop-blur-md text-themeText rounded-xl shadow-2xl border border-themeBorder w-72 flex flex-col gap-1.5 z-[9999] pointer-events-none transition-all duration-150"
      )
        div(class="flex items-center justify-between")
          span(class="text-[10px] font-bold uppercase tracking-wider text-themePrimary") {{ hoverPos.categoryName }}
          span(class="text-[10px] text-themeText3 font-mono") {{ hoveredItem.width || 320 }}×{{ hoveredItem.height || 80 }}px
        p(class="text-xs font-bold text-themeText") {{ hoveredItem.name }}
        p(class="text-[11px] text-themeText2 leading-tight") {{ hoveredItem.preview || hoveredItem.description }}

        //- Real UniversalElement Component Engine Render with Aspect-Ratio Auto-Scaling
        div(class="my-1.5 w-64 h-36 bg-themeBackground rounded-xl border border-themeBorder overflow-hidden relative flex items-center justify-center shadow-2xl p-1 self-center")
          div(
            v-if="previewElementObject"
            :style="{ width: (previewElementObject.width || 320) + 'px', height: (previewElementObject.height || 120) + 'px', transform: `scale(${previewScale})` }"
            class="origin-center flex items-center justify-center pointer-events-none"
          )
            UniversalElement(:element="previewElementObject")

        div(class="mt-1 pt-1.5 border-t border-themeBorder flex items-center justify-between text-[10px] text-themePrimary font-semibold")
          span Click to place on overlay
          Icon(icon="heroicons:plus" class="w-3.5 h-3.5")

  //- Teleported Theme Hover Preview Card
  Teleport(to="body")
    Transition(name="fade")
      div(
        v-if="hoveredTheme"
        :style="{ top: themeHoverPos.top + 'px', left: themeHoverPos.left + 'px' }"
        class="fixed p-3 bg-themeBackground2/95 backdrop-blur-md text-themeText rounded-xl shadow-2xl border border-themeBorder w-64 flex flex-col gap-2 z-[9999] pointer-events-none transition-all duration-150"
      )
        div(class="flex items-center justify-between")
          span(class="text-[10px] font-bold uppercase tracking-wider text-themePrimary") Theme Impact Preview
          span(class="text-[10px] text-themeText3 font-mono") {{ hoveredTheme.category || 'Preset' }}
        p(class="text-xs font-bold text-themeText") {{ hoveredTheme.name }}

        //- Modular Vue Component
        ThemeImpactPreview(:theme="hoveredTheme")

        div(class="pt-1 border-t border-themeBorder flex items-center justify-between text-[10px] text-themePrimary font-semibold")
          span Click to apply theme
          Icon(icon="heroicons:swatch" class="w-3.5 h-3.5")
</template>

<script setup lang="ts">
import { ref, computed, reactive } from "vue";
import { Icon } from "@iconify/vue";
import PixelarticonsSearch from "~icons/pixelarticons/search?width=16px&height=16px";
import type { ThemeDef } from "../../composables/overlay/useOverlayTheme";
import { availableThemes } from "../../composables/overlay/themeRegistry";
import { getComponentSchema, getElementDefaults, activeDraggingElement } from "../../composables/overlay/componentSchema";
import UniversalElement from "./UniversalElement.vue";
import ThemeImpactPreview from "./ThemeImpactPreview.vue";

// Hover card preview state
const hoveredItem = ref<Element | null>(null);
const hoverPos = ref({ top: 0, left: 0, categoryName: '' });

// Theme hover preview state
const hoveredTheme = ref<ThemeDef | null>(null);
const themeHoverPos = ref({ top: 0, left: 0 });

const setHoveredTheme = (theme: ThemeDef, event: MouseEvent) => {
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
  const cardEstimatedHeight = 220;
  const windowHeight = window.innerHeight;

  let computedTop = rect.top;
  if (computedTop + cardEstimatedHeight > windowHeight - 16) {
    computedTop = Math.max(16, windowHeight - cardEstimatedHeight - 16);
  }

  hoveredTheme.value = theme;
  themeHoverPos.value = {
    top: computedTop,
    left: rect.right + 12,
  };
};

const clearHoveredTheme = () => {
  hoveredTheme.value = null;
};

const previewElementObject = computed(() => {
  if (!hoveredItem.value) return null;
  const item = hoveredItem.value;
  const schema = getComponentSchema(item.schemaId || item.id);
  const templateHtml = schema?.template || item.customHTML || item.template;

  const element = {
    ...item,
    id: `preview-${item.id}`,
    schemaId: item.schemaId || item.id,
    name: item.name,
    type: item.type || schema?.type || (item.id === "chat" || item.id === "goal" || item.id.includes("counter") || item.id.includes("goal") ? "widget" : "text"),
    x: 0,
    y: 0,
    width: item.width || schema?.width || 320,
    height: item.height || schema?.height || 120,
    visible: true,
    locked: false,
    content: item.name,
    icon: item.icon,
    customHTML: templateHtml,
    template: templateHtml,
  };

  return element;
});

const previewScale = computed(() => {
  if (!previewElementObject.value) return 0.5;
  const w = previewElementObject.value.width || 320;
  const h = previewElementObject.value.height || 120;
  const scaleX = 236 / w;
  const scaleY = 132 / h;
  return Math.min(scaleX, scaleY, 1);
});

const setHoveredItem = (item: Element, categoryName: string, event: MouseEvent) => {
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
  const cardEstimatedHeight = 280;
  const windowHeight = window.innerHeight;

  let computedTop = rect.top;
  if (computedTop + cardEstimatedHeight > windowHeight - 16) {
    computedTop = Math.max(16, windowHeight - cardEstimatedHeight - 16);
  }

  hoveredItem.value = item;
  hoverPos.value = {
    top: computedTop,
    left: rect.right + 12,
    categoryName,
  };
};

const clearHoveredItem = () => {
  hoveredItem.value = null;
};

const handleItemDragStart = (event: DragEvent, item: Element) => {
  clearHoveredItem();
  activeDraggingElement.value = item;
  if (event.dataTransfer) {
    const payload = JSON.stringify(item);
    event.dataTransfer.setData('application/json', payload);
    event.dataTransfer.setData('vortideck/overlay-element', payload);
    event.dataTransfer.setData('text/plain', item.id);
    event.dataTransfer.effectAllowed = 'copy';
  }
};

const handleItemDragEnd = () => {
  activeDraggingElement.value = null;
};

const selectedThemeCategory = ref("all");
const themeFilterCategories = [
  { id: "all", name: "All" },
  { id: "gaming", name: "Gaming" },
  { id: "professional", name: "Pro" },
  { id: "creative", name: "Creative" },
  { id: "minimal", name: "Minimal" },
];

const filteredThemeCatalog = computed(() => {
  if (selectedThemeCategory.value === "all") {
    return availableThemes;
  }
  return availableThemes.filter((t) => t.category === selectedThemeCategory.value);
});

const applyThemeDirect = (themeInput: string | (typeof availableThemes)[number]) => {
  const targetTheme = typeof themeInput === "string"
    ? availableThemes.find((t) => t.id === themeInput)
    : themeInput;

  if (targetTheme) {
    emit("apply-theme", targetTheme);
  }
};

interface Element {
  id: string;
  name: string;
  description: string;
  icon: string;
  type?: string;
  schemaId?: string;
  width?: number;
  height?: number;
  customHTML?: string;
  template?: string;
  usageCount?: number;
  preview?: string;
  tier?: "free" | "premium" | "pro";
  [key: string]: unknown;
}

interface Category {
  id: string;
  name: string;
  icon: string;
  expanded: boolean;
  items: Element[];
}

interface Tab {
  id: string;
  label: string;
  icon: string;
  count?: number;
}

const props = withDefaults(
  defineProps<{
    leftPanelWidth?: number;
    isResizing?: boolean;
    activeLibraryTab?: string;
    searchQuery?: string;
    currentTheme?: ThemeDef | null;
    libraryTabs?: Tab[];
    builtinCategories?: Category[];
    streamWidgets?: Element[];
    streamAlerts?: Element[];
    favoriteElements?: Element[];
  }>(),
  {
    leftPanelWidth: 256,
    isResizing: false,
    activeLibraryTab: "theme",
    searchQuery: "",
    currentTheme: null,
    libraryTabs: () => [],
    builtinCategories: () => [],
    streamWidgets: () => [],
    streamAlerts: () => [],
    favoriteElements: () => [],
  }
);

// The editor page owns theme state and persistence. The sidebar is a pure
// selector, so its active marker cannot drift from the theme applied to the
// canvas and OBS output.
const activeOverlayTheme = computed(() => props.currentTheme);

const emit = defineEmits<{
  "update:leftPanelWidth": [width: number];
  "update:activeLibraryTab": [tab: string];
  "update:searchQuery": [query: string];
  "show-themes": [];
  "show-marketplace": [];
  "show-theme-settings": [];
  "show-component-creator": [];
  "apply-theme": [theme: (typeof availableThemes)[number]];
  "add-element": [element: Element];
  "remove-favorite": [id: string];
  "resize-start": [];
  "resize-end": [];
}>();

// Dev-only gating (same pattern as Sidebar.vue / debug.vue). The Component
// Creator is a stub prototype parked until the widget contract lands, so it
// renders ONLY in development builds — hidden entirely in production.
const isDev = process.env.NODE_ENV === "development";

const leftPanel = ref<HTMLElement | null>(null);
const resizeHandle = ref<HTMLElement | null>(null);

// Icon rail state
const isExpanded = ref(false);
const expandedWidth = ref(280);

const activeLibraryTab = computed({
  get: () => props.activeLibraryTab,
  set: (value) => emit("update:activeLibraryTab", value),
});

const searchQuery = computed({
  get: () => props.searchQuery,
  set: (value) => emit("update:searchQuery", value),
});

const filteredCategories = computed(() => {
  if (!searchQuery.value || !searchQuery.value.trim()) {
    return props.builtinCategories;
  }
  const query = searchQuery.value.toLowerCase().trim();
  return props.builtinCategories
    .map((category) => {
      const matchingItems = category.items.filter(
        (item) =>
          item.name.toLowerCase().includes(query) ||
          item.description.toLowerCase().includes(query)
      );
      return {
        ...category,
        expanded: matchingItems.length > 0,
        items: matchingItems,
      };
    })
    .filter((category) => category.items.length > 0);
});

const activeTabLabel = computed(() => {
  const tab = props.libraryTabs.find((t) => t.id === activeLibraryTab.value);
  return tab?.label || "Panel";
});

const handleIconClick = (tabId: string) => {
  if (activeLibraryTab.value === tabId && isExpanded.value) {
    isExpanded.value = false;
  } else {
    activeLibraryTab.value = tabId;
    isExpanded.value = true;
  }
};

const categoryExpandedState = reactive<Record<string, boolean>>({});

const isCategoryExpanded = (categoryId: string, defaultExpanded: boolean = false) => {
  // If user typed a search query, expand matching categories automatically
  if (searchQuery.value && searchQuery.value.trim()) {
    return true;
  }
  if (categoryExpandedState[categoryId] !== undefined) {
    return categoryExpandedState[categoryId];
  }
  return defaultExpanded;
};

const toggleBuiltinCategory = (categoryId: string, defaultExpanded: boolean = false) => {
  const current = isCategoryExpanded(categoryId, defaultExpanded);

  // Auto-close all other categories for an ultra-clean single-accordion UX
  props.builtinCategories.forEach((cat) => {
    categoryExpandedState[cat.id] = false;
  });

  categoryExpandedState[categoryId] = !current;
};

const removeFavorite = (id: string) => {
  emit("remove-favorite", id);
};

const startPanelResize = (event: MouseEvent) => {
  event.preventDefault();
  emit("resize-start");

  const startX = event.clientX;
  const startWidth = expandedWidth.value;

  const handleMouseMove = (e: MouseEvent) => {
    const newWidth = Math.max(220, Math.min(500, startWidth + (e.clientX - startX)));
    expandedWidth.value = newWidth;
  };

  const handleMouseUp = () => {
    emit("resize-end");
    document.removeEventListener("mousemove", handleMouseMove);
    document.removeEventListener("mouseup", handleMouseUp);
  };

  document.addEventListener("mousemove", handleMouseMove);
  document.addEventListener("mouseup", handleMouseUp);
};

// Tier-based element categorization functions
const getFreeElementCount = () => {
  return props.builtinCategories.reduce((total, category) => {
    return (
      total + category.items.filter((item) => !item.tier || item.tier === "free").length
    );
  }, 0);
};

const getFreeCategories = () => {
  return props.builtinCategories
    .map((category) => ({
      ...category,
      items: category.items.filter((item) => !item.tier || item.tier === "free"),
    }))
    .filter((category) => category.items.length > 0);
};

const getPremiumCategories = () => {
  return [
    {
      id: "progress-bars",
      name: "Progress Bars",
      icon: "heroicons:chart-bar",
      expanded: false,
      items: [
        { id: "follower-goal", name: "Follower Goal", icon: "heroicons:flag", description: "Animated progress bar for follower goals", preview: "Live follower count with animations", tier: "premium" },
        { id: "subscriber-goal", name: "Subscriber Goal", icon: "heroicons:star", description: "Subscriber goal tracker", preview: "Sub count with tier indicators", tier: "premium" },
        { id: "donation-goal", name: "Donation Goal", icon: "heroicons:currency-dollar", description: "Donation goal progress", preview: "Money raised with animations", tier: "premium" },
      ],
    },
    {
      id: "animated-counters",
      name: "Animated Counters",
      icon: "heroicons:hashtag",
      expanded: false,
      items: [
        { id: "live-follower-count", name: "Live Follower Count", icon: "heroicons:user-group", description: "Real-time follower counter", preview: "Animated number with effects", tier: "premium" },
        { id: "stream-duration", name: "Stream Duration", icon: "heroicons:clock", description: "Live stream timer", preview: "Hours:Minutes:Seconds display", tier: "premium" },
        { id: "viewer-count", name: "Viewer Count", icon: "heroicons:eye", description: "Live viewer counter", preview: "Real-time viewer updates", tier: "premium" },
      ],
    },
    {
      id: "notification-banners",
      name: "Notification Banners",
      icon: "heroicons:bell-alert",
      expanded: false,
      items: [
        { id: "follow-alert", name: "Follow Alert", icon: "heroicons:heart", description: "Animated follow notification", preview: "Sliding banner with username", tier: "premium" },
        { id: "sub-alert", name: "Sub Alert", icon: "heroicons:star", description: "Subscription notification", preview: "Tier-based sub animation", tier: "premium" },
        { id: "donation-alert", name: "Donation Alert", icon: "heroicons:currency-dollar", description: "Donation notification", preview: "Amount and message display", tier: "premium" },
      ],
    },
  ];
};

const getProCategories = () => {
  return [
    {
      id: "custom-html",
      name: "Custom HTML",
      icon: "heroicons:code-bracket",
      expanded: false,
      items: [
        { id: "html-editor", name: "HTML Editor", icon: "heroicons:document-text", description: "Full HTML/CSS/JS editor", preview: "Monaco editor with sandbox", tier: "pro" },
        { id: "widget-builder", name: "Widget Builder", icon: "heroicons:wrench-screwdriver", description: "Visual element creator", preview: "Drag-and-drop interface", tier: "pro" },
      ],
    },
    {
      id: "api-elements",
      name: "API Elements",
      icon: "heroicons:signal",
      expanded: false,
      items: [
        { id: "webhook-element", name: "Webhook Element", icon: "heroicons:globe-alt", description: "Connect to external APIs", preview: "REST API integration", tier: "pro" },
        { id: "websocket-element", name: "WebSocket Element", icon: "heroicons:bolt", description: "Real-time data streaming", preview: "Live data updates", tier: "pro" },
      ],
    },
    {
      id: "3d-elements",
      name: "3D Elements",
      icon: "heroicons:cube-transparent",
      expanded: false,
      items: [
        { id: "3d-model", name: "3D Model", icon: "heroicons:cube", description: "WebGL 3D models", preview: "Three.js integration", tier: "pro" },
        { id: "3d-effects", name: "3D Effects", icon: "heroicons:sparkles", description: "3D visual effects", preview: "Particle systems", tier: "pro" },
      ],
    },
  ];
};
</script>

<style scoped>
.slide-panel-enter-active,
.slide-panel-leave-active {
  transition: all 0.2s ease;
}

.slide-panel-enter-from,
.slide-panel-leave-to {
  opacity: 0;
  transform: translateX(-8px);
}

.expand-enter-active,
.expand-leave-active {
  transition: all 0.3s ease;
  max-height: 1000px;
  overflow: hidden;
}

.expand-enter-from,
.expand-leave-to {
  max-height: 0;
  opacity: 0;
}
</style>
