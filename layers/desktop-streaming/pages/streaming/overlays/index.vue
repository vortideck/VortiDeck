<template lang="pug">
  div(class="h-full w-full flex-1 overflow-hidden flex flex-col bg-themeBackground text-themeText" @contextmenu.prevent)
    OverlayHeader(
      :current-project="currentProject"
      :has-unsaved-changes="hasUnsavedChanges"
      :last-saved="lastSaved"
      :auto-save-enabled="autoSaveEnabled"
      :show-preview-elements="showPreviewElements"
      :show-grid="showGrid"
      :snap-to-grid="snapToGrid"
      :can-undo="canUndo"
      :can-redo="canRedo"
      :server-status="{ running: overlayManager.overlayState.value.serverRunning, url: overlayManager.overlayState.value.serverUrl }"
      :obs-status="{ connected: overlayManager.overlayState.value.obsConnected, streaming: obsIntegration.isStreaming.value, recording: obsIntegration.isRecording.value, canvasWidth: overlayManager.canvasWidth.value, canvasHeight: overlayManager.canvasHeight.value }"
      :server-loading="overlayManager.loading.value"
      :obs-loading="obsIntegration.loading.value"
      :export-loading="publishing"
      :auto-sync-enabled="obsIntegration.enableAutoSync.value"
      @update:auto-save-enabled="autoSaveEnabled = $event"
      @update:show-preview-elements="showPreviewElements = $event"
      @save-project="saveCurrentProject"
      @create-project="createProject"
      @show-project-manager="showProjectManager = true"
      @update-project-name="updateProjectName"
      @undo="undo"
      @redo="redo"
      @toggle-grid="showGrid = !showGrid"
      @toggle-snap-to-grid="snapToGrid = !snapToGrid"
      @toggle-preview-elements="showPreviewElements = !showPreviewElements"
      @sync-obs="syncWithOBS"
      @export-obs="exportToOBS"
      @open-add-element="showMarketplace = true"
      @open-settings="showThemeSettings = true"
    )
    div(v-if="v3Sync.lastSyncError.value" role="alert" class="px-4 py-2 text-sm bg-red-950 text-red-100")
      | Output sync failed: {{ v3Sync.lastSyncError.value }}
    div(class="flex items-center justify-between gap-3 px-4 py-1 text-xs bg-themeBackground2 text-themeText2 border-b border-themeBorder")
      span Draft · edits are saved here. Publish to update live output. Canvas Apply also publishes its layout.
      button(class="underline text-themeText" @click="showRehearsal = true") Rehearse
      button(v-if="canvasElements.some(element => element.pendingImportReview)" class="underline text-themeText" @click="showImportReview = true") Review imported content
      span(v-if="currentDocument") {{ currentDocument.publishedRevision === currentDocument.revision ? 'Published' : 'Unpublished changes' }}
    div(v-if="compositionLibrary.error.value" role="alert" class="px-4 py-2 text-sm text-themeText bg-themeBackground2")
      span {{ compositionLibrary.error.value }}
      button(class="ml-3 underline" @click="reloadComposition") Reload saved draft
    OverlayRehearsalModal(:show="showRehearsal" :content="rehearsalContent" @close="showRehearsal = false")
    OverlayModal(:show="showImportReview" title="Review imported custom content" subtitle="Custom markup, scripts and styles remain inactive until you approve them. Package access is reviewed separately." modal-class="max-w-4xl max-h-[85vh]" @close="showImportReview = false")
      pre(class="p-4 overflow-auto text-xs whitespace-pre-wrap text-themeText bg-themeBackground2") {{ JSON.stringify(canvasElements.filter(element => element.pendingImportReview), null, 2) }}
      template(#footer)
        button(class="px-4 py-2 rounded bg-themeButton text-white" @click="approveImportedContent") I reviewed this content — activate in draft
    
    //- Main content area
    div(class="flex-1 flex overflow-hidden relative" :class="publishing ? 'pointer-events-none' : ''")
      OverlayLeftSidebar(
        :left-panel-width="leftPanelWidth"
        :is-resizing="isResizing"
        :active-library-tab="activeLibraryTab"
        :search-query="searchQuery"
        :current-theme="currentTheme"
        :library-tabs="libraryTabs"
        :builtin-categories="builtinCategories"
        :stream-widgets="streamWidgets"
        :stream-alerts="streamAlerts"
        :favorite-elements="favoriteElements"
        @update:left-panel-width="leftPanelWidth = $event"
        @update:active-library-tab="activeLibraryTab = $event"
        @update:search-query="searchQuery = $event"
        @show-themes="showThemes = true"
        @show-marketplace="showMarketplace = true"
        @show-theme-settings="showThemeSettings = true"
        @show-component-creator="showComponentCreator = true"
        @apply-theme="applyTheme"
        @add-element="addElement"
        @remove-favorite="removeFavorite"
        @resize-start="isResizing = true"
        @resize-end="isResizing = false"
      )
      
      OverlayCanvas(
        :canvas-size="canvasSize"
        :zoom="zoom"
        :canvas-sizes="canvasSizes"
        :canvas-width="overlayManager.canvasWidth.value"
        :canvas-height="overlayManager.canvasHeight.value"
        :canvas-aspect-ratio="canvasAspectRatio"
        :canvas-base-width="canvasBaseWidth"
        :canvas-base-height="canvasBaseHeight"
        :canvas-scale="canvasScale"
        :show-grid="showGrid"
        :show-safe-zones="showSafeZones"
        :show-preview-elements="showPreviewElements"
        :visible-canvas-elements="visibleCanvasElements"
        :selected-element="selectedElement"
        :is-dragging="isDragging"
        :is-element-resizing="isElementResizing"
        :resize-handle="resizeHandle"
        @update:canvas-size="selectCanvasOrientation"
        @update:canvas-scale="canvasScale = $event"
        @open-settings="showCanvasSettings = true"
        @open-add-element="showMarketplace = true"
        @zoom-in="zoom += 10"
        @zoom-out="zoom -= 10"
        @canvas-click="handleCanvasClick"
        @element-mousedown="startDrag"
        @element-click="selectElement"
        @resize-start="startResize"
        @drag-end="onMouseUp"
        @drop-element="handleDropElement"
      )
      
      //- Floating right panel (overlays canvas)
      Transition(name="slide-right")
        OverlayRightPanel(
          v-if="showRightPanel"
          class="absolute top-2 right-2 z-20"
          :active-tab="activeTab"
          :right-tabs="rightTabs"
          :selected-element="selectedElement"
          :layers="layers"
          :visible-layers-count="visibleLayersCount"
          @close="showRightPanel = false"
          @update:active-tab="activeTab = $event"
          @update-element-property="updateElementProperty"
          @update-element-style="updateElementStyle"
          @update-element-data="updateElementData"
          :settings-busy="settingsBusy"
          :settings-error="settingsError"
          @apply-widget-settings="applyWidgetSettings"
          @duplicate-element="duplicateElement"
          @reset-element-style="resetElementStyle"
          @bring-to-front="bringToFront"
          @delete-element="deleteElement"
          @add-layer="addLayer"
          @select-layer="selectLayer"
          @toggle-layer-visibility="toggleLayerVisibility"
        )

      //- Toggle button to re-open right panel
      button(
        v-if="!showRightPanel"
        @click="showRightPanel = true"
        class="absolute top-2 right-2 z-20 p-2 bg-white/80 backdrop-blur-xl border border-neutral-200/40 rounded-lg shadow-sm hover:bg-white transition-colors"
        title="Show properties panel"
      )
        svg(class="w-4 h-4 text-neutral-600" fill="none" stroke="currentColor" viewBox="0 0 24 24")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h7")
    
    //- Context Menu
    ContextMenu(
      :show="showContextMenu"
      :position="contextMenuPosition"
      :menu-items="contextMenuItems"
      @close="hideContextMenu"
      @menu-action="handleContextMenuAction"
    )

    //- Modals
    ThemeBrowserModal(
      :show="showThemes"
      :theme-categories="themeCategories"
      :current-themes="currentThemes"
      :active-category="activeThemeCategory"
      :current-category-name="getCurrentThemeCategoryName"
      :current-theme="currentTheme"
      @close="showThemes = false"
      @update:active-category="activeThemeCategory = $event"
      @apply-theme="applyTheme"
      @preview-theme="previewTheme"
      @browse-store="showMarketplace = true"
    )

    TemplateLibraryModal(
      :show="showTemplates"
      :template-categories="templateCategories"
      :current-templates="currentTemplates"
      :active-category="activeTemplateCategory"
      :current-category-name="getCurrentTemplateCategoryName"
      :saved-templates="savedTemplates"
      @close="showTemplates = false"
      @update:active-category="activeTemplateCategory = $event"
      @save-template="saveTemplate"
      @use-template="useTemplate"
      @preview-template="previewTemplate"
      @browse-marketplace="showMarketplace = true"
    )

    MarketplaceModal(
      :show="showMarketplace"
      :marketplace-categories="marketplaceCategories"
      :marketplace-items="marketplaceItems"
      :active-category="activeMarketplaceCategory"
      :search-query="marketplaceSearchQuery"
      @close="showMarketplace = false"
      @update:active-category="activeMarketplaceCategory = $event"
      @update:search-query="marketplaceSearchQuery = $event"
      @add-item="addMarketplaceItem"
      @open-marketplace="openPackageMarketplace"
    )

    ProjectManagerModal(
      :show="showProjectManager"
      :projects="projects"
      :total-storage="totalStorage"
      @close="showProjectManager = false"
      @create-project="createProject"
      @load-project="loadProject"
      @duplicate-project="duplicateProject"
      @delete-project="deleteProject"
      @import-project="importProject"
      @export-all="exportAllProjects"
    )

    ThemeSettingsModal(
      :show="showThemeSettings"
      :current-theme="currentTheme"
      @close="showThemeSettings = false"
      @apply-theme="applyThemeSettings"
      @export-theme="exportTheme"
    )

    CanvasSettingsModal(
      :show="showCanvasSettings"
      :current-width="overlayManager.canvasWidth.value"
      :current-height="overlayManager.canvasHeight.value"
      :show-grid="showGrid"
      :snap-to-grid="snapToGrid"
      :show-safe-zones="showSafeZones"
      :overlay-url="serverConnection.serverUrl.value"
      :obs-canvas="obsIntegration.canvasSize.value"
      :busy="canvasApplyBusy"
      @close="showCanvasSettings = false"
      @apply-settings="handleApplyCanvasSettings"
    )

    ComponentCreator(
      :show="showComponentCreator"
      @close="showComponentCreator = false"
      @save="saveCustomComponent"
    )
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useMainStore } from "@/stores/Main";

// Import overlay manager composables
import { useOverlayManager } from "@desktopStreaming/composables/overlay/useOverlayManager";
import { useOverlayV3Sync } from "@desktopStreaming/composables/overlay/useOverlayV3Sync";
import {
  getElementDefaults,
  getComponentSchema,
  migrateLegacyComponentStyle,
} from "@desktopStreaming/composables/overlay/componentSchema";
import { themeCategories, getThemesByCategory } from "@desktopStreaming/composables/overlay/themeRegistry";
import type { ThemeDef } from "@desktopStreaming/composables/overlay/useOverlayTheme";
import IconDuplicate from "~icons/heroicons/document-duplicate";
import IconArrowUp from "~icons/heroicons/arrow-up";
import IconArrowDown from "~icons/heroicons/arrow-down";
import IconRefresh from "~icons/heroicons/arrow-path";
import IconPaintBrush from "~icons/heroicons/paint-brush";
import IconTrash from "~icons/heroicons/trash";
import { invoke } from "@tauri-apps/api/core";
import TextElementProperties from "@desktopStreaming/components/overlay/TextElementProperties.vue";
import DefaultElementProperties from "@desktopStreaming/components/overlay/DefaultElementProperties.vue";
import ComponentCreator from "@desktopStreaming/components/overlay/ComponentCreator.vue";
import {
  copyWidgetCapabilities,
  type PackageInfo,
  type WidgetCapabilities,
  widgetContributions,
} from "@desktopStreaming/composables/overlay/packageManifest";
import { useWidgetEventForwarder } from "@desktopStreaming/composables/overlay/useWidgetEventForwarder";
import { useCapabilityDialog } from "@desktop/composables/useCapabilityDialog";
import ContextMenu from "@desktopStreaming/components/overlay/ContextMenu.vue";
import ThemeBrowserModal from "@desktopStreaming/components/overlay/ThemeBrowserModal.vue";
import TemplateLibraryModal from "@desktopStreaming/components/overlay/TemplateLibraryModal.vue";
import MarketplaceModal from "@desktopStreaming/components/overlay/MarketplaceModal.vue";
import ProjectManagerModal from "@desktopStreaming/components/overlay/ProjectManagerModal.vue";
import ThemeSettingsModal from "@desktopStreaming/components/overlay/ThemeSettingsModal.vue";
import CanvasSettingsModal from "@desktopStreaming/components/overlay/CanvasSettingsModal.vue";
import { commitCanvas } from "@desktopStreaming/composables/overlay/canvasTransaction";
import OverlayRehearsalModal from "@desktopStreaming/components/overlay/OverlayRehearsalModal.vue";
import OverlayModal from "@desktopStreaming/components/overlay/OverlayModal.vue";
import { useOverlayCompositionLibrary, cloneComposition, compositionProject, type CompositionProject, type CompositionDocument } from "@desktopStreaming/composables/overlay/useOverlayCompositionLibrary";

interface CanvasChildElement {
  relativeX: number;
  relativeY: number;
  relativeWidth: number;
  relativeHeight: number;
  [key: string]: unknown;
}

interface CanvasElement {
  id: string;
  type: string;
  name: string;
  x: number;
  y: number;
  width: number;
  height: number;
  visible: boolean;
  selected: boolean;
  locked?: boolean;
  zIndex?: number;
  content?: string;
  customHTML?: string;
  icon?: string;
  style?: Record<string, string>;
  data?: Record<string, unknown>;
  config?: Record<string, unknown>;
  configSchema?: Record<string, unknown>;
  configRevision?: number;
  schemaId?: string;
  themeClass?: string;
  childElements?: CanvasChildElement[];
  componentDefinition?: unknown;
  componentTemplate?: unknown;
  isComponentGroup?: boolean;
  instanceOverrides?: {
    allowStyleOverride?: boolean;
    allowContentOverride?: boolean;
    allowPositionOverride?: boolean;
    lockAspectRatio?: boolean;
  };
  instanceValues?: Record<string, unknown>;
  instanceStyleOverrides?: Record<string, unknown>;
  designWidth?: number;
  designHeight?: number;
  scaleContent?: boolean;
  [key: string]: unknown;
}

type OverlayThemeState = ThemeDef & { cssVars: Record<string, string> };

interface OverlayProject {
  id: string;
  name: string;
  created: string;
  lastModified: string;
  elements: CanvasElement[];
  settings: {
    canvasSize: string;
    canvasWidth?: number;
    canvasHeight?: number;
    zoom: number;
    theme: OverlayThemeState;
  };
}

interface PackageLifecycleUiEvent {
  package_id: string;
  phase: "awaiting_review" | "active" | "granted_inactive" | "unavailable" | "uninstalled" | string;
  message: string;
  needs_review: boolean;
}

interface OverlayLibraryWidget {
  id: string;
  widgetId: string;
  contributionId: string;
  contributionIndex: number;
  configSchema?: Record<string, unknown>;
  schemaId?: string;
  name: string;
  icon: string;
  description: string;
  preview: string;
  type: string;
  requiresConsent: boolean;
  packageInfo: PackageInfo;
  trust: string;
  capabilities: WidgetCapabilities;
  customHTML?: string;
  content?: string;
  width: number;
  height: number;
}

interface OverlayLibraryItem {
  sourceItem: OverlayLibraryWidget;
}

const mainStore = useMainStore();
const packageWidgets = ref<OverlayLibraryWidget[]>([]);

// Initialize stream graphics elements

// Define layout
definePageMeta({
  layout: "streaming",
  title: "Stream Sessions",
  description: "Manage your streaming session templates and configurations",
});

// Removed activeScene - using project-based system now
const canvasSize = computed(() => overlayManager.canvasHeight.value > overlayManager.canvasWidth.value ? "mobile" : "desktop");
const zoom = ref(100);
const activeTab = ref("Properties");
const activeLibraryTab = ref("theme");
const showPreviewElements = ref(true);

// Project system
const currentProject = ref<OverlayProject | null>(null);
const projects = ref<OverlayProject[]>([]);
const compositionLibrary = useOverlayCompositionLibrary();
const currentDocument = computed(() => currentProject.value ? compositionLibrary.documents.value[currentProject.value.id] : undefined);
let restoringComposition = false;
const showProjectManager = ref(false);
const autoSaveEnabled = ref(true);
const lastSaved = ref<string | null>(null);
const hasUnsavedChanges = ref(false);
const publishing = ref(false);
const showProjectDropdown = ref(false);
const isEditingProjectName = ref(false);
const editingProjectName = ref("");
const totalStorage = ref("2.4 MB");

// Modal states
const showThemes = ref(false);
const showTemplates = ref(false);
const showMarketplace = ref(false);
const showThemeSettings = ref(false);
const showCanvasSettings = ref(false);
const showComponentCreator = ref(false);
const showRehearsal = ref(false);
const rehearsalContent = computed(() => ({ name: currentProject.value?.name || 'Draft', elements: canvasElements.value,
  settings: { canvasWidth: overlayManager.canvasWidth.value, canvasHeight: overlayManager.canvasHeight.value, theme: currentTheme.value } }));

const canvasApplyBusy = ref(false);
const handleApplyCanvasSettings = async (settings: {
  width: number;
  height: number;
  showGrid: boolean;
  snapToGrid: boolean;
  showSafeZones: boolean;
}) => {
  if (canvasApplyBusy.value || publishing.value) return;
  canvasApplyBusy.value = true;
  publishing.value = true;
  restoringComposition = true;
  let saved = false;
  let savedDocument: CompositionDocument | null = null;
  try {
    const to = { width: settings.width, height: settings.height };
    await commitCanvas({
      from: { width: overlayManager.canvasWidth.value, height: overlayManager.canvasHeight.value },
      to, elements: canvasElements.value,
      persist: async (elements, canvas) => {
        if (!currentProject.value) throw new Error('Create a composition before applying its canvas');
        savedDocument = await compositionLibrary.save({ ...cloneComposition(currentProject.value), elements: cloneComposition(elements),
          settings: { ...cloneComposition(currentProject.value.settings), canvasWidth: canvas.width, canvasHeight: canvas.height, zoom: zoom.value, theme: cloneComposition(currentTheme.value) },
        } as unknown as CompositionProject);
        return true;
      },
      commitEditor: (elements, canvas) => {
        canvasElements.value = elements;
        overlayManager.setCanvasDimensions(canvas.width, canvas.height);
      },
    });
    saved = true;
    showGrid.value = settings.showGrid;
    snapToGrid.value = settings.snapToGrid;
    showSafeZones.value = settings.showSafeZones;
    const document = savedDocument as CompositionDocument | null;
    if (!document) throw new Error('Canvas draft was not saved');
    const published = await v3Sync.syncElementsToBackend(cloneComposition(document.content.elements), to,
      { compositionId: document.id, expectedRevision: document.revision });
    if (!published) throw new Error(`Draft canvas saved; publication failed: ${v3Sync.lastSyncError.value}`);
    const accepted = await compositionLibrary.reload(document.id);
    await loadProject(compositionProject(accepted) as unknown as OverlayProject, false);
    const projectIndex = projects.value.findIndex(project => project.id === accepted.id);
    if (projectIndex >= 0) projects.value[projectIndex] = cloneComposition(currentProject.value!);
    if (!obsIntegration.isConnected.value) {
      mainStore.addNotification({ type: "info", title: "Canvas saved", message: "OBS is offline. Apply these settings again after connecting to update its base canvas." });
      return;
    }
    const confirmed = await obsIntegration.applyCanvasSize(to.width, to.height);
    if (confirmed.width !== to.width || confirmed.height !== to.height) {
      throw new Error(`OBS read back ${confirmed.width}×${confirmed.height}; saved canvas is ${to.width}×${to.height}`);
    }
    const sourceConfirmed = confirmed.managed_source_found === true && confirmed.managed_source_matches === true;
    mainStore.addNotification({
      key: "overlay-canvas-applied", type: sourceConfirmed ? "success" : "warning",
      title: sourceConfirmed ? "Canvas sizes match" : "OBS canvas updated; source unconfirmed",
      message: sourceConfirmed ? `Overlay and OBS use ${to.width}×${to.height}. Widget rendering is checked separately.` : "The managed overlay source is missing or has not confirmed its viewport. Refresh or add it, then check synchronization.",
    });
  } catch (error) {
    mainStore.addNotification({ key: "overlay-canvas-apply-failed",
      title: saved ? "Canvas saved; synchronization incomplete" : "Canvas was not saved",
      message: String(error), type: "error", duration: 9000 });
  } finally { await nextTick(); restoringComposition = false; canvasApplyBusy.value = false; publishing.value = false; }
};

const selectCanvasOrientation = async (orientation: string) => {
  const width = overlayManager.canvasWidth.value;
  const height = overlayManager.canvasHeight.value;
  await handleApplyCanvasSettings({
    width: orientation === "mobile" ? Math.min(width, height) : Math.max(width, height),
    height: orientation === "mobile" ? Math.max(width, height) : Math.min(width, height),
    showGrid: showGrid.value, snapToGrid: snapToGrid.value, showSafeZones: showSafeZones.value,
  });
};

// The dialog is global (mounted once in app.vue). Packages discovered by the
// backend open it from the library when consent is missing or stale.
const capabilityDialog = useCapabilityDialog();
// Phase 4: forwards live stream events to sandboxed community-widget iframes,
// filtered by each widget's granted data-subscribe (least privilege).
const widgetForwarder = useWidgetEventForwarder();

// Theme system — single source of truth in useOverlayTheme
const { currentTheme, applyTheme: applyThemeFn, patchVars, mountTheme } = useOverlayTheme();


const activeThemeCategory = ref("gaming");
// themeCategories imported from themeRegistry


// Template system
const activeTemplateCategory = ref("gaming");
const savedTemplates = ref<string[]>([]);
const templateCategories = ref([
  { id: "gaming", name: "Gaming", icon: "heroicons:puzzle-piece", count: 15 },
  { id: "just-chatting", name: "Just Chatting", icon: "heroicons:chat-bubble-left-right", count: 12 },
  { id: "creative", name: "Creative", icon: "heroicons:paint-brush", count: 8 },
  { id: "music", name: "Music", icon: "heroicons:musical-note", count: 6 },
]);

// Overlay component library. Discovery/install remains owned by the canonical
// package marketplace; this is only an editor projection of current packages.
const activeMarketplaceCategory = ref("all");
const marketplaceSearchQuery = ref("");
const marketplaceCategories = computed(() => [
  { id: "all", name: "All Components", count: packageWidgets.value.length },
  {
    id: "builtin",
    name: "VortiDeck Built-ins",
    count: packageWidgets.value.filter((item) => item.trust === "builtin").length,
  },
  {
    id: "packages",
    name: "Installed Packages",
    count: packageWidgets.value.filter((item) => item.trust !== "builtin").length,
  },
]);

const marketplaceItemsCatalog = computed(() => packageWidgets.value.map((widget) => {
  const packageInfo = widget.packageInfo as PackageInfo | undefined;
  return {
    id: widget.id,
    category: widget.trust === "builtin" ? "builtin" : "packages",
    name: widget.name,
    author: packageInfo?.author || (widget.trust === "builtin" ? "VortiDeck" : "Local package"),
    description: widget.description || packageInfo?.description || "Overlay component",
    icon: widget.icon || "heroicons:squares-plus",
    versionLabel: packageInfo?.version ? `v${packageInfo.version}` : "Built in",
    trust: widget.trust,
    requiresConsent: Boolean(widget.requiresConsent),
    sourceItem: widget,
  };
}));

const filteredMarketplaceItems = computed(() => {
  let items = marketplaceItemsCatalog.value;

  if (activeMarketplaceCategory.value !== "all") {
    items = items.filter((item) => item.category === activeMarketplaceCategory.value);
  }

  if (marketplaceSearchQuery.value.trim()) {
    const q = marketplaceSearchQuery.value.toLowerCase().trim();
    items = items.filter(
      (item) =>
        item.name.toLowerCase().includes(q) ||
        item.author.toLowerCase().includes(q) ||
        item.description.toLowerCase().includes(q)
    );
  }

  return items;
});

const marketplaceItems = filteredMarketplaceItems;

const addMarketplaceItem = (item: OverlayLibraryItem) => {
  if (!item?.sourceItem) return;
  addElement(item.sourceItem);
};

const openPackageMarketplace = async () => {
  showMarketplace.value = false;
  await navigateTo("/deck/store/plugins?category=widgets&returnTo=/streaming/overlays");
};


// Computed properties for themes
const currentThemes = computed(() => {
  return getThemesByCategory(activeThemeCategory.value);
});

const getCurrentThemeCategoryName = computed(() => {
  const category = themeCategories.find((c) => c.id === activeThemeCategory.value);
  return category ? category.name + " Themes" : "Themes";
});

const currentTemplates = computed(() => {
  const templates = [
    {
      id: "template-1",
      name: "FPS Competitive",
      description: "Clean HUD for competitive gaming",
      type: "gaming",
      elements: 8,
      animations: true,
      isPro: false,
      icon: "heroicons:viewfinder-circle",
    },
    {
      id: "template-2",
      name: "Chat Focused",
      description: "Perfect for just chatting streams",
      type: "just-chatting",
      elements: 6,
      animations: false,
      isPro: false,
      icon: "heroicons:chat-bubble-left-right",
    },
    {
      id: "template-3",
      name: "Creative Studio",
      description: "Great for art and creative content",
      type: "creative",
      elements: 10,
      animations: true,
      isPro: true,
      icon: "heroicons:paint-brush",
    },
  ];
  return templates.filter(
    (template) =>
      activeTemplateCategory.value === "all" ||
      template.type === activeTemplateCategory.value
  );
});

const getCurrentTemplateCategoryName = computed(() => {
  const category = templateCategories.value.find(
    (c) => c.id === activeTemplateCategory.value
  );
  return category ? category.name : "Gaming Templates";
});

const saveTemplate = (template: any) => {
  if (!savedTemplates.value.includes(template.id)) {
    savedTemplates.value.push(template.id);
  }
  console.log("Template saved:", template.name);
};

const useTemplate = (template: any) => {
  console.log("Using template:", template.name);
  // Apply template logic here
};

const previewTemplate = (template: any) => {
  console.log("Previewing template:", template.name);
  // Preview template logic here
};

const loadProject = async (project: OverlayProject, syncBackend = true) => {
  console.log("Loading project:", project.name);

  // Set the current project
  restoringComposition = true;
  currentProject.value = cloneComposition(project);

  // Load the project's elements into the canvas
  if (project.elements && Array.isArray(project.elements)) {
    canvasElements.value = cloneComposition(project.elements);
    console.log(`Loaded ${project.elements.length} elements from project`);
  } else {
    canvasElements.value = [];
    console.log("No elements in project, starting with empty canvas");
  }

  // Load project settings if they exist
  if (project.settings) {
    overlayManager.setCanvasDimensions(
      project.settings.canvasWidth || overlayManager.canvasWidth.value,
      project.settings.canvasHeight || overlayManager.canvasHeight.value,
    );
    if (project.settings.zoom) {
      zoom.value = project.settings.zoom;
    }
    if (project.settings.theme) {
      currentTheme.value = project.settings.theme;
      // Re-apply the restored theme end-to-end. On restart, onMounted bakes the
      // DEFAULT theme into the editor canvas + OBS :root before this project
      // loads — so without re-applying here, editor and OBS desync from the
      // (correct) currentTheme and it looks like the theme reverted. This also
      // covers manual project switching from the project manager.
      mountTheme(); // editor canvas DOM vars
    }
  }

  // Clear selection
  reconcileCanvasPackageWidgets();
  selectedElement.value = null;

  // Close the project manager
  showProjectManager.value = false;

  // Reset unsaved changes flag
  hasUnsavedChanges.value = false;

  console.log("Project loaded successfully:", project.name);
  await nextTick();
  restoringComposition = false;
  if (syncBackend) {
    const synced = await v3Sync.initialSync(canvasElements.value, {
      width: overlayManager.canvasWidth.value,
      height: overlayManager.canvasHeight.value,
    });
    if (!synced) {
      mainStore.addNotification({
        key: "overlay-project-live-sync-failed",
        title: "Project opened in the editor",
        message: `The live overlay still uses its previous saved design: ${v3Sync.lastSyncError.value || "native sync failed"}`,
        type: "error",
        autoClose: true,
        duration: 9000,
      });
    }
  }
};

const duplicateProject = (project: OverlayProject) => {
  // A copy owns its settings and instance identities; sharing either lets a
  // later sync or an agent edit affect the original project's widgets.
  const cloned: OverlayProject = JSON.parse(JSON.stringify(project));
  cloned.elements.forEach(element => {
    element.id = generateProjectId();
    element.configRevision = 0;
    delete element.nativeSyncPending;
  });
  const duplicate = {
    ...cloned,
    id: `${project.id}_copy_${Date.now()}`,
    name: `${project.name} Copy`,
    lastModified: new Date().toISOString(),
  };
  projects.value.push(duplicate);
  saveProjectsToStorage();
  console.log("Project duplicated:", duplicate.name);
};

const deleteProject = async (projectId: string) => {
  try { await compositionLibrary.remove(projectId); } catch { return; }
  const index = projects.value.findIndex((p) => p.id === projectId);
  if (index > -1) {
    projects.value.splice(index, 1);
    if (currentProject.value?.id === projectId) { currentProject.value = null; canvasElements.value = []; }
    console.log("Project deleted:", projectId);
  }
};

const showImportReview = ref(false);
const approveImportedContent = async () => {
  try {
    const saved = await saveCurrentProject();
    if (!saved) return;
    const document = await invoke<CompositionDocument>("approve_imported_overlay_content", { id: saved.id, expectedRevision: saved.revision });
    compositionLibrary.accept(document);
    await loadProject(compositionProject(document) as unknown as OverlayProject, false);
    const index = projects.value.findIndex(project => project.id === document.id);
    if (index >= 0) projects.value[index] = compositionProject(document) as unknown as OverlayProject;
    showImportReview.value = false;
  } catch (cause) { settingsError.value = String(cause); }
};

const importProject = async () => {
  try {
    await compositionLibrary.flush();
    const result = await invoke<{ documents: CompositionDocument[]; diagnostics: { message: string }[] } | null>("import_overlay_setup");
    if (!result) return;
    for (const document of result.documents) {
      compositionLibrary.accept(document);
      projects.value.push(compositionProject(document) as unknown as OverlayProject);
    }
    settingsError.value = result.diagnostics.map(issue => issue.message).join(" · ");
  } catch (cause) { settingsError.value = String(cause); }
};

const exportAllProjects = async () => {
  try {
    if (currentProject.value && !await saveCurrentProject()) return;
    await compositionLibrary.flush();
    await invoke("export_overlay_setup");
  } catch (cause) { settingsError.value = String(cause); }
};

const applyThemeSettings = async (settings: any) => {
  const overrides: Record<string, string> = {};
  if (settings.colors) {
    const names = ["--theme-primary", "--theme-secondary", "--theme-accent", "--theme-bg", "--theme-text", "--theme-surface"];
    settings.colors.forEach((c: string, i: number) => { if (names[i]) overrides[names[i]] = c; });
  }
  if (settings.settings?.borderRadius != null) {
    overrides["--theme-border-radius"] = `${settings.settings.borderRadius}px`;
  }
  patchVars(overrides);
  // Store + bake merged CSS vars and broadcast UPDATE_VARS live (no reload).
  markUnsavedChanges();
};

const exportTheme = () => {
  console.log("Exporting current theme");
};

const saveCustomComponent = (componentDefinition: any) => {
  console.log("Saving custom component:", componentDefinition);

  // Add the component elements to the current overlay
  addComponentTemplate(componentDefinition);

  // TODO: Also save component to user's library for future reuse
  showComponentCreator.value = false;

  console.log(
    `Component "${componentDefinition.name}" added to overlay with ${componentDefinition.elements.length} elements`
  );
};

// Function to add a component template to the overlay as a single grouped component
const addComponentTemplate = (componentDef: any) => {
  // NEW APPROACH: Use the component's designed size and position children relative to (0,0)
  // No more bounding box calculations - use the fixed design area size

  const COMPONENT_DESIGN_WIDTH = 300; // Fixed design area width
  const COMPONENT_DESIGN_HEIGHT = 200; // Fixed design area height

  // Place component in center of canvas initially
  const canvasCenterX = canvasBaseWidth.value / 2 - COMPONENT_DESIGN_WIDTH / 2;
  const canvasCenterY = canvasBaseHeight.value / 2 - COMPONENT_DESIGN_HEIGHT / 2;

  // Create a single grouped component element
  const groupedComponent = {
    id: `component_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
    type: "component",
    name: componentDef.name,

    // Component position on main canvas (where user can drag it)
    x: canvasCenterX,
    y: canvasCenterY,

    // Component's designed size (fixed from creator)
    width: COMPONENT_DESIGN_WIDTH,
    height: COMPONENT_DESIGN_HEIGHT,

    visible: true,
    selected: false,
    themeClass: "overlay-card",

    // Component-specific properties
    componentDefinition: componentDef,
    isComponentGroup: true,

    // Store child elements with positions RELATIVE to component's (0,0)
    childElements: componentDef.elements.map((element: any, index: number) => {
      return {
        ...element,
        id: `${element.id}_child`,

        // Use element's ORIGINAL position from the creator as relative position
        // This preserves the exact layout designed in the creator
        relativeX: element.x,
        relativeY: element.y,
        relativeWidth: element.width,
        relativeHeight: element.height,

        // Preserve all original properties
        content: element.content,
        style: element.style,
        type: element.type,
        name: element.name,
      };
    }),

    // Component instance overrides
    instanceOverrides: {
      allowStyleOverride: true,
      allowContentOverride: false, // Content changes should be on child elements
      allowPositionOverride: true,
      lockAspectRatio: false,
    },
  };

  canvasElements.value.push(groupedComponent);
  markUnsavedChanges();

  console.log(
    `Component "${componentDef.name}" added as single grouped element with ${componentDef.elements.length} child elements`
  );
  console.log("Grouped component:", groupedComponent);
  console.log("Child elements:", groupedComponent.childElements);
};

// Helper function to get theme class for element type
const getThemeClassForElement = (elementType: string) => {
  // Apply appropriate theme classes based on element type
  switch (elementType) {
    case "text":
    case "chat":
    case "counter":
      return "overlay-element";
    case "webcam":
    case "healthbar":
    case "component":
      return "overlay-card";
    default:
      return "overlay-element";
  }
};

// Handle click outside dropdown to close it
const handleClickOutside = (event: MouseEvent) => {
  const dropdownElement = document.querySelector(".project-dropdown");
  if (
    showProjectDropdown.value &&
    dropdownElement &&
    !dropdownElement.contains(event.target as Node)
  ) {
    showProjectDropdown.value = false;
  }
};

// Template ref for project name input
const projectNameInput = ref<HTMLInputElement | null>(null);

// Start editing project name
const startEditingProjectName = () => {
  if (!currentProject.value) return;
  isEditingProjectName.value = true;
  editingProjectName.value = currentProject.value.name;
  nextTick(() => {
    if (projectNameInput.value) {
      projectNameInput.value.focus();
      projectNameInput.value.select();
    }
  });
};

// Save project name changes
const saveProjectName = () => {
  if (!currentProject.value || !editingProjectName.value.trim()) {
    isEditingProjectName.value = false;
    return;
  }

  currentProject.value.name = editingProjectName.value.trim();
  currentProject.value.lastModified = new Date().toISOString();

  const projectId = currentProject.value.id;
  const projectIndex = projects.value.findIndex((p) => p.id === projectId);
  if (projectIndex > -1) {
    projects.value[projectIndex] = currentProject.value;
  }

  saveProjectsToStorage();
  isEditingProjectName.value = false;
  markUnsavedChanges();
};

// Cancel editing project name
const cancelEditingProjectName = () => {
  isEditingProjectName.value = false;
  editingProjectName.value = "";
};

const updateProjectName = (name: string) => {
  if (currentProject.value) {
    currentProject.value.name = name;
    hasUnsavedChanges.value = true;
  }
};
const isDragging = ref(false);
const dragStart = ref({ x: 0, y: 0 });
const elementStart = ref({ x: 0, y: 0 });
const showContextMenu = ref(false);
const contextMenuPosition = ref({ x: 0, y: 0 });
const contextMenuTarget = ref<CanvasElement | null>(null);
const selectedElement = ref<CanvasElement | null>(null);
const dragElement = ref<CanvasElement | null>(null);
const dragOffset = ref({ x: 0, y: 0 });
const isElementResizing = ref(false);
const resizeElement = ref<CanvasElement | null>(null);
const resizeHandle = ref("");

// Canvas elements with their properties
// Build initial elements from the registry so styles use theme vars consistently
const makeInitialElement = (
  typeId: string,
  overrides: Pick<CanvasElement, "id" | "name" | "x" | "y" | "width" | "height"> & Partial<CanvasElement>,
): CanvasElement => {
  const defaults = getElementDefaults(typeId);
  return {
    ...defaults,
    ...overrides,
    id: overrides.id,
    type: overrides.type ?? defaults?.type ?? typeId,
    name: overrides.name,
    x: overrides.x,
    y: overrides.y,
    width: overrides.width,
    height: overrides.height,
    visible: overrides.visible ?? true,
    selected: overrides.selected ?? false,
  };
};

const canvasElements = ref<CanvasElement[]>([
  makeInitialElement("webcam-frame", {
    id: "webcam",
    name: "Webcam Frame",
    x: 1248,
    y: 864,
    width: 192,
    height: 144,
  }),
  makeInitialElement("chat", {
    id: "chat",
    name: "Chat Box",
    x: 1200,
    y: 24,
    width: 288,
    height: 192,
  }),
  makeInitialElement("text", {
    id: "title-text",
    name: "Stream Title",
    content: "Welcome to My Stream!",
    x: 400,
    y: 50,
    width: 300,
    height: 80,
    style: {
      fontSize: "24px",
      fontFamily: "var(--theme-font-primary)",
      fontWeight: "bold",
      color: "var(--theme-text)",
      textAlign: "center",
      backgroundColor: "transparent",
      borderStyle: "none",
    },
  }),
  makeInitialElement("counter", {
    id: "followers",
    name: "Follower Count",
    x: 24,
    y: 24,
    width: 180,
    height: 90,
  }),
]);

const searchQuery = ref("");
const favoriteElements = ref([
  { id: "fav1", name: "Chat Box", icon: "heroicons:chat-bubble-left-right", usageCount: 12 },
  { id: "fav2", name: "Follow Alert", icon: "heroicons:heart", usageCount: 8 },
]);
const recentElements = ref([
  { id: "rec1", name: "Donation Alert", icon: "heroicons:currency-dollar" },
  { id: "rec2", name: "Timer Widget", icon: "heroicons:clock" },
  { id: "rec3", name: "Goal Bar", icon: "heroicons:flag" },
]);

const installedPacks = ref<unknown[]>([]);
const myPurchases = ref<unknown[]>([]);
const tabScrollContainer = ref<HTMLElement | null>(null);

const leftPanel = ref<HTMLElement | null>(null);
const leftPanelWidth = ref(256);
const isResizing = ref(false);
const showGrid = ref(false);
const snapToGrid = ref(false);
const showSafeZones = ref(false);
const showRightPanel = ref(true);
const canRedo = computed(() => currentDocument.value?.canRedo === true && !compositionLibrary.busy.value);

// Initialize overlay manager first (moved from later in file)
const overlayManager = useOverlayManager();

// Every editor mode uses the persisted composition coordinate space.
const actualCanvasWidth = computed(() => overlayManager.canvasWidth.value);
const actualCanvasHeight = computed(() => overlayManager.canvasHeight.value);
const canvasAspectRatio = computed(() => `${actualCanvasWidth.value} / ${actualCanvasHeight.value}`);
const canvasBaseWidth = actualCanvasWidth;
const canvasBaseHeight = actualCanvasHeight;

// Canvas scaling is now handled by OverlayCanvas component

// Canvas dimensions (exactly like overlay.vue)
const canvasWidth = computed(() => canvasBaseWidth.value);
const canvasHeight = computed(() => canvasBaseHeight.value);

// Canvas scale factor - now updated by OverlayCanvas component
const canvasScale = ref(0.5);

// Computed property for visible layers count
const visibleLayersCount = computed(() => {
  return layers.value.filter((layer) => layer.visible).length;
});

// Computed property for visible canvas elements
const visibleCanvasElements = computed(() => {
  return canvasElements.value.filter((element) => element.visible);
});

const libraryTabs = [
  {
    id: "theme",
    label: "Current Theme",
    icon: "heroicons:swatch",
    count: computed(() => (currentTheme.value ? 1 : null)),
  },
  { id: "elements", label: "Add to Canvas", icon: "heroicons:plus-circle", count: null },
];

const streamAlerts = [
  {
    id: "follower",
    name: "New Follower",
    icon: "heroicons:heart",
    description: "When someone follows",
    preview: "Shows animated heart with username",
  },
  {
    id: "subscriber",
    name: "New Subscriber",
    icon: "heroicons:star",
    description: "New subscription alert",
    preview: "Displays sub tier and message",
  },
  {
    id: "donation",
    name: "Donation",
    icon: "heroicons:currency-dollar",
    description: "Tips and donations",
    preview: "Shows amount and donor message",
  },
  {
    id: "raid",
    name: "Raid Alert",
    icon: "heroicons:bolt",
    description: "Incoming raid notification",
    preview: "Raid animation with viewer count",
  },
  {
    id: "host",
    name: "Host Alert",
    icon: "heroicons:home",
    description: "When someone hosts",
    preview: "Host notification with channel name",
  },
];

// All widgets—including VortiDeck's embedded essentials—arrive through the
// package contribution contract. Native text/image/video/shape primitives stay
// outside this list because they are not executable widgets.
const loadPackageWidgets = async () => {
  try {
    const pkgs: PackageInfo[] = await v3Sync.refreshWidgetGrants();
    const seenPackageIds = new Set<string>();
    const widgetPackages = pkgs.filter((pkg) => {
      if (widgetContributions(pkg).length === 0) return false;
      if (seenPackageIds.has(pkg.id)) return false;
      seenPackageIds.add(pkg.id);
      return true;
    });
    const packagesWithGrantedWidgets = await Promise.all(
      widgetPackages.map(async (pkg) => ({
          pkg,
          // Runtime plugins can also contribute widgets. `has_runtime` is not
          // a grant for that widget contribution, so ask the DB-backed widget
          // grant source directly instead of filtering bundles out.
          granted: pkg.trust === "builtin"
            ? true
            : Boolean(await invoke("get_widget_permissions", { widgetId: pkg.id })),
      })),
    );

    packageWidgets.value = packagesWithGrantedWidgets.flatMap(({ pkg, granted }) => {
      const requiresConsent = pkg.trust !== "builtin" && (!granted || pkg.needs_reconsent);
      return widgetContributions(pkg).map((widget, contributionIndex) => {
        const schemaId = pkg.trust === "builtin" ? widget.schemaId : undefined;
        const defaults = schemaId ? getElementDefaults(schemaId) : undefined;
        return {
        // A package can contribute multiple widgets. The library item needs a
        // stable per-contribution ID; widgetId remains the package identity for
        // capability checks and package-level overlay targeting.
        id: `${pkg.id}:${widget.id || `widget-${contributionIndex}`}`,
        widgetId: pkg.id,
        contributionId: widget.id || `widget-${contributionIndex}`,
        contributionIndex,
        configSchema: widget.configSchema,
        schemaId,
        name: widget.name || (widgetContributions(pkg).length > 1
          ? `${pkg.name} ${contributionIndex + 1}`
          : pkg.name),
        icon: requiresConsent ? "heroicons:shield-check" : (widget.icon || "heroicons:chart-bar"),
        description: requiresConsent
          ? (pkg.needs_reconsent ? "Review updated access before adding" : "Review access before adding")
          : (widget.description || pkg.description || "Package widget"),
        preview: requiresConsent
          ? "Permission review required"
          : (pkg.trust === "builtin" ? "Built into VortiDeck" : (pkg.author ? `by ${pkg.author}` : "Package widget")),
        type: defaults?.type || "widget",
        requiresConsent,
        packageInfo: pkg,
        trust: pkg.trust,
        capabilities: widget.capabilities,
        customHTML: widget.content || defaults?.customHTML,
        content: widget.content || defaults?.content,
        width: widget.width || defaults?.width || 320,
        height: widget.height || defaults?.height || 80,
        };
      });
    });
  } catch (e) {
    console.warn("Failed to load widget packages:", e);
  }
};

const streamWidgets = computed(() => packageWidgets.value);

/**
 * Older projects stored first-party schema widgets before those widgets had a
 * package identity. Enrich only matching built-in schema instances in place;
 * dimensions, styles, data, and IDs remain untouched.
 */
const migrateSavedBuiltinWidgetMetadata = () => {
  const builtins = new Map(
    packageWidgets.value
      .filter((widget) => widget.trust === "builtin" && widget.schemaId)
      .map((widget) => [widget.schemaId, widget]),
  );
  if (builtins.size === 0) return;

  let changed = false;
  const elements = new Set<any>([
    ...canvasElements.value,
    ...projects.value.flatMap((project) => project.elements || []),
  ]);
  for (const element of elements) {
    const packageId = String(element.widgetId || "").replace(/^plugin\./, "");
    const inferredSchemaId = element.schemaId
      || (packageId === "builtin.vortideck-widgets" ? element.contributionId : undefined);
    const builtin = builtins.get(inferredSchemaId);
    if (!builtin?.schemaId) continue;

    const nextCapabilities = copyWidgetCapabilities(builtin.capabilities);
    const styleChanged = migrateLegacyComponentStyle(builtin.schemaId, element.style);
    const needsUpdate = element.schemaId !== builtin.schemaId
      || element.widgetId !== builtin.widgetId
      || element.contributionId !== builtin.contributionId
      || element.trust !== "builtin"
      || JSON.stringify(element.capabilities || {}) !== JSON.stringify(nextCapabilities);
    if (!needsUpdate && !styleChanged) continue;

    element.schemaId = builtin.schemaId;
    element.widgetId = builtin.widgetId;
    element.contributionId = builtin.contributionId;
    element.trust = "builtin";
    element.capabilities = nextCapabilities;
    changed = true;
  }

  if (changed) saveProjectsToStorage();
};

/**
 * Replace executable package content/capabilities in saved canvas instances
 * with the currently discovered, consented contribution. Layout, user data,
 * appearance and instance identity are preserved. This keeps the trusted
 * editor from re-signing stale iframe HTML after a package update.
 */
const reconcileCanvasPackageWidgets = (packageId?: string) => {
  const canonical = (id: unknown) => String(id || "").replace(/^plugin\./, "");
  let changed = false;
  for (const element of canvasElements.value) {
    if (!element.widgetId || element.trust === "builtin") continue;
    if (packageId && canonical(element.widgetId) !== canonical(packageId)) continue;
    const candidates = packageWidgets.value.filter(
      (widget) => canonical(widget.widgetId) === canonical(element.widgetId) && !widget.requiresConsent,
    );
    const current = candidates.find(
      (widget) => widget.contributionId === element.contributionId,
    ) || (candidates.length === 1 ? candidates[0] : undefined);
    if (!current) continue;

    const nextCapabilities = JSON.stringify(current.capabilities || {});
    const priorCapabilities = JSON.stringify(element.capabilities || {});
    if (
      element.name !== current.name
      || element.customHTML !== current.customHTML
      || element.content !== current.content
      || priorCapabilities !== nextCapabilities
      || element.contributionId !== current.contributionId
      || JSON.stringify(element.configSchema) !== JSON.stringify(current.configSchema)
    ) {
      element.name = current.name;
      element.customHTML = current.customHTML;
      element.content = current.content;
      element.capabilities = copyWidgetCapabilities(current.capabilities);
      element.contributionId = current.contributionId;
      element.configSchema = current.configSchema;
      element.trust = current.trust;
      element.schemaId = current.schemaId;
      changed = true;
    }
  }
  return changed;
};

const builtinCategories = computed(() => [
  {
    id: "basic",
    name: "Basic Elements",
    icon: "heroicons:cube",
    expanded: true,
    items: [
      {
        id: "text",
        name: "Text",
        icon: "heroicons:document-text",
        description: "Add custom text",
        preview: "Editable text with styling",
      },
      {
        id: "image",
        name: "Image",
        icon: "heroicons:photo",
        description: "Upload or link image",
        preview: "Static or animated images",
      },
      {
        id: "shape",
        name: "Shape",
        icon: "heroicons:square-2-stack",
        description: "Basic shapes",
        preview: "Rectangles, circles, etc.",
      },
      {
        id: "video",
        name: "Video",
        icon: "heroicons:film",
        description: "Video file or URL",
        preview: "MP4, WebM video player",
      },
    ],
  },
  {
    id: "widgets",
    name: "Widgets & Live Goals",
    icon: "heroicons:chart-bar",
    expanded: false,
    items: streamWidgets.value,
  },
  {
    id: "alerts",
    name: "Stream Alerts",
    icon: "heroicons:bell",
    expanded: false,
    items: streamAlerts,
  },
  {
    id: "stream",
    name: "Frames & Overlays",
    icon: "heroicons:tv",
    expanded: false,
    items: [
      {
        id: "webcam-frame",
        name: "Webcam Frame",
        icon: "heroicons:video-camera",
        description: "Decorative webcam border",
        preview: "Styled frame for webcam",
      },
      {
        id: "banner",
        name: "Banner",
        icon: "heroicons:megaphone",
        description: "Info banner or ticker",
        preview: "Scrolling text banner",
      },
      {
        id: "logo",
        name: "Logo Watermark",
        icon: "heroicons:sparkles",
        description: "Brand logo overlay",
        preview: "Semi-transparent logo",
      },
      {
        id: "social",
        name: "Social Media Bar",
        icon: "heroicons:share",
        description: "Social handles display",
        preview: "Twitter, Instagram, etc.",
      },
    ],
  },
  {
    id: "effects",
    name: "Animations & Effects",
    icon: "heroicons:sparkles",
    expanded: false,
    items: [
      {
        id: "particles",
        name: "Particle Effects",
        icon: "heroicons:fire",
        description: "Animated particles",
        preview: "Confetti, snow, etc.",
      },
      {
        id: "transition",
        name: "Scene Transition",
        icon: "heroicons:arrows-right-left",
        description: "Animated transitions",
        preview: "Fade, slide, wipe",
      },
      {
        id: "glow",
        name: "Glow Effect",
        icon: "heroicons:sun",
        description: "Animated glow/pulse",
        preview: "Attention-grabbing glow",
      },
      {
        id: "webgl-confetti",
        name: "Canvas Confetti",
        icon: "heroicons:sparkles",
        description: "Real physics confetti simulation",
        preview: "Interactive confetti burst",
      },
      {
        id: "webgl-fireballs",
        name: "Fireball Physics",
        icon: "heroicons:fire",
        description: "Interactive colliding fireballs",
        preview: "Exploding physics fireballs",
      },
    ],
  },
]);

const canvasSizes = [
  { id: "desktop", label: "Landscape", icon: "heroicons:computer-desktop" },
  { id: "mobile", label: "Portrait", icon: "heroicons:device-phone-mobile" },
];

const rightTabs = ["Properties", "Layers"];

// Layers computed from canvasElements (single source of truth)
const layers = computed(() => {
  return canvasElements.value.map((element) => ({
    id: element.id,
    name: element.name,
    type: element.type,
    visible: element.visible,
    selected: element.selected || selectedElement.value?.id === element.id,
  }));
});



// Essential computed properties for existing functionality
const canUndo = computed(() => currentDocument.value?.canUndo === true && !compositionLibrary.busy.value);

// Lifecycle hooks
const settingsBusy = ref(false);
const settingsError = ref('');
let unlistenWidgetSettings: UnlistenFn | null = null;
let unlistenComposition: UnlistenFn | null = null;
let unlistenCompositionReload: UnlistenFn | null = null;
const acceptWidgetSettings = (saved: any) => {
  const element = canvasElements.value.find(el => el.id === saved.elementId);
  if (!element || Number(element.configRevision || 0) > saved.configRevision) return;
  if (hasUnsavedChanges.value || currentDocument.value?.publishedRevision !== currentDocument.value?.revision) {
    settingsError.value = 'Live widget settings changed. This draft remains editable; Publish explicitly replaces the live configuration.';
    return;
  }
  element.config = saved.config;
  element.configRevision = saved.configRevision;
  markUnsavedChanges();
};
const applyWidgetSettings = async (config: Record<string, unknown>, expectedRevision: number) => {
  const element = selectedElement.value;
  if (!element || settingsBusy.value) return;
  settingsBusy.value = true;
  settingsError.value = '';
  try {
    if (Number(element.draftConfigRevision ?? element.configRevision ?? 0) !== expectedRevision) throw new Error('Draft widget settings changed; reload before applying');
    element.config = await invoke<Record<string, unknown>>('validate_draft_widget_settings', {
      packageId: element.widgetId || null, contributionId: element.contributionId || null, config,
    });
    element.draftConfigRevision = expectedRevision + 1;
    element.nativeSyncPending = false;
    markUnsavedChanges();
    await saveCurrentProject();
  } catch (error) {
    settingsError.value = String(error);
  } finally { settingsBusy.value = false; }
};
let unlistenPackageLifecycle: UnlistenFn | null = null;

const handlePackageLifecycle = async (event: PackageLifecycleUiEvent) => {
  const packageUsedByCanvas = canvasElements.value.some(
    (element) => String(element.widgetId || "").replace(/^plugin\./, "")
      === event.package_id.replace(/^plugin\./, ""),
  );
  if (!packageUsedByCanvas) return;

  if (event.phase === "awaiting_review") {
    await loadPackageWidgets();
    mainStore.addNotification({
      key: `overlay-package-review-${event.package_id}`,
      title: "Overlay widget paused",
      message: `${event.message} Its saved placement is preserved until you review the package.`,
      type: "warning",
      autoClose: false,
    });
    return;
  }

  if (event.phase === "active" || event.phase === "granted_inactive") {
    await loadPackageWidgets();
    reconcileCanvasPackageWidgets(event.package_id);
    // Re-grant must replace any paused placeholder even when the package's
    // name, code and schema are unchanged.
    if (await v3Sync.syncElementsToBackend(canvasElements.value, {
      width: overlayManager.canvasWidth.value,
      height: overlayManager.canvasHeight.value,
    })) saveCurrentProject();
    return;
  }

  if (event.phase === "unavailable" || event.phase === "uninstalled") {
    await loadPackageWidgets();
    mainStore.addNotification({
      key: `overlay-package-unavailable-${event.package_id}`,
      title: "Overlay widget unavailable",
      message: `${event.message}. Its saved placement remains available for repair or replacement.`,
      type: "warning",
      autoClose: true,
      duration: 7000,
    });
  }
};

const handleKeyDown = (event: KeyboardEvent) => {
  // Don't delete when typing in an input/textarea
  const tag = (event.target as HTMLElement).tagName;
  if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;

  if ((event.key === "Delete" || event.key === "Backspace") && selectedElement.value) {
    event.preventDefault();
    deleteElement();
  }
};

onMounted(async () => {
  unlistenWidgetSettings = await listen('overlay-widget-config-updated', event => acceptWidgetSettings(event.payload));
  unlistenComposition = await listen<CompositionDocument>('vortideck:composition-changed', async event => {
    if (compositionLibrary.busy.value || publishing.value || restoringComposition) return;
    const document = event.payload;
    const previous = compositionLibrary.documents.value[document.id];
    if (previous && previous.revision >= document.revision) return;
    if (currentProject.value?.id === document.id && hasUnsavedChanges.value) {
      compositionLibrary.error.value = 'Another caller changed this draft. Reload and reconcile your unsaved edits.';
      return;
    }
    compositionLibrary.accept(document);
    const project = compositionProject(document) as unknown as OverlayProject;
    const index = projects.value.findIndex(item => item.id === document.id);
    if (index >= 0) projects.value[index] = project;
    else projects.value.push(project);
    if (currentProject.value?.id === document.id) await loadProject(project, false);
  });
  unlistenCompositionReload = await listen('vortideck:composition-reload-required', () => {
    compositionLibrary.error.value = 'Composition updates were missed. Reload before editing.';
  });
  unlistenPackageLifecycle = await listen<PackageLifecycleUiEvent>(
    "package-lifecycle",
    ({ payload }) => void handlePackageLifecycle(payload),
  );
  document.addEventListener("click", handleClickOutside);
  document.addEventListener("contextmenu", handleGlobalContextMenu);
  document.addEventListener("click", handleGlobalClick);
  document.addEventListener("keydown", handleKeyDown);
});

onUnmounted(() => {
  unlistenWidgetSettings?.();
  unlistenComposition?.();
  unlistenCompositionReload?.();
  unlistenPackageLifecycle?.();
  unlistenPackageLifecycle = null;
  widgetForwarder.stop();
  document.removeEventListener("click", handleClickOutside);
  document.removeEventListener("contextmenu", handleGlobalContextMenu);
  document.removeEventListener("click", handleGlobalClick);
  document.removeEventListener("keydown", handleKeyDown);
});

const handleDropElement = (payload: { element: any; x?: number; y?: number }) => {
  if (payload.x !== undefined && payload.y !== undefined) {
    addElement(payload.element, { x: payload.x, y: payload.y });
  } else {
    addElement(payload.element);
  }
};

// Function to add new element
const addElement = async (element: any, position?: { x: number; y: number }) => {
  console.log("Adding element:", element);

  if (element.requiresConsent && element.packageInfo) {
    capabilityDialog.open(element.packageInfo, loadPackageWidgets);
    return;
  }

  // Look up defaults from the registry
  const defaults = getElementDefaults(element.schemaId || element.id);

  const newElement = {
    // Registry defaults first (type, name, width, height, style, content, customHTML, themeClass)
    ...(defaults || {}),
    // Then spread the incoming element (can override name, etc.)
    ...element,
    // ID, position, and state always fresh — must come AFTER spreads
    id: generateProjectId(),
    x: position?.x !== undefined ? Math.max(0, Math.min(position.x, canvasBaseWidth.value - (defaults?.width || element.width || 120))) : Math.random() * (canvasBaseWidth.value - (defaults?.width || 200)),
    y: position?.y !== undefined ? Math.max(0, Math.min(position.y, canvasBaseHeight.value - (defaults?.height || element.height || 80))) : Math.random() * (canvasBaseHeight.value - (defaults?.height || 100)),
    width: defaults?.width || element.width || 120,
    height: defaults?.height || element.height || 80,
    visible: true,
    selected: false,
    // Design-time dimensions — used by Scale content mode to compute transform scale
    designWidth: defaults?.width || element.width || 120,
    designHeight: defaults?.height || element.height || 80,
    scaleContent: false,
    config: JSON.parse(JSON.stringify(element.config || {})),
    configRevision: 0,
    // Seed element.data from schema defaultData so {data.*} placeholders render immediately
    data: { ...(defaults?.defaultData || {}), ...JSON.parse(JSON.stringify(element.data || {})) },
  };


  // Effects (particles, transition, glow) default to full-canvas so they cover the stream.
  // Users can still resize/reposition after adding. Glow is excluded — it's meant to be
  // placed over a specific element, not the whole canvas.
  const schema = getComponentSchema(element.schemaId || element.id);
  if (schema?.category === 'effect' && element.id !== 'glow' && !position) {
    newElement.x = 0;
    newElement.y = 0;
    newElement.width = canvasBaseWidth.value;
    newElement.height = canvasBaseHeight.value;
  }

  canvasElements.value.push(newElement);
  selectElement(newElement);
  markUnsavedChanges();
  try {
    await v3Sync.syncElementAdd(newElement);
    settingsError.value = '';
  } catch (error) {
    newElement.nativeSyncPending = true;
    settingsError.value = `Widget was not added to the output: ${String(error)}`;
  }
  console.log("Element added:", newElement);
};

const removeFavorite = (elementId: string) => {
  favoriteElements.value = favoriteElements.value.filter((fav) => fav.id !== elementId);
};

let scrollTimeout: ReturnType<typeof setTimeout> | null = null;


const restoreHistory = async (direction: 'undo' | 'redo') => {
  if (!currentProject.value) return;
  try {
    const saved = await saveCurrentProject();
    if (!saved) return;
    const document = await compositionLibrary.history(saved.id, direction);
    await loadProject(compositionProject(document) as unknown as OverlayProject, false);
    const index = projects.value.findIndex(project => project.id === document.id);
    if (index >= 0) projects.value[index] = cloneComposition(currentProject.value!);
  } catch (error) { settingsError.value = String(error); }
};
const undo = () => restoreHistory('undo');
const redo = () => restoreHistory('redo');
const reloadComposition = async () => {
  if (!currentProject.value) return;
  try {
    const document = await compositionLibrary.reload(currentProject.value.id);
    await loadProject(compositionProject(document) as unknown as OverlayProject, false);
  } catch (error) { settingsError.value = String(error); }
};

// Get sub-composables from overlay manager
const serverConnection = overlayManager.serverConnection;
const obsIntegration = overlayManager.obsIntegration;
const v3Sync = useOverlayV3Sync({ live: () => false });

// Live preview state

// Initialize overlay systems on mount
onMounted(async () => {
  try {
    await overlayManager.initialize();

    // Establish the saved design before reconciling package contributions.
    // Startup intentionally does not sync from loadProject: third-party grant
    // state must be loaded before executable widget HTML reaches OBS.
    await loadProjectsFromStorage();
    if (projects.value.length === 0) {
      await createProject("Default Overlay");
    } else if (!currentProject.value) {
      const recent = [...projects.value].sort(
        (a, b) => new Date(b.lastModified).getTime() - new Date(a.lastModified).getTime(),
      )[0];
      if (recent) await loadProject(recent, false);
    }

    await loadPackageWidgets();
    const reconciledPackages = reconcileCanvasPackageWidgets();
    migrateSavedBuiltinWidgetMetadata();

    // loadPackageWidgets refreshed the same grant snapshot used by rendering.
    if (reconciledPackages) saveCurrentProject();
    if (canvasElements.value.length > 0) {
      await v3Sync.initialSync(canvasElements.value, {
        width: overlayManager.canvasWidth.value,
        height: overlayManager.canvasHeight.value,
      });
    }

    // Phase 4: begin delivering live stream events to subscribed widget iframes.
    // Draft previews receive rehearsal events only.

    // Immediately refresh server status to get real backend state
    await serverConnection.refreshStatus();

    console.log("✅ Overlay V3 systems initialized");
    console.log("📊 Raw server connection status:", {
      serverRunning: serverConnection.serverRunning.value,
      hasAuthorizedUrl: Boolean(serverConnection.serverUrl.value),
      canvasSize: serverConnection.overlayStatus.value?.canvas_size,
    });
    console.log("📊 Overlay manager state:", {
      running: overlayManager.overlayState.value.serverRunning,
      obsConnected: overlayManager.overlayState.value.obsConnected,
      elementsCount: overlayManager.overlayState.value.elementsCount,
      lastSync: overlayManager.overlayState.value.lastSync,
    });

  } catch (err) {
    console.error("❌ Failed to initialize overlay systems:", err);
  }
});

const syncWithOBS = async () => {
  try {
    console.log("🔄 Syncing with OBS...");

    // Observe agreement only. Global OBS changes require explicit Canvas Settings Apply.
    await obsIntegration.syncWithObs();

    console.log("✅ Canvas size request sent to OBS");
    // The actual canvas size update will come via obs-canvas-size-changed event
  } catch (err) {
    console.error("❌ Failed to sync with OBS:", err);
    // TODO: Show error notification
  }
};

const exportToOBS = async () => {
  if (publishing.value) return;
  publishing.value = true;
  try {
    console.log("🎬 Exporting V3 overlay to OBS...");

    const document = await saveCurrentProject();
    if (!document) throw new Error('Save the draft before publishing');
    const success = await v3Sync.exportToOBS(cloneComposition(document.content.elements), {
      width: Number(document.content.settings.canvasWidth),
      height: Number(document.content.settings.canvasHeight),
    }, { compositionId: document.id, expectedRevision: document.revision });
    if (success) {
      const published = await compositionLibrary.reload(document.id);
      await loadProject(compositionProject(published) as unknown as OverlayProject, false);
    }

    if (success) {
      console.log("✅ Successfully exported V3 overlay to OBS!");
      // TODO: Show success notification
    } else {
      throw new Error("Failed to export V3 overlay to OBS");
    }
  } catch (err) {
    console.error("❌ Failed to export V3 overlay to OBS:", err);
    settingsError.value = String(err);
  } finally { publishing.value = false; }
};

const isTemplateSaved = (templateId: string) => {
  return savedTemplates.value.includes(templateId);
};

// Removed loadThemeScene - using project-based system now

const applyTheme = async (theme: any) => {
  applyThemeFn(theme);
  showThemes.value = false;
  // Store + bake theme vars into the overlay HTML (:root) AND broadcast
  // UPDATE_VARS live. Widgets keep var(--theme-*) references, so the cascade
  // recolors them instantly with no iframe reload.
  markUnsavedChanges();
};

// Apply default theme on mount and setup responsive canvas
onMounted(() => {
  // Apply default theme CSS vars to the canvas DOM
  mountTheme();
  // Canvas sizing and saved-project startup are handled by their owning paths.
});

// Autosave functionality
let autoSaveTimer: ReturnType<typeof setInterval> | null = null;

const startAutoSave = () => {
  if (autoSaveTimer) clearInterval(autoSaveTimer);

  autoSaveTimer = setInterval(() => {
    if (autoSaveEnabled.value && hasUnsavedChanges.value && currentProject.value) {
      saveCurrentProject();
      console.log(`Auto-saved project: ${currentProject.value.name}`);
    }
  }, 30000); // Auto-save every 30 seconds
};

// Watch for changes that should trigger autosave
watch(
  [canvasElements, canvasSize, zoom, currentTheme],
  () => {
    if (restoringComposition) return;
    markUnsavedChanges();

    // Immediate autosave on any change (like scenes do) - but not during drag or resize
    if (
      autoSaveEnabled.value &&
      currentProject.value &&
      !isDragging.value &&
      !isElementResizing.value
    ) {
      saveCurrentProject();
      console.log(`Auto-saved project immediately: ${currentProject.value.name}`);
    }
  },
  { deep: true }
);

// Start autosave on mount
onMounted(() => {
  startAutoSave();
});

// Canvas sizing is now handled by OverlayCanvas component

// Cleanup on unmount
onUnmounted(() => {
  if (autoSaveTimer) {
    clearInterval(autoSaveTimer);
  }
});

const previewTheme = (theme: any) => {
  console.log("Previewing theme:", theme.name);
  // Show preview of the theme without applying it permanently
  const canvas = document.querySelector(".canvas-container");
  if (canvas) {
    // Store current styles to restore later
    const originalStyles = {};
    const tempCssVars = {
      // Colors
      "--theme-primary": theme.colors[0],
      "--theme-secondary": theme.colors[1],
      "--theme-accent": theme.colors[2],
      "--theme-bg": theme.colors[3] || "#000000",
      "--theme-text": theme.colors[4] || "#ffffff",
      "--theme-surface": theme.colors[5] || theme.colors[3] || "#111111",

      // Component styling
      "--theme-card-bg": theme.colors[5] || theme.colors[3] || "#111111",
      "--theme-button-bg": theme.colors[0],
      "--theme-button-text": theme.colors[4] || "#ffffff",
      "--theme-border": theme.colors[0],
    };

    // Apply preview styles
    Object.entries(tempCssVars).forEach(([key, value]) => {
      (canvas as HTMLElement).style.setProperty(key, value);
    });

    // Auto-remove preview after 3 seconds
    setTimeout(() => {
      if (currentTheme.value && currentTheme.value.cssVars) {
        Object.entries(currentTheme.value.cssVars).forEach(([key, value]) => {
          (canvas as HTMLElement).style.setProperty(key, value);
        });
      }
    }, 3000);
  }
};

const handleTabScroll = (event: WheelEvent) => {
  if (tabScrollContainer.value) {
    event.preventDefault();
    const scrollAmount = event.deltaY * 0.6;

    // Clear any existing timeout
    if (scrollTimeout) {
      clearTimeout(scrollTimeout);
    }

    // For fast scrolling, use immediate response
    if (Math.abs(event.deltaY) > 50) {
      tabScrollContainer.value.scrollLeft += scrollAmount;
    } else {
      // For slow/precise scrolling, use smooth behavior
      tabScrollContainer.value.scrollBy({
        left: scrollAmount,
        behavior: "smooth",
      });
    }

    // Reset after scrolling stops
    scrollTimeout = setTimeout(() => {
      scrollTimeout = null;
    }, 100);
  }
};

const startPanelResize = (event: MouseEvent) => {
  isResizing.value = true;
  const startX = event.clientX;
  const startWidth = leftPanelWidth.value;

  const handleMouseMove = (e: MouseEvent) => {
    if (!isResizing.value) return;

    const deltaX = e.clientX - startX;
    const newWidth = startWidth + deltaX;

    // Constrain width between 200px and 500px
    if (newWidth >= 200 && newWidth <= 500) {
      leftPanelWidth.value = newWidth;
    }
  };

  const handleMouseUp = () => {
    isResizing.value = false;
    document.removeEventListener("mousemove", handleMouseMove);
    document.removeEventListener("mouseup", handleMouseUp);
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  };

  document.addEventListener("mousemove", handleMouseMove);
  document.addEventListener("mouseup", handleMouseUp);
  document.body.style.cursor = "col-resize";
  document.body.style.userSelect = "none";

  event.preventDefault();
};

// Interactive canvas functions
const selectElement = (element: any) => {
  console.log("Selecting element:", element.name);
  selectedElement.value = element;
  showContextMenu.value = false;
};

const startDrag = (event: MouseEvent, element: any) => {
  if (event.button !== 0) return; // Only left click

  console.log("Starting drag for element:", element.name);
  event.preventDefault();
  event.stopPropagation();

  // Cache the DOM element FIRST and disable transitions immediately
  let target = event.target as HTMLElement;
  while (target && !target.classList.contains("element-interactive")) {
    target = target.parentElement as HTMLElement;
  }
  dragElementDOM = target;

  // No need to disable transitions - they don't affect position/size anymore

  selectElement(element);
  isDragging.value = true;
  dragElement.value = element;

  // Use pointer capture for consistent tracking (Canva technique)
  if (dragElementDOM && (event as any).pointerId) {
    dragElementDOM.setPointerCapture((event as any).pointerId);
  }

  // Cache canvas container and rect for the drag operation
  if (!cachedCanvasContainer) {
    cachedCanvasContainer = document.querySelector(".canvas-container");
  }
  if (!cachedCanvasContainer) {
    console.error("Canvas container not found");
    isDragging.value = false;
    return;
  }

  cachedCanvasRect = cachedCanvasContainer.getBoundingClientRect();

  // Calculate the scale factor (same as updateDragPosition)
  const canvasDisplayScale = canvasScale.value;
  const zoomScale = zoom.value / 100;
  const totalScale = canvasDisplayScale * zoomScale;

  // Calculate mouse position relative to canvas container (get fresh rect)
  const canvasRect = cachedCanvasContainer.getBoundingClientRect();
  const mouseXInCanvas = event.clientX - canvasRect.left;
  const mouseYInCanvas = event.clientY - canvasRect.top;

  // Convert mouse position to logical coordinates
  const mouseXUnscaled = mouseXInCanvas / totalScale;
  const mouseYUnscaled = mouseYInCanvas / totalScale;

  // Simple drag offset - mouse position relative to element's logical position
  dragOffset.value = {
    x: mouseXUnscaled - element.x,
    y: mouseYUnscaled - element.y,
  };

  // Add window-level event listeners for drag (like overlay.vue)
  window.addEventListener("mousemove", updateDragPosition);
  window.addEventListener("mouseup", onMouseUp);

  // Prevent text selection during drag
  document.body.style.cursor = "grabbing";
  document.body.style.userSelect = "none";

  // Disable pointer events on all element overlays to prevent flicker (like overlay.vue)
  const allOverlays = document.querySelectorAll<HTMLElement>("[data-element-overlay]");
  allOverlays.forEach((overlay) => {
    overlay.style.pointerEvents = "none";
  });
};

// Cache for performance
let cachedCanvasContainer: HTMLElement | null = null;
let cachedCanvasRect: DOMRect | null = null;
let dragElementDOM: HTMLElement | null = null;
let resizeElementDOM: HTMLElement | null = null;

// Direct real-time updates for immediate responsiveness

const updateDragPosition = (event: MouseEvent) => {
  if (!isDragging.value || !dragElement.value || !cachedCanvasRect || !dragElementDOM)
    return;

  // Calculate mouse position relative to canvas
  // Get fresh canvas rect for accurate coordinates
  if (!cachedCanvasContainer) return;
  const canvasRect = cachedCanvasContainer.getBoundingClientRect();
  const mouseXInCanvas = event.clientX - canvasRect.left;
  const mouseYInCanvas = event.clientY - canvasRect.top;

  // Convert visual coordinates back to logical coordinates
  // The canvas content is scaled, so we need to unscale mouse coordinates
  const canvasDisplayScale = canvasScale.value;
  const zoomScale = zoom.value / 100;
  const totalScale = canvasDisplayScale * zoomScale;

  const actualX = Math.round(mouseXInCanvas / totalScale - dragOffset.value.x);
  const actualY = Math.round(mouseYInCanvas / totalScale - dragOffset.value.y);

  const actualCanvasWidth = overlayManager?.canvasWidth?.value || canvasWidth.value;
  const actualCanvasHeight = overlayManager?.canvasHeight?.value || canvasHeight.value;

  // Hard limits so elements don't get lost in the void (1 full screen width outside)
  const minX = -actualCanvasWidth;
  const maxX = actualCanvasWidth * 2;
  const minY = -actualCanvasHeight;
  const maxY = actualCanvasHeight * 2;

  let constrainedX = Math.max(minX, Math.min(actualX, maxX));
  let constrainedY = Math.max(minY, Math.min(actualY, maxY));

  // Magnetic Snapping (8 physical pixels of "barrier" pull, less aggressive)
  const SNAP_THRESHOLD = 8 / totalScale;

  // Snap left/right edges
  if (Math.abs(constrainedX - 0) < SNAP_THRESHOLD) {
    constrainedX = 0;
  } else if (Math.abs((constrainedX + dragElement.value.width) - actualCanvasWidth) < SNAP_THRESHOLD) {
    constrainedX = actualCanvasWidth - dragElement.value.width;
  }

  // Snap top/bottom edges
  if (Math.abs(constrainedY - 0) < SNAP_THRESHOLD) {
    constrainedY = 0;
  } else if (Math.abs((constrainedY + dragElement.value.height) - actualCanvasHeight) < SNAP_THRESHOLD) {
    constrainedY = actualCanvasHeight - dragElement.value.height;
  }

  // Direct DOM manipulation for instant visual feedback (no scale needed since elements are in scaled container)
  dragElementDOM.style.left = constrainedX + "px";
  dragElementDOM.style.top = constrainedY + "px";

  // Update reactive data (will be applied when drag ends)
  dragElement.value.x = constrainedX;
  dragElement.value.y = constrainedY;

  // Broadcast position to overlay clients (throttled 50ms)
  if (!dragPositionThrottled) {
    dragPositionThrottled = true;
    v3Sync.syncElementPosition(dragElement.value.id, constrainedX, constrainedY);
    setTimeout(() => { dragPositionThrottled = false; }, 100);
  }
};

let dragPositionThrottled = false;

// Direct real-time mouse move handler
const onMouseMove = updateDragPosition;

const onMouseUp = (event: MouseEvent) => {
  if (!isDragging.value) return;

  // Capture element before clearing drag state
  const droppedElement = dragElement.value;

  // Remove window-level event listeners (like overlay.vue)
  window.removeEventListener("mousemove", updateDragPosition);
  window.removeEventListener("mouseup", onMouseUp);

  // Restore cursor and selection
  document.body.style.cursor = "";
  document.body.style.userSelect = "";

  // Re-enable pointer events on all element overlays (like overlay.vue)
  const allOverlays = document.querySelectorAll<HTMLElement>("[data-element-overlay]");
  allOverlays.forEach((overlay) => {
    overlay.style.pointerEvents = "";
  });

  // Persist final position to backend (survives page refresh)
  if (droppedElement) {
    v3Sync.syncElementUpdate(droppedElement);
  }

  // Clean up drag state
  isDragging.value = false;
  dragElement.value = null;
  dragOffset.value = { x: 0, y: 0 };

  // Clear cache for next drag operation
  cachedCanvasContainer = null;
  cachedCanvasRect = null;
  dragElementDOM = null;

  // Mark unsaved changes and trigger autosave after drop is complete
  markUnsavedChanges();
  if (autoSaveEnabled.value && currentProject.value) {
    saveCurrentProject();
    console.log(`Auto-saved project after element drop: ${currentProject.value.name}`);
  }
};

const showContextMenuHandler = (event: MouseEvent, element: any) => {
  event.preventDefault();
  event.stopPropagation();

  // Calculate context menu position with edge detection
  const menuWidth = 200; // Approximate menu width
  const menuHeight = 250; // Approximate menu height
  let x = event.clientX;
  let y = event.clientY;

  // Adjust position if menu would go off screen
  if (x + menuWidth > window.innerWidth) {
    x = window.innerWidth - menuWidth - 10;
  }
  if (y + menuHeight > window.innerHeight) {
    y = window.innerHeight - menuHeight - 10;
  }

  // If context menu is already open, close it first then immediately reopen for new element
  if (showContextMenu.value) {
    showContextMenu.value = false;
    // Use nextTick to ensure the menu closes before reopening
    nextTick(() => {
      selectElement(element);
      contextMenuTarget.value = element;
      contextMenuPosition.value = { x, y };
      showContextMenu.value = true;
    });
  } else {
    selectElement(element);
    contextMenuTarget.value = element;
    contextMenuPosition.value = { x, y };
    showContextMenu.value = true;
  }
};

const hideContextMenu = () => {
  showContextMenu.value = false;
  contextMenuTarget.value = null;
};

// Global context menu handler - only for overlay canvas elements
const handleGlobalContextMenu = (event: MouseEvent) => {
  // Don't interfere if right-clicking on the context menu itself
  if ((event.target as Element)?.closest(".absolute.bg-white.rounded-lg")) {
    return;
  }

  // Only handle context menu for canvas elements - check if we're in the canvas area
  const canvasContainer = (event.target as Element)?.closest(".canvas-container");
  if (!canvasContainer) {
    // Not in canvas area - let browser handle default context menu
    return;
  }

  event.preventDefault();
  console.log("Canvas right-click detected at:", event.clientX, event.clientY);

  // Find if we're right-clicking on an element-interactive
  let targetElement: Element | null = event.target as Element;
  let canvasElement: CanvasElement | undefined;

  // Traverse up to find an element that has element data
  while (targetElement && targetElement !== document.body) {
    if (targetElement.classList?.contains("element-interactive")) {
      // Found a canvas element - extract the element data
      const elementId = targetElement.getAttribute("data-element-id");
      console.log("Found element-interactive with id:", elementId);
      if (elementId) {
        canvasElement = canvasElements.value.find((el) => el.id === elementId);
        console.log("Found canvas element:", canvasElement?.name);
      }
      break;
    }
    targetElement = targetElement.parentElement;
  }

  // If we found a canvas element, show/switch context menu
  if (canvasElement) {
    console.log("Showing context menu for element:", canvasElement.name);
    showContextMenuHandler(event, canvasElement);
  } else {
    console.log("No canvas element found - closing menu");
    hideContextMenu();
  }
};

// Global click handler to close context menu
const handleGlobalClick = (event: MouseEvent) => {
  // Don't close if clicking on the context menu itself
  if ((event.target as Element)?.closest(".absolute.bg-white.rounded-lg")) {
    return;
  }

  // Close context menu on any other click
  if (showContextMenu.value) {
    hideContextMenu();
  }
};

// Context menu actions
const duplicateElement = () => {
  const target = contextMenuTarget.value || selectedElement.value;
  if (!target) return;

  const original = target;
  const duplicate = {
    ...original,
    id: `${original.id}_${Date.now()}`,
    name: `${original.name} Copy`,
    x: original.x + 20,
    y: original.y + 20,
    style: { ...original.style },
  };

  canvasElements.value.push(duplicate);
  v3Sync.syncElementAdd(duplicate).catch(error => { settingsError.value = String(error); });
  selectElement(duplicate);
  hideContextMenu();
};

// Reassign compact, NON-NEGATIVE z-indices (0..N-1) from a desired stack order,
// then push the change to the overlay.
//
// Why non-negative: the editor canvas has an opaque "Game Capture" simulation
// layer; a negative z-index paints beneath it and the element vanishes (this is
// what made send-to-back — and the full-canvas fireball — disappear).
//
// Why z-index and not array/DOM order: moving an iframe's DOM node reloads it
// (losing chat history). z-index edits the wrapper's style in place — zero
// reloads, in both editor and overlay.
//
// Why compact: re-deriving 0..N-1 each time avoids the unbounded growth of a
// "max+1 forever" counter. Not limiting — the range just tracks element count.
const applyZOrder = (reordered: any[]) => {
  reordered.forEach((el, i) => { el.zIndex = i; });
  // Draft stacking stays local until publication.
  // 2) Persist: z-index now lives on the outer .overlay-element div (Rust
  //    z_index field), rendered by the backend. Re-syncing regenerates the
  //    overlay HTML and broadcasts once; ids are preserved and iframe srcdocs
  //    are unchanged, so the overlay diffs in place — no reload. Keeps the
  //    order correct across reloads / OBS F5.
  v3Sync.syncElementsToBackend(canvasElements.value, {
    width: overlayManager.canvasWidth.value,
    height: overlayManager.canvasHeight.value,
  });
};

const bringToFront = () => {
  const target = contextMenuTarget.value || selectedElement.value;
  if (!target) return;
  // Current visual order = ascending z-index (stable by array position), then
  // move the target to the top of the stack.
  const ordered = [...canvasElements.value].sort(
    (a, b) => (a.zIndex || 0) - (b.zIndex || 0),
  );
  applyZOrder([...ordered.filter((el) => el.id !== target.id), target]);
  hideContextMenu();
};

const sendToBack = () => {
  const target = contextMenuTarget.value || selectedElement.value;
  if (!target) return;
  const ordered = [...canvasElements.value].sort(
    (a, b) => (a.zIndex || 0) - (b.zIndex || 0),
  );
  applyZOrder([target, ...ordered.filter((el) => el.id !== target.id)]);
  hideContextMenu();
};

// One-step ordering: move the target a single position up/down the stack.
// (bringToFront/sendToBack above move it all the way to the extreme.)
const bringForward = () => {
  const target = contextMenuTarget.value || selectedElement.value;
  if (!target) return;
  const ordered = [...canvasElements.value].sort(
    (a, b) => (a.zIndex || 0) - (b.zIndex || 0),
  );
  const i = ordered.findIndex((el) => el.id === target.id);
  if (i < 0 || i >= ordered.length - 1) { hideContextMenu(); return; } // already at front
  const reordered = [...ordered];
  const current = reordered[i]!;
  reordered[i] = reordered[i + 1]!;
  reordered[i + 1] = current;
  applyZOrder(reordered);
  hideContextMenu();
};

const sendBackward = () => {
  const target = contextMenuTarget.value || selectedElement.value;
  if (!target) return;
  const ordered = [...canvasElements.value].sort(
    (a, b) => (a.zIndex || 0) - (b.zIndex || 0),
  );
  const i = ordered.findIndex((el) => el.id === target.id);
  if (i <= 0) { hideContextMenu(); return; } // already at back
  const reordered = [...ordered];
  const current = reordered[i]!;
  reordered[i] = reordered[i - 1]!;
  reordered[i - 1] = current;
  applyZOrder(reordered);
  hideContextMenu();
};

const resetElementStyle = () => {
  const target = contextMenuTarget.value || selectedElement.value;
  if (!target) return;

  const element = target;
  // Reset to default style based on type
  if (element.type === "webcam") {
    element.style = {
      borderRadius: "8px",
      borderWidth: "2px",
      borderColor: "#ffffff",
      backgroundColor: "var(--theme-card-bg)",
      borderStyle: "solid",
    };
  } else if (element.type === "chat") {
    element.style = {
      borderRadius: "8px",
      borderWidth: "1px",
      borderColor: "var(--theme-border)",
      backgroundColor: "var(--theme-card-bg)",
      borderStyle: "solid",
    };
  } else if (element.type === "counter") {
    element.style = {
      borderRadius: "8px",
      backgroundColor: "var(--theme-button-bg)",
      color: "var(--theme-button-text)",
      borderStyle: "none",
    };
  }
  hideContextMenu();
};

const applyThemeToElement = () => {
  if (!contextMenuTarget.value || !currentTheme.value) return;

  const element = contextMenuTarget.value;
  // Apply current theme colors to element
  element.style = {
    ...element.style,
    borderColor: currentTheme.value.colors[0] ?? "#ffffff",
    backgroundColor: currentTheme.value.colors[3] ?? "#000000",
    color: currentTheme.value.colors[4] ?? "#ffffff",
  };
  hideContextMenu();
};

const deleteElement = async () => {
  const target = contextMenuTarget.value || selectedElement.value;
  if (!target) return;

  const index = canvasElements.value.findIndex((el) => el.id === target.id);
  if (index > -1) {
    canvasElements.value.splice(index, 1);
    v3Sync.syncElementRemove(target.id);
    markUnsavedChanges();
  }

  if (selectedElement.value?.id === target.id) {
    selectedElement.value = null;
  }
  hideContextMenu();
};

// Element resize functionality
const startResize = (event: MouseEvent, element: any, handle: string) => {
  console.log("Starting resize for element:", element.name, "handle:", handle);
  event.preventDefault();
  event.stopPropagation();

  // Cache the DOM element FIRST and disable transitions immediately
  let target = event.target as HTMLElement;
  while (target && !target.classList.contains("element-interactive")) {
    target = target.parentElement as HTMLElement;
  }
  resizeElementDOM = target;

  // No need to disable transitions anymore

  selectElement(element);
  isElementResizing.value = true;
  resizeElement.value = element;
  resizeHandle.value = handle;

  const startX = event.clientX;
  const startY = event.clientY;
  const startWidth = element.width;
  const startHeight = element.height;
  const startLeft = element.x;
  const startTop = element.y;

  const updateResizePosition = (e: MouseEvent) => {
    // Scale mouse deltas by total scale (like drag calculations)
    const totalScale = canvasScale.value * (zoom.value / 100);
    const deltaX = (e.clientX - startX) / totalScale;
    const deltaY = (e.clientY - startY) / totalScale;

    let newWidth = startWidth;
    let newHeight = startHeight;
    let newX = startLeft;
    let newY = startTop;

    // Calculate new dimensions based on resize handle
    switch (handle) {
      case "nw":
        newWidth = startWidth - deltaX;
        newHeight = startHeight - deltaY;
        newX = startLeft + deltaX;
        newY = startTop + deltaY;
        break;
      case "n":
        newHeight = startHeight - deltaY;
        newY = startTop + deltaY;
        break;
      case "ne":
        newWidth = startWidth + deltaX;
        newHeight = startHeight - deltaY;
        newY = startTop + deltaY;
        break;
      case "w":
        newWidth = startWidth - deltaX;
        newX = startLeft + deltaX;
        break;
      case "e":
        newWidth = startWidth + deltaX;
        break;
      case "sw":
        newWidth = startWidth - deltaX;
        newHeight = startHeight + deltaY;
        newX = startLeft + deltaX;
        break;
      case "s":
        newHeight = startHeight + deltaY;
        break;
      case "se":
        newWidth = startWidth + deltaX;
        newHeight = startHeight + deltaY;
        break;
    }

    // Enforce minimum size
    newWidth = Math.max(50, newWidth);
    newHeight = Math.max(30, newHeight);

    // Aspect-ratio lock when Scale content is enabled.
    // Each handle has a PRIMARY axis — the dragged edge tracks the mouse exactly.
    // The other dimension and position are derived from the design ratio.
    if (element.scaleContent && element.designWidth && element.designHeight) {
      const ratio = element.designWidth / element.designHeight; // e.g. 4.3 for wide card

      if (handle === 'e' || handle === 'ne' || handle === 'se') {
        // Right edge is primary — X drives width, right edge tracks mouse
        newWidth  = Math.max(50, startWidth + deltaX);
        newHeight = Math.round(newWidth / ratio);
        newX      = startLeft;                                    // left edge stays
        newY      = handle === 'ne'
          ? startTop + startHeight - newHeight                    // bottom stays (ne)
          : startTop;                                             // top stays (se)
      } else if (handle === 'w' || handle === 'nw' || handle === 'sw') {
        // Left edge is primary — -X drives width, left edge tracks mouse
        newWidth  = Math.max(50, startWidth - deltaX);
        newHeight = Math.round(newWidth / ratio);
        newX      = startLeft + startWidth - newWidth;            // right edge stays
        newY      = handle === 'nw'
          ? startTop + startHeight - newHeight                    // bottom stays (nw)
          : startTop;                                             // top stays (sw)
      } else if (handle === 'n') {
        // Top edge primary — -Y drives height, bottom stays
        newHeight = Math.max(30, startHeight - deltaY);
        newWidth  = Math.round(newHeight * ratio);
        newX      = startLeft;
        newY      = startTop + startHeight - newHeight;
      } else if (handle === 's') {
        // Bottom edge primary — Y drives height, top stays
        newHeight = Math.max(30, startHeight + deltaY);
        newWidth  = Math.round(newHeight * ratio);
        newX      = startLeft;
        newY      = startTop;
      }
    }

    // Constrain to logical canvas dimensions (2560x1440, not visual scaled size)
    const actualCanvasWidth = overlayManager?.canvasWidth?.value || canvasWidth.value;
    const actualCanvasHeight = overlayManager?.canvasHeight?.value || canvasHeight.value;

    // Hard limits so elements don't get lost in the void (1 full screen width outside)
    const minX = -actualCanvasWidth;
    const maxX = actualCanvasWidth * 2;
    const minY = -actualCanvasHeight;
    const maxY = actualCanvasHeight * 2;

    newX = Math.max(minX, Math.min(newX, maxX));
    newY = Math.max(minY, Math.min(newY, maxY));

    // Magnetic Snapping for Resize (8 physical pixels of "barrier" pull, less aggressive)
    const SNAP_THRESHOLD = 8 / totalScale;

    // Snap left edge
    if (handle.includes('w')) {
      if (Math.abs(newX - 0) < SNAP_THRESHOLD) {
        const diff = 0 - newX;
        newX = 0;
        newWidth -= diff;
      }
    }
    // Snap right edge
    if (handle.includes('e')) {
      const rightEdge = newX + newWidth;
      if (Math.abs(rightEdge - actualCanvasWidth) < SNAP_THRESHOLD) {
        newWidth = actualCanvasWidth - newX;
      }
    }
    // Snap top edge
    if (handle.includes('n')) {
      if (Math.abs(newY - 0) < SNAP_THRESHOLD) {
        const diff = 0 - newY;
        newY = 0;
        newHeight -= diff;
      }
    }
    // Snap bottom edge
    if (handle.includes('s')) {
      const bottomEdge = newY + newHeight;
      if (Math.abs(bottomEdge - actualCanvasHeight) < SNAP_THRESHOLD) {
        newHeight = actualCanvasHeight - newY;
      }
    }

    // Direct DOM manipulation for instant visual feedback (no scale needed since elements are in scaled container)
    if (resizeElementDOM) {
      resizeElementDOM.style.width = newWidth + "px";
      resizeElementDOM.style.height = newHeight + "px";
      resizeElementDOM.style.left = newX + "px";
      resizeElementDOM.style.top = newY + "px";
    }

    // Update reactive data
    element.width = newWidth;
    element.height = newHeight;
    element.x = newX;
    element.y = newY;
  };

  // Direct real-time resize handler
  const handleMouseMove = updateResizePosition;

  const handleMouseUp = () => {
    // No transition cleanup needed anymore

    document.removeEventListener("mousemove", handleMouseMove);
    document.removeEventListener("mouseup", handleMouseUp);
    document.body.style.cursor = "";
    document.body.style.userSelect = "";

    // Persist resized element to backend
    if (resizeElement.value) {
      v3Sync.syncElementUpdate(resizeElement.value);
    }

    isElementResizing.value = false;
    resizeElement.value = null;
    resizeHandle.value = "";
    resizeElementDOM = null;

    // Mark unsaved changes and trigger autosave after resize is complete
    markUnsavedChanges();
    if (autoSaveEnabled.value && currentProject.value) {
      saveCurrentProject();
      console.log(
        `Auto-saved project after element resize: ${currentProject.value.name}`
      );
    }
  };

  document.addEventListener("mousemove", handleMouseMove);
  document.addEventListener("mouseup", handleMouseUp);
  document.body.style.cursor = window.getComputedStyle(event.target as Element).cursor;
  document.body.style.userSelect = "none";
};

// Click outside to deselect
const handleCanvasClick = (event: MouseEvent) => {
  if ((event.target as HTMLElement).classList.contains("canvas-container")) {
    selectedElement.value = null;
    hideContextMenu();
  }
};

// Missing utility functions
const generateProjectId = () => {
  return `project_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
};

const markUnsavedChanges = () => {
  hasUnsavedChanges.value = true;
};

const saveProjectsToStorage = async () => {
  try {
    for (const project of projects.value) await compositionLibrary.save(project as unknown as CompositionProject);
  } catch (error) { settingsError.value = String(error); }
};

const loadProjectsFromStorage = async () => {
  const raw = localStorage.getItem('overlayProjects');
  const legacy: OverlayProject[] = raw ? JSON.parse(raw) : [];
  legacy.forEach(project => {
    project.settings.canvasWidth ??= overlayManager.canvasWidth.value;
    project.settings.canvasHeight ??= overlayManager.canvasHeight.value;
  });
  projects.value = await compositionLibrary.load(legacy as unknown as CompositionProject[]) as unknown as OverlayProject[];
  if (raw) {
    localStorage.setItem('overlayProjectsPreNative', raw);
    localStorage.removeItem('overlayProjects');
  }
};

const createProject = async (name?: string) => {
  const newProject: OverlayProject = {
    id: generateProjectId(), name: name || 'Untitled Overlay',
    created: new Date().toISOString(), lastModified: new Date().toISOString(),
    elements: cloneComposition(canvasElements.value),
    settings: { canvasSize: canvasSize.value, canvasWidth: overlayManager.canvasWidth.value,
      canvasHeight: overlayManager.canvasHeight.value, zoom: zoom.value, theme: cloneComposition(currentTheme.value) },
  };
  try {
    const document = await compositionLibrary.save(newProject as unknown as CompositionProject);
    projects.value.push(newProject);
    currentProject.value = newProject;
    hasUnsavedChanges.value = false;
    lastSaved.value = document.lastModified;
  } catch (error) { settingsError.value = String(error); }
};

const saveCurrentProject = async () => {
  if (!currentProject.value) return null;
  const project: OverlayProject = {
    ...cloneComposition(currentProject.value), lastModified: new Date().toISOString(),
    elements: cloneComposition(canvasElements.value),
    settings: { canvasSize: canvasSize.value, canvasWidth: overlayManager.canvasWidth.value,
      canvasHeight: overlayManager.canvasHeight.value, zoom: zoom.value, theme: cloneComposition(currentTheme.value) },
  };
  try {
    const document = await compositionLibrary.save(project as unknown as CompositionProject);
    if (currentProject.value?.id === project.id) {
      currentProject.value = project;
      const index = projects.value.findIndex(item => item.id === project.id);
      if (index >= 0) projects.value[index] = cloneComposition(project);
      hasUnsavedChanges.value = false;
      lastSaved.value = document.lastModified;
    }
    return document;
  } catch (error) { settingsError.value = String(error); return null; }
};

// Enhanced property update functions with override validation
const updateElementProperty = (property: string, value: any) => {
  if (!selectedElement.value) return;

  const element = selectedElement.value;

  // Check if this element is from a component template with restrictions
  if (element.componentTemplate) {
    // Check content override permission
    const contentProperties = ["content", "text", "value", "name"];
    if (
      contentProperties.includes(property) &&
      !element.instanceOverrides?.allowContentOverride
    ) {
      console.warn(
        `Property '${property}' cannot be modified - content override disabled for this component element`
      );
      return;
    }

    // Check position override permission
    const positionProperties = ["x", "y", "width", "height"];
    if (
      positionProperties.includes(property) &&
      !element.instanceOverrides?.allowPositionOverride
    ) {
      console.warn(
        `Property '${property}' cannot be modified - position override disabled for this component element`
      );
      return;
    }
  }

  // Apply the property change
  element[property] = value;

  // Store instance override for component elements
  if (element.componentTemplate) {
    if (!element.instanceValues) {
      element.instanceValues = {};
    }
    element.instanceValues[property] = value;
  }

  markUnsavedChanges();
  v3Sync.syncElementUpdate(element);
};

const updateElementStyle = (property: string, value: any) => {
  if (!selectedElement.value) return;

  const element = selectedElement.value;

  // Check if this element is from a component template with style restrictions
  if (element.componentTemplate && !element.instanceOverrides?.allowStyleOverride) {
    console.warn(
      `Style property '${property}' cannot be modified - style override disabled for this component element`
    );
    return;
  }

  if (!element.style) {
    element.style = {};
  }
  element.style[property] = value;

  // Store instance style override for component elements
  if (element.componentTemplate) {
    if (!element.instanceStyleOverrides) {
      element.instanceStyleOverrides = {};
    }
    element.instanceStyleOverrides[property] = value;
  }

  markUnsavedChanges();
  v3Sync.syncElementUpdate(element);
};

/**
 * Update a design-time data value on the selected element.
 * Writes to element.data[key] — substituted into {data.key} template placeholders.
 */
const updateElementData = (key: string, value: string | number) => {
  if (!selectedElement.value) return;
  const element = selectedElement.value;
  if (!element.data) element.data = {};
  element.data[key] = value;
  markUnsavedChanges();
  v3Sync.syncElementUpdate(element);
};

const addLayer = (layer: any) => {
  console.log("Adding layer:", layer);
};

const selectLayer = (layerId: string) => {
  const element = canvasElements.value.find((el) => el.id === layerId);
  if (element) {
    selectElement(element);
  }
};

const toggleLayerVisibility = (layerId: string) => {
  const element = canvasElements.value.find((el) => el.id === layerId);
  if (element) {
    element.visible = !element.visible;
    markUnsavedChanges();
    v3Sync.syncElementUpdate(element);
  }
};

// Missing computed for context menu
const contextMenuItems = computed(() => [
  { id: "duplicate", label: "Duplicate", action: "duplicate", icon: IconDuplicate },
  {
    id: "bring-to-front",
    label: "Bring to Front",
    action: "bring-to-front",
    icon: IconArrowUp,
  },
  {
    id: "bring-forward",
    label: "Bring Forward",
    action: "bring-forward",
    icon: IconArrowUp,
  },
  {
    id: "send-backward",
    label: "Send Backward",
    action: "send-backward",
    icon: IconArrowDown,
  },
  {
    id: "send-to-back",
    label: "Send to Back",
    action: "send-to-back",
    icon: IconArrowDown,
  },
  { id: "reset-style", label: "Reset Style", action: "reset-style", icon: IconRefresh },
  {
    id: "apply-theme",
    label: "Apply Theme",
    action: "apply-theme",
    icon: IconPaintBrush,
  },
  { id: "delete", label: "Delete", action: "delete", icon: IconTrash, danger: true },
]);

const handleContextMenuAction = (action: string) => {
  switch (action) {
    case "duplicate":
      duplicateElement();
      break;
    case "bring-to-front":
      bringToFront();
      break;
    case "bring-forward":
      bringForward();
      break;
    case "send-backward":
      sendBackward();
      break;
    case "send-to-back":
      sendToBack();
      break;
    case "reset-style":
      resetElementStyle();
      break;
    case "apply-theme":
      applyThemeToElement();
      break;
    case "delete":
      deleteElement();
      break;
    default:
      console.log("Unknown action:", action);
  }
};
</script>

<style scoped>
input[type="range"] {
  @apply appearance-none h-1.5 bg-neutral-200 rounded-full outline-none;
}

input[type="range"]::-webkit-slider-thumb {
  @apply appearance-none w-4 h-4 bg-blue-500 rounded-full cursor-pointer hover:bg-blue-600;
}

input[type="range"]::-moz-range-thumb {
  @apply w-4 h-4 bg-blue-500 rounded-full cursor-pointer border-0 hover:bg-blue-600;
}

.tab-scroll {
  scrollbar-width: none;
  -ms-overflow-style: none;
  position: relative;
}

.tab-scroll::-webkit-scrollbar {
  display: none;
}

.tab-scroll::before,
.tab-scroll::after {
  content: "";
  position: absolute;
  top: 0;
  bottom: 0;
  width: 8px;
  pointer-events: none;
  z-index: 10;
  transition: opacity 0.2s ease;
}

.tab-scroll::before {
  left: 0;
  background: linear-gradient(to right, rgba(255, 255, 255, 0.8), transparent);
}

.tab-scroll::after {
  right: 0;
  background: linear-gradient(to left, rgba(255, 255, 255, 0.8), transparent);
}

.tab-scroll:hover {
  cursor: grab;
}

.tab-scroll:active {
  cursor: grabbing;
}

/* Resize handle styles */
/* Panel resize handle with gradient */
.panel-resize-handle {
  transition: background-color 0.15s ease;
}

.panel-resize-handle::after {
  content: "";
  position: absolute;
  top: 10%;
  right: 1px;
  width: 2px;
  height: 80%;
  background: linear-gradient(to bottom, transparent, #a855f7, transparent);
  border-radius: 2px;
  opacity: 0;
  transition: opacity 0.2s ease;
}

.panel-resize-handle:hover::after {
  opacity: 0.6;
}

/* Element resize handles (blue dots) */
.resize-handle {
  transition: background-color 0.15s ease;
}

/* Smooth panel transitions - only when not resizing */
.left-panel {
  transition: width 0.1s ease-out;
}

.left-panel.resizing {
  transition: none !important;
}

.modal-enter-active,
.modal-leave-active {
  transition: all 0.3s ease;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
  transform: scale(0.9);
}

.modal-enter-to,
.modal-leave-from {
  opacity: 1;
  transform: scale(1);
}

.expand-enter-active,
.expand-leave-active {
  transition: all 0.3s ease;
  overflow: hidden;
}

.expand-enter-from,
.expand-leave-to {
  opacity: 0;
  max-height: 0;
}

.expand-enter-to,
.expand-leave-from {
  opacity: 1;
  max-height: 500px;
}

/* Responsive canvas wrapper that maintains aspect ratio */
.responsive-canvas-wrapper {
  width: 100%;
  max-width: calc(100vw - 650px); /* Account for left/right panels */
  max-height: calc(100vh - 200px); /* Account for header */
  display: flex;
  align-items: center;
  justify-content: center;
}

/* Responsive breakpoints */
@media (max-width: 1400px) {
  .responsive-canvas-wrapper {
    max-width: calc(100vw - 580px);
  }
}

@media (max-width: 1200px) {
  .responsive-canvas-wrapper {
    max-width: calc(100vw - 500px);
  }
}

@media (max-width: 1024px) {
  .responsive-canvas-wrapper {
    max-width: calc(100vw - 400px);
  }
}

/* Theme-aware components using CSS variables */
.canvas-container {
  /* Apply theme variables to canvas and all child elements */
  font-family: var(--theme-font-primary, "Inter", sans-serif);
  color: var(--theme-text, #ffffff);
  background-color: var(--theme-bg, #000000);

  /* Theme-aware elements can use these variables */
  --element-border: var(--theme-border, 2px solid #3b82f6);
  --element-radius: var(--theme-border-radius, 8px);
  --element-shadow: var(--theme-shadow, 0 4px 12px rgba(0, 0, 0, 0.1));
  --element-spacing: var(--theme-spacing-lg, 16px);
  --transition-speed: var(--theme-transition-normal, 250ms);

  /* Responsive sizing while maintaining aspect ratio */
  transform-origin: center;
  box-shadow: 0 0 50px rgba(0, 0, 0, 0.1);
  /* Scale to fit within wrapper while maintaining aspect ratio */
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
}

/* Example themed components */
.canvas-container .overlay-element {
  border: var(--element-border);
  border-radius: var(--element-radius);
  box-shadow: var(--element-shadow);
  padding: var(--element-spacing);
  background: var(--theme-card-bg, #111111);
  /* Only transition visual properties, not position/size */
  transition: background var(--transition-speed) var(--theme-ease, ease-in-out),
    border var(--transition-speed) var(--theme-ease, ease-in-out),
    box-shadow var(--transition-speed) var(--theme-ease, ease-in-out);
  font-family: var(--theme-font-primary);

  /* Image border support */
  border-image: var(--theme-border-image);
  border-image-slice: var(--theme-border-image-slice);
  border-image-width: var(--theme-border-image-width);
  border-image-repeat: var(--theme-border-image-repeat);
}

.canvas-container .overlay-button {
  background: var(--theme-button-bg, #3b82f6);
  color: var(--theme-button-text, #ffffff);
  border: var(--theme-border-thin, 1px solid #3b82f6);
  border-radius: var(--theme-border-radius-sm, 4px);
  padding: var(--theme-spacing-sm, 8px) var(--theme-spacing-md, 12px);
  font-family: var(--theme-font-secondary);
  font-weight: var(--theme-font-weight-medium, 500);
  font-size: var(--theme-font-size-base, 14px);
  /* Only transition visual properties, not position/size */
  transition: background var(--theme-transition-fast, 150ms)
      var(--theme-ease, ease-in-out),
    border var(--theme-transition-fast, 150ms) var(--theme-ease, ease-in-out),
    box-shadow var(--theme-transition-fast, 150ms) var(--theme-ease, ease-in-out);
  filter: var(--theme-glow, none);
}

.canvas-container .overlay-button:hover {
  background: var(--theme-button-hover, #2563eb);
  box-shadow: var(--theme-shadow-sm, 0 2px 4px rgba(0, 0, 0, 0.1));
}

.canvas-container .overlay-text {
  font-family: var(--theme-font-primary);
  font-size: var(--theme-font-size-base, 14px);
  color: var(--theme-text, #ffffff);
  text-shadow: var(--theme-glow, none);
}

.canvas-container .overlay-input {
  background: var(--theme-input-bg, #000000);
  border: var(--theme-input-border, 1px solid #3b82f6);
  border-radius: var(--theme-border-radius, 8px);
  color: var(--theme-text, #ffffff);
  padding: var(--theme-spacing-sm, 8px);
  font-family: var(--theme-font-primary);
  /* Only transition visual properties, not position/size */
  transition: background var(--theme-transition-fast, 150ms)
      var(--theme-ease, ease-in-out),
    border var(--theme-transition-fast, 150ms) var(--theme-ease, ease-in-out);
}

.canvas-container .overlay-card {
  background: var(--theme-card-bg, #111111);
  border: var(--theme-border-thin, 1px solid #3b82f6);
  border-radius: var(--theme-border-radius, 8px);
  box-shadow: var(--theme-shadow, 0 4px 12px rgba(0, 0, 0, 0.1));
  padding: var(--theme-spacing-lg, 16px);
  filter: var(--theme-glow, none);
}

/* Animation effects based on theme */
.canvas-container .theme-glow-effect {
  animation: theme-glow 2s ease-in-out infinite alternate;
}

@keyframes theme-glow {
  from {
    box-shadow: var(--theme-shadow, 0 4px 12px rgba(0, 0, 0, 0.1));
  }
  to {
    box-shadow: var(--theme-shadow-lg, 0 8px 24px rgba(0, 0, 0, 0.15));
  }
}

/* Advanced image-based border system */
.canvas-container .image-border-element {
  position: relative;
  border: none;
  padding: calc(var(--theme-border-image-width, 0px) + var(--theme-spacing-lg, 16px));
}

/* Corner decorations for image borders */
.canvas-container .image-border-element::before,
.canvas-container .image-border-element::after {
  content: "";
  position: absolute;
  background-image: var(--theme-corner-image);
  background-size: contain;
  background-repeat: no-repeat;
  width: var(--theme-border-image-width, 30px);
  height: var(--theme-border-image-width, 30px);
  opacity: 0.8;
}

.canvas-container .image-border-element::before {
  top: 0;
  left: 0;
}

.canvas-container .image-border-element::after {
  bottom: 0;
  right: 0;
  transform: rotate(180deg);
}

/* Pattern backgrounds for themed elements */
.canvas-container .pattern-background {
  background-image: var(--theme-pattern-image);
  background-size: 100px 100px;
  background-repeat: repeat;
  background-blend-mode: overlay;
  opacity: 0.1;
}

/* Edge decorations */
.canvas-container .edge-decorated::before {
  content: "";
  position: absolute;
  top: 0;
  left: 50%;
  transform: translateX(-50%);
  width: 80%;
  height: 4px;
  background-image: var(--theme-edge-image);
  background-size: cover;
  background-repeat: repeat-x;
}

/* Complex border frames using multiple images */
.canvas-container .complex-frame {
  position: relative;
  border: var(--theme-border-image-width, 20px) solid transparent;
  border-image: var(--theme-border-image) var(--theme-border-image-slice, 30)
    var(--theme-border-image-repeat, stretch);
}

/* Cyberpunk-style circuit borders */
.canvas-container .cyberpunk-border {
  position: relative;
  overflow: visible;
}

.canvas-container .cyberpunk-border::before {
  content: "";
  position: absolute;
  inset: -10px;
  background: linear-gradient(
      45deg,
      transparent 40%,
      var(--theme-primary, #ff00ff) 50%,
      transparent 60%
    ),
    var(--theme-pattern-image);
  background-size: 20px 20px, 50px 50px;
  opacity: 0.3;
  z-index: -1;
  border-radius: var(--theme-border-radius);
}

/* Retro pixel borders using image tiling */
.canvas-container .pixel-border {
  border: var(--theme-border-image-width, 8px) solid;
  border-image: var(--theme-border-image) 8 repeat;
  image-rendering: pixelated;
  image-rendering: -moz-crisp-edges;
}

/* Neon frame effects with image overlays */
.canvas-container .neon-frame {
  position: relative;
  border: 2px solid var(--theme-primary, #ff0080);
  box-shadow: 0 0 10px var(--theme-primary, #ff0080),
    inset 0 0 10px var(--theme-primary, #ff0080);
}

.canvas-container .neon-frame::after {
  content: "";
  position: absolute;
  inset: -15px;
  background-image: var(--theme-border-image);
  background-size: cover;
  opacity: 0.6;
  z-index: -1;
  filter: blur(2px) hue-rotate(30deg);
}

/* Interactive canvas elements - GPU accelerated (Canva-style) */
.element-interactive {
  /* Only transition properties that don't interfere with drag operations */
  transition: border-color 0.15s ease, box-shadow 0.15s ease, z-index 0s;
  z-index: 10;
  border: 2px solid transparent;
  /* GPU acceleration optimizations */
  will-change: transform;
  transform: translateZ(0);
  backface-visibility: hidden;
  perspective: 1000px;
}

.element-interactive:hover {
  border-color: rgba(59, 130, 246, 0.5);
  box-shadow: 0 0 0 1px rgba(59, 130, 246, 0.3), 0 4px 12px rgba(59, 130, 246, 0.1);
  z-index: 20;
  transform: translateZ(0); /* Force hardware acceleration */
}

/* Override any theme transitions that might interfere with drag operations */
.element-interactive[class*="theme"],
.element-interactive .overlay-element,
.element-interactive * {
  transition: border-color 0.15s ease, background-color 0.15s ease, box-shadow 0.15s ease !important;
}

.element-interactive.selected {
  z-index: 30;
  border-color: rgba(59, 130, 246, 0.8);
}

.element-interactive.selected:hover {
  border-color: rgba(59, 130, 246, 1);
}

.resize-handle {
  pointer-events: all !important;
  transition: all 0.2s ease;
  border: 1px solid #ffffff;
  border-radius: 1px;
  z-index: 50;
  position: absolute;
}

.resize-handle:hover {
  background-color: #2563eb !important;
  transform: scale(1.3);
  box-shadow: 0 0 0 1px rgba(37, 99, 235, 0.5);
}

.selection-outline {
  z-index: 40;
}

/* Fade transition for context menu */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

/* Right panel slide transition */
.slide-right-enter-active,
.slide-right-leave-active {
  transition: transform 0.2s ease, opacity 0.2s ease;
}

.slide-right-enter-from,
.slide-right-leave-to {
  transform: translateX(20px);
  opacity: 0;
}
</style>
