<template lang="pug">
div(
  v-if="show"
  @click.stop=""
  class="flow-node-library absolute top-14 left-6 w-72 rounded-2xl p-4 z-[1000] max-h-[calc(100vh-18rem)] flex flex-col"
)
  div(class="flow-node-library-tabs absolute flex items-center justify-start top-[-1.6rem] left-0 w-full h-[1.6rem] gap-x-1.5 px-3")
    button(
      v-for="category in menuCategories"
      :key="category.name"
      :class="category.active ? 'flow-node-library-tab--active' : 'flow-node-library-tab--idle'"
      class="flow-node-library-tab text-xs rounded-t-xl px-3.5 py-1"
      @click="toggleMenuCategory(category)"
    ) {{ category.name }}
  
  div(class="flow-node-library-header flex items-center gap-2.5 mb-4 pb-2.5 cursor-pointer" @click="clearNodeLibrarySearch")
    div(class="flow-node-library-badge w-7 h-7 rounded-xl flex items-center justify-center shrink-0")
      EvaCubeOutline(class="w-4 h-4")
    h3(class="text-sm font-bold text-themeText tracking-tight") Node Library
  
  // Search bar for nodes (only visible on Nodes tab)
  div(v-if="activeTab === 'Nodes'" class="mb-3.5 relative")
    input(
      autofocus
      type="text"
      v-model="nodeLibrarySearch"
      placeholder="Search library..."
      class="flow-node-library-input w-full px-3 py-1.5 pr-7 rounded-xl text-xs focus:outline-none"
    )
    button(
      v-if="nodeLibrarySearch"
      @click="nodeLibrarySearch = ''"
      class="flow-node-library-input-clear absolute right-2.5 top-1/2 transform -translate-y-1/2 transition-colors cursor-pointer"
    )
      EvaCloseOutline(class="w-3.5 h-3.5")

  // Search bar for flows (only visible on Flows tab)  
  div(v-if="activeTab === 'Flows'" class="mb-3.5 relative")
    input(
      type="text"
      v-model="flowsSearch"
      placeholder="Search flows..."
      class="flow-node-library-input w-full px-3 py-1.5 pr-7 rounded-xl text-xs focus:outline-none"
    )
    button(
      v-if="flowsSearch"
      @click="flowsSearch = ''"
      class="flow-node-library-input-clear absolute right-2.5 top-1/2 transform -translate-y-1/2 transition-colors cursor-pointer"
    )
      EvaCloseOutline(class="w-3.5 h-3.5")
  
  div(class="flex-1 overflow-y-scroll pr-2 -mr-2 custom-scrollbar-always")
    // Flows Tab Content
    div(v-if="activeTab === 'Flows'")
      // New Flow Button
      button(
        @click="createNewFlow"
        class="w-full mb-3 px-3 py-2 bg-themePrimary/15 hover:bg-themePrimary/25 border border-themePrimary/40 rounded-xl text-themePrimary text-xs font-semibold transition-all shadow-2xs cursor-pointer flex items-center justify-center gap-1.5"
        title="Create a new flow"
      )
        span(class="text-sm font-bold leading-none") +
        span(class="text-xs font-semibold") New Flow

      // Flows List
      div(v-if="filteredFlows.length > 0" class="space-y-1.5")
        div(
          v-for="flow in filteredFlows"
          :key="flow.id"
          class="flow-node-library-flow-card group p-2.5 rounded-xl cursor-pointer transition-all duration-150"
          :class="getFlowCardClasses(flow)"
          @click="loadFlow(flow)"
        )
          div(class="flex items-center justify-between")
            div(class="flex-1 min-w-0")
              div(class="flex items-center gap-1.5 mb-0.5")
                span(v-if="isCurrentFlow(flow)" class="text-xs text-themePrimary") ●
                span(class="text-xs font-semibold text-themeText truncate" :class="getFlowNameClasses(flow)") {{ flow.name }}
                span(v-if="flow.isTemporary" class="text-[10px] font-semibold bg-amber-500/15 text-amber-700 dark:text-amber-300 px-1.5 py-0.2 rounded-full border border-amber-500/30") Unsaved
                span(v-else-if="flow.version" class="text-[10px] font-mono bg-themeBackground2 text-themeText3 px-1.5 py-0.2 rounded border border-themeBorder") v{{ flow.version }}
              div(v-if="flow.description" class="text-[11px] text-themeText3 truncate") {{ flow.description }}
              div(class="text-[10px] text-themeText3/80 mt-0.5") {{ formatDate(flow.updated_at) }}
            div(class="flex items-center gap-1.5 ml-2")
              button(
                v-if="!flow.isTemporary"
                @click.stop="deleteFlow(flow)"
                class="opacity-0 group-hover:opacity-100 p-1 text-red-400 hover:text-red-300 transition-all duration-200"
                title="Delete Flow"
              )
                span(class="text-xs") 🗑️
              span(class="text-themeText3 group-hover:text-themeText text-xs") →
      
      // Empty state
      div(v-else class="text-center py-8")
        div(class="text-themeText3 mb-2") 📋
        div(class="text-xs text-themeText2 mb-1") No flows found
        div(class="text-[11px] text-themeText3") Save your first flow to see it here
    
    // Nodes Tab Content
    div(v-else-if="activeTab === 'Nodes'")
      div(class="flow-node-library-filter-bar mb-3")
        button(
          class="flow-node-library-filter-chip"
          :class="nodeQuickFilter === 'all' ? 'flow-node-library-filter-chip--active' : ''"
          @click="nodeQuickFilter = 'all'"
        ) All
        button(
          class="flow-node-library-filter-chip"
          :class="nodeQuickFilter === 'favorites' ? 'flow-node-library-filter-chip--active' : ''"
          @click="nodeQuickFilter = 'favorites'"
        ) Favorites
        button(
          class="flow-node-library-filter-chip"
          :class="nodeQuickFilter === 'recent' ? 'flow-node-library-filter-chip--active' : ''"
          @click="nodeQuickFilter = 'recent'"
        ) Recent

      template(v-if="nodeQuickFilter === 'favorites'")
        h3(class="flow-node-library-section-head mt-2 text-sm font-medium flex justify-between items-center p-2 rounded-md")
          div(class="flex items-center gap-2")
            span(class="text-themeText font-semibold text-xs") Favorite Nodes
          div(class="flow-node-picker-pill") {{ favoriteNodes.length }}
        div(v-if="favoriteNodes.length > 0" class="space-y-1.5 mb-4")
          div(
            v-for="type in favoriteNodes"
            :key="'favorite-' + type.name"
            class="flow-node-library-node-card flow-node-picker-node group cursor-move"
            draggable="true"
            @dragstart="handleNodeDragStart($event, type.name)"
            @click="handleAddNodeOfType(type.name)"
          )
            div(class="flow-node-library-node-main-col")
              div(class="flow-node-library-node-main")
                span(class="flow-node-library-node-label truncate") {{ type.label }}
                span(class="flow-node-picker-pill flow-node-library-node-pill") {{ getNodeTypeBadge(type) }}
              div(class="flow-node-library-node-meta-row")
                span(class="flow-node-library-node-meta truncate") {{ type.name }}
                span(class="flow-node-library-node-io") {{ getNodeTypeIoSummary(type) }}
            div(class="flow-node-library-actions")
              button(
                class="flow-node-library-node-favorite"
                :class="isFavoriteNode(type.name) ? 'is-active' : ''"
                @click.stop="toggleFavoriteNode(type.name, $event)"
                title="Toggle favorite"
              )
                SolarStarBold(class="w-3.5 h-3.5")
              button(
                class="flow-node-library-node-add"
                @click.stop="handleAddNodeOfType(type.name)"
                title="Add to canvas"
              )
                EvaPlusOutline(class="w-3.5 h-3.5")
        div(v-else class="flow-node-picker-empty") No favorites yet.

      template(v-else-if="nodeQuickFilter === 'recent'")
        h3(class="flow-node-library-section-head mt-2 text-sm font-medium flex justify-between items-center p-2 rounded-md")
          div(class="flex items-center gap-2")
            span(class="text-themeText font-semibold text-xs") Recent Nodes
          div(class="flow-node-picker-pill") {{ recentNodes.length }}
        div(v-if="recentNodes.length > 0" class="space-y-1.5 mb-4")
          div(
            v-for="type in recentNodes"
            :key="'recent-' + type.name"
            class="flow-node-library-node-card flow-node-picker-node group cursor-move"
            draggable="true"
            @dragstart="handleNodeDragStart($event, type.name)"
            @click="handleAddNodeOfType(type.name)"
          )
            div(class="flow-node-library-node-main-col")
              div(class="flow-node-library-node-main")
                span(class="flow-node-library-node-label truncate") {{ type.label }}
                span(class="flow-node-picker-pill flow-node-library-node-pill") {{ getNodeTypeBadge(type) }}
              div(class="flow-node-library-node-meta-row")
                span(class="flow-node-library-node-meta truncate") {{ type.name }}
                span(class="flow-node-library-node-io") {{ getNodeTypeIoSummary(type) }}
            div(class="flow-node-library-actions")
              button(
                class="flow-node-library-node-favorite"
                :class="isFavoriteNode(type.name) ? 'is-active' : ''"
                @click.stop="toggleFavoriteNode(type.name, $event)"
                title="Toggle favorite"
              )
                SolarStarBold(class="w-3.5 h-3.5")
              button(
                class="flow-node-library-node-add"
                @click.stop="handleAddNodeOfType(type.name)"
                title="Add to canvas"
              )
                EvaPlusOutline(class="w-3.5 h-3.5")
        div(v-else class="flow-node-picker-empty") No recent nodes yet.

      template(v-else)
        // Every node source uses the same canonical category/subgroup catalog.
        template(v-for="category in filteredCatalog" :key="category.id")
          h3(
            class="flow-node-library-section-head mt-2 text-sm font-medium flex justify-between items-center cursor-pointer group p-2 rounded-md transition-colors duration-200"
            @click="toggleCategory(category.id)"
          )
            div(class="flex items-center gap-2")
              span(class="text-xs text-themeText3 transition-transform duration-200" :class="expandedCategories[category.id] ? 'rotate-90' : ''") ›
              span(class="text-themeText font-medium") {{ category.label }}
            div(class="flow-node-picker-pill") {{ category.items.length }}

          div(v-if="expandedCategories[category.id]" class="ml-1 mb-3")
            div(v-for="group in category.groups" :key="`${category.id}-${group.id}`" class="space-y-1.5")
              div(
                v-if="category.groups.length > 1 || group.id !== 'General'"
                class="px-2 pt-2 pb-0.5 text-[10px] font-semibold uppercase tracking-wider text-themeText3"
              ) {{ group.label }}
              div(
                v-for="item in group.items"
                :key="item.name"
                class="flow-node-library-node-card flow-node-picker-node group cursor-move"
                draggable="true"
                @dragstart="handleNodeDragStart($event, item.name)"
                @click="handleAddNodeOfType(item.name)"
              )
                div(class="flow-node-library-node-main-col")
                  div(class="flow-node-library-node-main")
                    span(class="flow-node-library-node-label truncate") {{ item.label }}
                    span(class="flow-node-picker-pill flow-node-library-node-pill") {{ getNodeTypeBadge(item) }}
                  div(class="flow-node-library-node-meta-row")
                    span(class="flow-node-library-node-meta truncate") {{ item.name }}
                    span(class="flow-node-library-node-io") {{ getNodeTypeIoSummary(item) }}
                div(class="flow-node-library-actions")
                  button(
                    class="flow-node-library-node-favorite"
                    :class="isFavoriteNode(item.name) ? 'is-active' : ''"
                    @click.stop="toggleFavoriteNode(item.name, $event)"
                    title="Toggle favorite"
                  )
                    SolarStarBold(class="w-3.5 h-3.5")
                  button(
                    class="flow-node-library-node-add"
                    @click.stop="handleAddNodeOfType(item.name)"
                    title="Add to canvas"
                  )
                    EvaPlusOutline(class="w-3.5 h-3.5")

    // Variables Tab Content
    div(v-else-if="activeTab === 'Variables'")
      // Search bar for variables
      div(class="mb-3.5 relative")
        input(
          type="text"
          v-model="variablesSearch"
          placeholder="Search variables..."
          class="flow-node-library-input w-full px-3 py-1.5 pr-7 rounded-xl text-xs focus:outline-none"
        )
        button(
          v-if="variablesSearch"
          @click="variablesSearch = ''"
          class="flow-node-library-input-clear absolute right-2.5 top-1/2 transform -translate-y-1/2 transition-colors cursor-pointer"
        )
          EvaCloseOutline(class="w-3.5 h-3.5")

      // Variables list
      div(v-if="filteredVariables.length > 0" class="space-y-1.5")
        div(
          v-for="variable in filteredVariables"
          :key="`${variable.nodeId}-${variable.name}`"
          class="group p-2.5 bg-themeBackground hover:bg-themeBackground3 border border-themeBorder hover:border-themeBorder2 rounded-xl cursor-pointer transition-all duration-150 shadow-2xs"
        )
          div(class="flex items-center justify-between")
            div(class="flex-1 min-w-0")
              div(class="flex items-center gap-2 mb-0.5")
                span(class="text-themePrimary text-xs font-bold font-mono") $
                span(class="text-xs font-bold text-themeText truncate") {{ variable.name }}
              div(class="text-[11px] text-themeText3 truncate") from: {{ variable.nodeName }}
              div(v-if="variable.value !== undefined" class="text-[10px] font-mono text-themeText3 truncate mt-0.5") value: {{ formatVariableValue(variable.value) }}
            div(class="flex items-center gap-1.5 ml-2")
              button(
                @click.stop="highlightVariableNode(variable.nodeId)"
                class="opacity-0 group-hover:opacity-100 p-1 text-themeText3 hover:text-themePrimary transition-all duration-200"
                title="Locate SetVariable node"
              )
                span(class="text-xs") 📍
              button(
                @click.stop="selectVariable(variable.name)"
                class="opacity-0 group-hover:opacity-100 p-1 text-themeText3 hover:text-themePrimary transition-all duration-200"
                title="Use this variable"
              )
                span(class="text-xs") ✓

      // Empty state - no variables defined
      div(v-else-if="extractedVariables.length === 0" class="text-center py-8")
        div(class="text-3xl mb-2.5") 📦
        div(class="text-xs font-bold text-themeText mb-1") No variables defined
        div(class="text-[11px] text-themeText3 mb-4") Add a "Set Variable" node to create variables
        button(
          @click="emit('addNode', 'set-variable')"
          class="px-3.5 py-1.5 bg-themePrimary/15 hover:bg-themePrimary/25 border border-themePrimary/40 rounded-xl text-themePrimary text-xs font-semibold transition-all shadow-2xs cursor-pointer inline-flex items-center gap-1.5"
        )
          span +
          span Add Set Variable

      // Empty state - search has no results
      div(v-else class="text-center py-8")
        div(class="text-themeText3 mb-2") 🔍
        div(class="text-xs text-themeText2") No variables match "{{ variablesSearch }}"
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import EvaCubeOutline from "~icons/eva/cube-outline";
import EvaCloseOutline from "~icons/eva/close-outline";
import SolarStarBold from "~icons/solar/star-bold";
import EvaPlusOutline from "~icons/eva/plus-outline";

import { buildNodeCatalog } from "../../../utils/nodeCatalog";

// Define interfaces for the data we work with
interface NodeType {
  name: string;
  label: string;
  category: string;
  category_display_name?: string;
  subcategory?: string;
  color?: string;
  description?: string;
  inputs?: Array<{ id?: string; name?: string; type?: string }>;
  outputs?: Array<{ id?: string; name?: string; type?: string }>;
}

interface MenuCategory {
  name: string;
  icon: string;
  color: string;
  active: boolean;
}

// Flow node interface for variables extraction
interface FlowNode {
  id: number;
  type: string;
  text: string;
  inputs?: Array<{
    id: string;
    name: string;
    type: string;
    value?: any;
  }>;
}

// Props
interface Props {
  show: boolean;
  triggerNodes?: NodeType[];
  controlNodes?: NodeType[];
  aiNodes?: NodeType[];
  audioNodes?: NodeType[];
  wasmPlugins?: NodeType[];
  nodeTypeColors?: Record<string, string>;
  flows?: any[];
  currentFlowId?: string | null;
  nodes?: FlowNode[]; // Current flow nodes for variable extraction
}

const props = withDefaults(defineProps<Props>(), {
  show: false,
  triggerNodes: () => [],
  controlNodes: () => [],
  aiNodes: () => [],
  audioNodes: () => [],
  wasmPlugins: () => [],
  nodeTypeColors: () => ({}),
  flows: () => [],
  currentFlowId: null,
  nodes: () => [],
});

// Emits
const emit = defineEmits<{
  close: [];
  dragStart: [event: DragEvent, nodeTypeName: string];
  addNode: [nodeTypeName: string];
  loadFlow: [flow: any];
  deleteFlow: [flow: any];
  createNewFlow: [];
  selectVariable: [variableName: string]; // When user clicks a variable to insert
  highlightNode: [nodeId: number]; // To highlight the SetVariable node
}>();

// Local reactive state
const nodeLibrarySearch = ref("");
const flowsSearch = ref("");
const nodeQuickFilter = ref<"all" | "favorites" | "recent">("all");
const favoriteNodeNames = ref<string[]>([]);
const recentNodeNames = ref<string[]>([]);
const NODE_LIBRARY_FAVORITES_KEY = "flow-node-library-favorites";
const NODE_LIBRARY_RECENTS_KEY = "flow-node-library-recents";
const MAX_RECENTS = 12;
const menuCategories = ref<MenuCategory[]>([
  {
    name: "Nodes",
    icon: "mdi:node-plus",
    color: "bg-gray-800/95",
    active: true,
  },
  {
    name: "Variables",
    icon: "mdi:variable",
    color: "bg-gray-800/95",
    active: false,
  },
  {
    name: "Flows",
    icon: "mdi:flow-chart",
    color: "bg-gray-800/95",
    active: false,
  },
]);

const expandedCategories = ref<Record<string, boolean>>({
  TRIGGER: true,
  AUDIO: true,
  CONTROL: true,
  AI: true,
  SYSTEM: true,
  INTEGRATION: true,
  STREAM: true,
  UTILITIES: true,
  ACTION: true,
  MEDIA: true,
  VARIABLES: true,
  MACRO: true,
  PLUGINS: true,
  TEXT: true,
  DATA: true,
  OTHER: true,
});

// Helper functions

const normalizeSearch = (value: string): string => value.toLowerCase().trim();

const isFavoriteNode = (nodeTypeName: string): boolean =>
  favoriteNodeNames.value.includes(nodeTypeName);

const markNodeAsRecent = (nodeTypeName: string) => {
  const deduped = [nodeTypeName, ...recentNodeNames.value.filter((name) => name !== nodeTypeName)];
  recentNodeNames.value = deduped.slice(0, MAX_RECENTS);
  if (typeof window !== "undefined") {
    localStorage.setItem(NODE_LIBRARY_RECENTS_KEY, JSON.stringify(recentNodeNames.value));
  }
};

const toggleFavoriteNode = (nodeTypeName: string, event?: Event) => {
  event?.stopPropagation();
  event?.preventDefault();

  if (favoriteNodeNames.value.includes(nodeTypeName)) {
    favoriteNodeNames.value = favoriteNodeNames.value.filter((name) => name !== nodeTypeName);
  } else {
    favoriteNodeNames.value = [...favoriteNodeNames.value, nodeTypeName];
  }

  if (typeof window !== "undefined") {
    localStorage.setItem(
      NODE_LIBRARY_FAVORITES_KEY,
      JSON.stringify(favoriteNodeNames.value)
    );
  }
};

const getNodeTypeBadge = (node: NodeType): string => {
  const category = String(node.category || "").toUpperCase();
  if (category.includes("TRIGGER")) return "Trigger";
  if (category.includes("AI")) return "AI";
  if (category.includes("AUDIO")) return "Audio";
  if (category.includes("CONTROL")) return "Control";
  return "Action";
};

const getNodeTypeIoSummary = (node: NodeType): string => {
  const inputCount = Array.isArray(node.inputs)
    ? node.inputs.filter((input) => input?.type !== "flow").length
    : 0;
  const outputCount = Array.isArray(node.outputs)
    ? node.outputs.filter((output) => output?.type !== "flow").length
    : 0;
  return `${inputCount} in · ${outputCount} out`;
};

const matchesQuickFilter = (node: NodeType): boolean => {
  if (nodeQuickFilter.value === "favorites") return isFavoriteNode(node.name);
  if (nodeQuickFilter.value === "recent") return recentNodeNames.value.includes(node.name);
  return true;
};

const matchesNodeSearch = (node: NodeType): boolean => {
  const query = normalizeSearch(nodeLibrarySearch.value);
  if (!query) return true;

  return (
    normalizeSearch(node.label).includes(query) ||
    normalizeSearch(node.name).includes(query) ||
    normalizeSearch(node.category_display_name || "").includes(query) ||
    normalizeSearch(node.subcategory || "").includes(query) ||
    normalizeSearch(node.description || "").includes(query)
  );
};

const filterNodeCollection = (items: NodeType[]) =>
  items.filter((item) => matchesQuickFilter(item) && matchesNodeSearch(item));

const allNodeTypes = computed<NodeType[]>(() => {
  const merged = [
    ...props.triggerNodes,
    ...props.controlNodes,
    ...props.aiNodes,
    ...props.audioNodes,
    ...props.wasmPlugins,
  ];
  const byName = new Map<string, NodeType>();
  for (const node of merged) {
    if (!node?.name) continue;
    if (!byName.has(node.name)) byName.set(node.name, node);
  }
  return Array.from(byName.values());
});

const favoriteNodes = computed<NodeType[]>(() => {
  const byName = new Map(allNodeTypes.value.map((node) => [node.name, node]));
  return favoriteNodeNames.value
    .map((name) => byName.get(name))
    .filter((node): node is NodeType => Boolean(node))
    .filter((node) => matchesNodeSearch(node));
});

const recentNodes = computed<NodeType[]>(() => {
  const byName = new Map(allNodeTypes.value.map((node) => [node.name, node]));
  return recentNodeNames.value
    .map((name) => byName.get(name))
    .filter((node): node is NodeType => Boolean(node))
    .filter((node) => matchesNodeSearch(node));
});

const filteredCatalog = computed(() =>
  buildNodeCatalog(filterNodeCollection(allNodeTypes.value)),
);

// Flows computed properties
const activeTab = computed(() => {
  return menuCategories.value.find((c) => c.active)?.name || "Nodes";
});

const filteredFlows = computed(() => {
  if (!flowsSearch.value) return props.flows;
  return props.flows.filter(
    (flow) =>
      flow.name.toLowerCase().includes(flowsSearch.value.toLowerCase()) ||
      (flow.description &&
        flow.description.toLowerCase().includes(flowsSearch.value.toLowerCase()))
  );
});

// Variables extracted from SetVariable nodes
interface ExtractedVariable {
  name: string;
  nodeId: number;
  nodeName: string;
  value?: any;
}

const variablesSearch = ref("");

const extractedVariables = computed<ExtractedVariable[]>(() => {
  const variables: ExtractedVariable[] = [];

  for (const node of props.nodes) {
    // Match SetVariable node type (case-insensitive, various naming conventions)
    if (
      node.type.toLowerCase() === "set-variable" ||
      node.type.toLowerCase() === "setvariable" ||
      node.type === "SetVariable"
    ) {
      // Find the variableName input
      const varNameInput = node.inputs?.find(
        (input) =>
          input.id === "variableName" ||
          input.name === "Variable Name" ||
          input.name.toLowerCase() === "variablename"
      );

      if (
        varNameInput?.value &&
        typeof varNameInput.value === "string" &&
        varNameInput.value.trim()
      ) {
        // Find the value input for display
        const valueInput = node.inputs?.find(
          (input) => input.id === "value" || input.name === "Value"
        );

        variables.push({
          name: varNameInput.value.trim(),
          nodeId: node.id,
          nodeName: node.text || "Set Variable",
          value: valueInput?.value,
        });
      }
    }
  }

  return variables;
});

const filteredVariables = computed(() => {
  if (!variablesSearch.value) return extractedVariables.value;
  return extractedVariables.value.filter((v) =>
    v.name.toLowerCase().includes(variablesSearch.value.toLowerCase())
  );
});

// Get all unique variable names for GetVariable dropdowns
const variableNames = computed(() => {
  return [...new Set(extractedVariables.value.map((v) => v.name))];
});

// Methods
const toggleMenuCategory = (category: MenuCategory) => {
  menuCategories.value.forEach((c) => {
    c.active = c === category;
  });
};

const toggleCategory = (category: string | number) => {
  const categoryKey = category.toString();
  expandedCategories.value[categoryKey] = !expandedCategories.value[categoryKey];
};

const clearNodeLibrarySearch = () => {
  nodeLibrarySearch.value = "";
  nodeQuickFilter.value = "all";
};

const getNodeTypeColor = (nodeTypeName: string): string => {
  return props.nodeTypeColors[nodeTypeName] || "#6B7280";
};

// Event handlers
const handleNodeDragStart = (event: DragEvent, nodeTypeName: string) => {
  emit("dragStart", event, nodeTypeName);
};

const handleAddNodeOfType = (nodeTypeName: string) => {
  markNodeAsRecent(nodeTypeName);
  emit("addNode", nodeTypeName);
};

// Flows methods
const loadFlow = (flow: any) => {
  emit("loadFlow", flow);
};

const deleteFlow = (flow: any) => {
  emit("deleteFlow", flow);
};

const createNewFlow = () => {
  emit("createNewFlow");
};

// Variable methods
const selectVariable = (variableName: string) => {
  emit("selectVariable", variableName);
};

const highlightVariableNode = (nodeId: number) => {
  emit("highlightNode", nodeId);
};

const formatVariableValue = (value: any): string => {
  if (value === undefined || value === null) return "(not set)";
  if (typeof value === "string")
    return value.length > 20 ? value.substring(0, 20) + "..." : value;
  if (typeof value === "object")
    return JSON.stringify(value).substring(0, 20) + "...";
  return String(value);
};

const formatDate = (dateString: string) => {
  try {
    // Handle null, undefined, or empty strings
    if (!dateString || dateString.trim() === "") {
      return "No date";
    }

    const date = new Date(dateString);

    // Check if the date is valid
    if (isNaN(date.getTime())) {
      return "Invalid date";
    }

    return (
      date.toLocaleDateString() +
      " " +
      date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
    );
  } catch {
    return "Invalid date";
  }
};

// Flow-related helper methods
const isCurrentFlow = (flow: any): boolean => {
  return flow.id === props.currentFlowId;
};

const getFlowCardClasses = (flow: any): string => {
  let classes = "flow-node-library-flow-card--default";

  if (isCurrentFlow(flow)) {
    classes = "flow-node-library-flow-card--current";
  } else if (flow.isTemporary) {
    classes = "flow-node-library-flow-card--temporary";
  }

  return classes;
};

const getFlowNameClasses = (flow: any): string => {
  if (isCurrentFlow(flow)) {
    return "flow-node-library-flow-name--current";
  } else if (flow.isTemporary) {
    return "flow-node-library-flow-name--temporary";
  }
  return "flow-node-library-flow-name--default";
};

onMounted(() => {
  if (typeof window === "undefined") return;

  try {
    const favoriteRaw = localStorage.getItem(NODE_LIBRARY_FAVORITES_KEY);
    favoriteNodeNames.value = favoriteRaw ? JSON.parse(favoriteRaw) : [];
  } catch {
    favoriteNodeNames.value = [];
  }

  try {
    const recentRaw = localStorage.getItem(NODE_LIBRARY_RECENTS_KEY);
    recentNodeNames.value = recentRaw ? JSON.parse(recentRaw) : [];
  } catch {
    recentNodeNames.value = [];
  }
});
</script>

<script lang="ts">
export default {
  name: "NodeLibrary",
};
</script>


<style lang="scss" scoped>
.flow-node-library {
  border: 1px solid hsl(var(--border));
  background: hsl(var(--background2) / 0.95);
  backdrop-filter: blur(14px);
  -webkit-backdrop-filter: blur(14px);
  box-shadow: 0 16px 36px -12px rgba(0, 0, 0, 0.35);
}

.flow-node-library-tab {
  font-weight: 600;
  transition: all 0.15s ease;
  border-top: 1px solid transparent;
  border-left: 1px solid transparent;
  border-right: 1px solid transparent;
}

.flow-node-library-tab--active {
  background: hsl(var(--background2));
  color: hsl(var(--primary));
  border-color: hsl(var(--border));
  border-bottom: 1px solid hsl(var(--background2));
}

.flow-node-library-tab--idle {
  background: hsl(var(--background) / 0.8);
  color: hsl(var(--text3));
  border-color: hsl(var(--border));
}

.flow-node-library-tab--idle:hover {
  color: hsl(var(--text));
  background: hsl(var(--background));
}

.flow-node-library-header {
  border-bottom: 1px solid hsl(var(--border) / 0.6);
}

.flow-node-library-filter-bar {
  display: flex;
  align-items: center;
  gap: 0.35rem;
}

.flow-node-library-filter-chip {
  font-size: 0.68rem;
  font-weight: 600;
  padding: 0.2rem 0.55rem;
  border-radius: 999px;
  border: 1px solid hsl(var(--border));
  background: hsl(var(--background));
  color: hsl(var(--text3));
  transition: all 0.15s ease;
}

.flow-node-library-filter-chip:hover {
  border-color: hsl(var(--border2));
  color: hsl(var(--text));
}

.flow-node-library-filter-chip--active {
  border-color: hsl(var(--primary) / 0.5);
  color: hsl(var(--primary));
  background: hsl(var(--primary) / 0.12);
}

.flow-node-library-section-head {
  border: 1px solid hsl(var(--border));
  background: hsl(var(--background));
  color: hsl(var(--text2));
  border-radius: 0.5rem;
}

.flow-node-library-section-head:hover {
  border-color: hsl(var(--border2));
  background: hsl(var(--background3));
}

.flow-node-library-badge {
  border: 1px solid hsl(var(--primary) / 0.3);
  background: hsl(var(--primary) / 0.12);
  color: hsl(var(--primary));
}

.flow-node-library-input {
  border: 1px solid hsl(var(--border)) !important;
  background: hsl(var(--background)) !important;
  color: hsl(var(--text)) !important;
}

.flow-node-library-input::placeholder {
  color: hsl(var(--text2) / 0.6) !important;
  opacity: 1;
}

.flow-node-library-input:focus {
  border-color: hsl(var(--primary) / 0.6) !important;
  box-shadow: 0 0 0 2px hsl(var(--primary) / 0.15);
}

.flow-node-library-input-clear {
  color: hsl(var(--text3));
}

.flow-node-library-input-clear:hover {
  color: hsl(var(--text));
}

.flow-node-library-flow-card--default:hover {
  border-color: hsl(var(--border2));
  background: hsl(var(--background3));
}

.flow-node-library-flow-card--current {
  border-color: hsl(var(--primary) / 0.5);
  background: hsl(var(--primary) / 0.1);
}

.flow-node-library-flow-card--current:hover {
  border-color: hsl(var(--primary) / 0.7);
  background: hsl(var(--primary) / 0.16);
}

.flow-node-library-flow-card--temporary {
  border: 1px dashed hsl(var(--primary) / 0.5);
  background: hsl(var(--background));
}

.flow-node-library-flow-card--temporary:hover {
  border-color: hsl(var(--primary) / 0.7);
  background: hsl(var(--background3));
}

.flow-node-library-flow-name--default {
  color: hsl(var(--text));
}

.flow-node-library-flow-name--current {
  color: hsl(var(--primary));
}

.flow-node-library-flow-name--temporary {
  color: hsl(var(--text));
}

.flow-node-library-node-card {
  border: 1px solid hsl(var(--border));
  border-radius: 0.75rem;
  background: hsl(var(--background));
  padding: 0;
  margin-bottom: 0;
  display: flex;
  align-items: stretch;
  overflow: hidden;
  transition: all 0.15s ease;
}

.flow-node-library-node-card:hover {
  border-color: hsl(var(--border2));
  background: hsl(var(--background3));
  transform: translateY(-1px);
}



.flow-node-library-node-main-col {
  flex: 1;
  min-width: 0;
  padding: 0.35rem 0.5rem;
  display: flex;
  flex-direction: column;
  justify-content: center;
}

.flow-node-library-node-main {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  min-width: 0;
}

.flow-node-library-node-label {
  color: hsl(var(--text));
  font-size: 0.78rem;
  line-height: 1.1rem;
  font-weight: 600;
}

.flow-node-library-node-pill {
  flex-shrink: 0;
  font-size: 0.58rem;
  padding: 0.05rem 0.35rem;
  color: hsl(var(--text3));
  background: hsl(var(--background2));
  border: 1px solid hsl(var(--border));
  border-radius: 999px;
}

.flow-node-library-node-meta-row {
  margin-top: 0.1rem;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.35rem;
  min-width: 0;
}

.flow-node-library-node-meta {
  color: hsl(var(--text3));
  font-size: 0.64rem;
  min-width: 0;
}

.flow-node-library-node-io {
  color: hsl(var(--text3) / 0.75);
  font-size: 0.62rem;
  flex-shrink: 0;
  white-space: nowrap;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

.flow-node-library-actions {
  display: flex;
  align-items: center;
  gap: 0.15rem;
  padding: 0 0.4rem;
  border-left: 1px solid hsl(var(--border));
  background: hsl(var(--background2) / 0.4);
  align-self: stretch;
  flex-shrink: 0;
}

.flow-node-library-node-favorite {
  width: 1.35rem;
  height: 1.35rem;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 0.35rem;
  color: hsl(var(--text2));
  transition: all 0.15s ease;
  cursor: pointer;

  &:hover {
    color: #f59e0b;
    background: rgba(245, 158, 11, 0.15);
    transform: scale(1.1);
  }

  &.is-active {
    color: #f59e0b;
    background: rgba(245, 158, 11, 0.18);
  }
}

.flow-node-library-node-add {
  width: 1.35rem;
  height: 1.35rem;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 0.35rem;
  color: hsl(var(--text));
  transition: all 0.15s ease;
  cursor: pointer;

  &:hover {
    color: hsl(var(--primary));
    background: hsl(var(--primary) / 0.15);
    transform: scale(1.1);
  }
}

.flow-node-library :deep(.custom-scrollbar-always) {
  scrollbar-gutter: stable;

  &::-webkit-scrollbar {
    width: 6px;
    height: 6px;
    display: block;
  }
  &::-webkit-scrollbar-track {
    background: transparent;
  }
  &::-webkit-scrollbar-thumb {
    background: hsl(var(--border2));
    border-radius: 999px;
  }
  &::-webkit-scrollbar-thumb:hover {
    background: hsl(var(--text3) / 0.5);
  }
  &::-webkit-scrollbar-button {
    display: none !important;
    width: 0 !important;
    height: 0 !important;
  }
}
</style>
