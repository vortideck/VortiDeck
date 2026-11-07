<template lang="pug">
    div(class="flow-editor-page flex flex-col h-screen bg-themeBackground text-themeText select-none overflow-hidden")
      // Header
      FlowPanelHeader(
          :showSettings="showSettings"
          :nodes="nodes"
          :connections="connections"
          :activeFlowExecution="activeFlowExecution"
          :isFlowRunning="isFlowRunning"
          :flowStartPending="flowStartPending"
          :camera="camera"
          :selectedNodes="selectedNodes"
          :selectedNode="selectedNode"
          :currentFlowName="currentFlowMetadata.name || 'Untitled Flow'"
          :hasUnsavedChanges="hasUnsavedChanges"
          :isNewFlow="currentFlowMetadata.isNew"
          :recoveryQueueCount="durableRecoveryRuns.length"
          @toggleSettings="showSettings = !showSettings"
          @executeFlow="executeFlowWithUnified()"
          @cancelFlow="cancelFlowUnified"
          @showRuntimeOperations="showRuntimeOperations = true"
          @deleteSelection="deleteSelection"
          @clearAll="clearAll"
          @exportFlow="exportFlowToFile"
          @exportRecipe="exportFlowRecipe"
          @rehearseFlow="rehearseFlowSafely"
          @importFlow="handleImportFromHeader"
          @saveFlow="saveFlow"
          @saveFlowAs="saveFlowAs"
          @newFlow="createNewFlow"
          @editFlowInfo="editFlowInfo"
        )
    
      // Settings dropdown - positioned outside canvas area
      div(
        v-if="showSettings"
        class="flow-settings-menu fixed w-80 bg-themeBackground2 rounded-2xl shadow-2xl border border-themeBorder z-[9999] p-5 settings-menu"
        :style="{ top: '4.5rem', right: '1rem' }"
        @click.stop=""
        @mousedown.stop=""
        @mouseup.stop=""
      )
        // Header
        div(class="flex items-center justify-between mb-4 pb-3 border-b border-themeBorder/40")
          div(class="flex items-center gap-2.5")
            div(class="w-7 h-7 rounded-xl bg-themePrimary/12 border border-themePrimary/20 flex items-center justify-center text-themePrimary")
              SolarSettingsBold(class="w-4 h-4")
            h3(class="text-sm font-bold text-themeText tracking-tight") Editor Settings
          button(@click="showSettings = false" class="p-1 rounded-lg text-themeText3 hover:text-themeText hover:bg-themeBackground transition-colors cursor-pointer" title="Close")
            SolarCloseCircleBold(class="w-4 h-4")
        
        div(class="space-y-4")
          // Snap to grid toggle
          div(class="flex items-center justify-between p-3 rounded-xl bg-themeBackground border border-themeBorder")
            div(class="flex flex-col")
              span(class="text-xs font-semibold text-themeText") Snap to Grid
              span(class="text-[11px] text-themeText3 leading-tight mt-0.5") Align nodes when dragging
            label(class="theme-toggle-switch")
              input(type="checkbox" v-model="snapToGrid" class="sr-only")
              span(class="theme-toggle-slider" :class="{ 'theme-toggle-slider--checked': snapToGrid }")
                span(class="theme-toggle-knob")
          
          // Show grid toggle
          div(class="flex items-center justify-between p-3 rounded-xl bg-themeBackground border border-themeBorder")
            div(class="flex flex-col")
              span(class="text-xs font-semibold text-themeText") Background Grid
              span(class="text-[11px] text-themeText3 leading-tight mt-0.5") Show canvas pattern
            label(class="theme-toggle-switch")
              input(type="checkbox" v-model="showGrid" class="sr-only")
              span(class="theme-toggle-slider" :class="{ 'theme-toggle-slider--checked': showGrid }")
                span(class="theme-toggle-knob")

          // Grid pattern style picker
          div(v-if="showGrid" class="p-3 rounded-xl bg-themeBackground border border-themeBorder")
            div(class="flex items-center justify-between mb-2")
              span(class="text-xs font-semibold text-themeText") Pattern Style
              span(class="text-[10px] font-medium text-themePrimary bg-themePrimary/10 border border-themePrimary/20 px-1.5 py-0.5 rounded") Adaptive LOD
            div(class="grid grid-cols-3 gap-1.5")
              button(
                v-for="pattern in ['dots', 'crosses', 'lines']"
                :key="pattern"
                @click="gridPattern = pattern"
                class="px-2 py-1.5 rounded-lg text-xs font-semibold capitalize transition-all cursor-pointer text-center"
                :class="gridPattern === pattern ? 'bg-themePrimary text-white shadow-2xs' : 'bg-themeBackground2 hover:bg-themeBackground3 text-themeText3 hover:text-themeText border border-themeBorder'"
              ) {{ pattern }}
        
        ActionInputContractEditor(class="mt-4" :reference="currentFlowMetadata.isNew ? null : { kind: 'flow', id: currentFlowMetadata.id }")

        // Keyboard shortcuts section
        div(class="mt-4 pt-3 border-t border-themeBorder/40")
          div(class="flex items-center gap-1.5 mb-2")
            HugeiconsCommandLine(class="w-3.5 h-3.5 text-themeText3")
            span(class="text-[11px] font-bold uppercase tracking-wider text-themeText3") Shortcuts
          div(class="space-y-1.5 text-xs text-themeText2")
            div(class="flex justify-between items-center py-0.5")
              span Pan Canvas
              span(class="bg-themeBackground px-2 py-0.5 rounded-md border border-themeBorder text-[10px] font-mono") Left Drag
            div(class="flex justify-between items-center py-0.5")
              span Select Nodes
              span(class="bg-themeBackground px-2 py-0.5 rounded-md border border-themeBorder text-[10px] font-mono") Shift + Drag
            div(class="flex justify-between items-center py-0.5")
              span Node Menu
              span(class="bg-themeBackground px-2 py-0.5 rounded-md border border-themeBorder text-[10px] font-mono") Right Click
            div(class="flex justify-between items-center py-0.5")
              span Multi-select
              span(class="bg-themeBackground px-2 py-0.5 rounded-md border border-themeBorder text-[10px] font-mono") Ctrl + Click
            div(class="flex justify-between items-center py-0.5")
              span Delete Selected
              span(class="bg-themeBackground px-2 py-0.5 rounded-md border border-themeBorder text-[10px] font-mono") Delete
            div(class="flex justify-between items-center py-0.5")
              span Deselect
              span(class="bg-themeBackground px-2 py-0.5 rounded-md border border-themeBorder text-[10px] font-mono") Escape
    
      // Main content
      div(class="flex-1 relative")
        
        // Flow execution status panel (when flow is running) - MOVED TO BOTTOM
        FlowPanelExecutionPanel(
          :activeFlowExecution="activeFlowExecution"
          :showFlowStatusPanel="showFlowStatusPanel"
          :isFlowRunning="isFlowRunning"
          :flowProgress="flowProgress"
          :isInspectorOpen="isInspectorOpen"
          :concurrentExecutionInfo="getConcurrentExecutionInfo"
          :recoveryQueueCount="durableRecoveryRuns.length"
          :recoveryDecisionPending="recoveryDecisionPending"
          :recoveryDecisionError="recoveryDecisionError"
          @cancelFlow="cancelFlowUnified"
          @showHistory="showExecutionHistory = true"
          @closePanel="closeFlowStatusPanel"
          @failRecovery="failActiveRecovery"
          @resumeRecovery="resumeActiveRecovery"
        )
  
        // Execution History Modal
        FlowModalExecutionHistoryModal(
          :show="showExecutionHistory"
          :flowId="currentFlowMetadata.id"
          :activeFlowExecution="activeFlowExecution"
          @close="showExecutionHistory = false"
        )

        FlowModalRuntimeOperationsModal(
          :show="showRuntimeOperations"
          @close="showRuntimeOperations = false"
        )

        FlowModalPreparationModal(
          :show="showPreparationModal"
          :mode="preparationMode"
          :report="rehearsalReport"
          :importPreview="recipeImportPreview"
          :loading="preparationLoading"
          :error="preparationError"
          @close="closePreparationModal"
          @runRealFlow="runPreparedFlow"
          @confirmImport="confirmRecipeImport"
        )
  
        // Node palette - Simplified with unified backend data
        FlowPanelNodeLibrary(
          :show="showNodeLibrary"
          :triggerNodes="triggerCategoryNodeTypes"
          :controlNodes="controlCategoryNodeTypes"
          :aiNodes="aiCategoryNodeTypes"
          :audioNodes="audioCategoryNodeTypes"
          :wasmPlugins="dynamicCategoryNodeTypes"
          :nodeTypeColors="nodeTypeColors"
          :flows="allFlowsWithTemporary"
          :currentFlowId="currentFlowMetadata.id"
          :nodes="nodes"
          @close="showNodeLibrary = false"
          @dragStart="onDragStart"
          @addNode="addNodeOfType"
          @loadFlow="loadFlowFromDatabase"
          @deleteFlow="deleteFlowFromDatabase"
          @createNewFlow="createNewFlow"
          @selectVariable="onSelectVariable"
          @highlightNode="onHighlightNode"
        )
        // Flowchart canvas
        FlowCanvas(
          ref="canvasRef"
          :nodes="nodes"
          :connections="connections"
          :nodeTypeColors="nodeTypeColors"
          :camera="camera"
          :selectedNodes="selectedNodes"
          :selectedNode="selectedNode"
          :isDragging="isDragging"
          :isResizing="isResizing"
          :resizingNodeId="resizeNode?.id || null"
          :isSelecting="isSelecting"
          :isConnecting="isConnecting"
          :connectionStart="connectionStart"
          :connectionEnd="connectionEnd"
          :hoveredFlowPort="hoveredFlowPort"
          :hoveredConnection="hoveredConnection"
          :selectionBoxStyle="selectionBoxStyle"
          :gridStyle="gridStyle"
          :connectionPaths="connectionPaths"
          :validConnections="validConnections"
          :getNodeStatusClasses="getNodeStatusClasses"
          :getNodeStatusTitle="getNodeStatusTitle"
          :getNodeExecutionTime="getNodeExecutionTime"
          :getNodeRetryCount="getNodeRetryCount"
          :isNodeRunningConcurrently="isNodeRunningConcurrently"
          :getAiModelName="getAiModelName"
          :getAiModelShortName="getAiModelShortName"
          :getTemplateName="getTemplateName"
          :getTemplateShortName="getTemplateShortName"
          :isTextOnlyNode="isTextOnlyNode"
          :formatExecutionTime="formatExecutionTime"
          :nodeProgress="nodeProgress"
          :getConnectionColor="getConnectionColor"
          :getConnectionStrokeWidth="getConnectionStrokeWidth"
          :getAnimationDuration="getAnimationDuration"
          :getConnectionMidpoint="getConnectionMidpoint"
          @drop="onDrop"
          @mouseMove="handleMouseMove"
          @mouseUp="handleMouseUp"
          @startPan="startPan"
          @canvasContextMenu="handleCanvasContextMenu"
          @clickOutside="handleClickOutside"
          @zoom="handleZoom"
          @cancelConnection="cancelConnection"
          @unselectNodes="unselectNodes"
          @startDrag="startDrag"
          @startResize="startResize"
          @startConnection="startConnection"
          @deleteNode="handleDeleteClick"
          @toggleMinimized="toggleNodeMinimized"
          @aiModelClick="handleAiModelClick"
          @showTriggerChangeMenu="showTriggerChangeMenu"
          @deleteConnection="deleteConnection"
          @updateHoveredFlowPort="hoveredFlowPort = $event"
          @updateHoveredConnection="hoveredConnection = $event"
          @openInspector="handleOpenInspector"
        )

        FlowPanelMinimap(
          class="absolute z-[1001] transition-all duration-200"
          :class="(isInspectorOpen ? 'right-[26rem]' : 'right-5') + ' ' + ((showFlowStatusPanel && activeFlowExecution) ? 'bottom-[7.2rem]' : 'bottom-5')"
          :nodes="nodes"
          :connections="validConnections"
          :camera="camera"
          :viewport="canvasViewport"
          @panTo="handleMinimapPanTo"
        )

        button(
          class="flow-inspector-toggle absolute top-1/2 -translate-y-1/2 z-[1001] transition-all duration-200"
          :class="isInspectorOpen ? 'right-[24.5rem] flow-inspector-toggle--open' : 'right-0 flow-inspector-toggle--closed'"
          :title="isInspectorOpen ? 'Hide Inspector (I)' : 'Show Inspector (I)'"
          @click.stop="toggleInspector"
        )
          span {{ isInspectorOpen ? '>' : '<' }}

        div(
          class="flow-inspector-dock absolute top-0 right-0 h-full w-[24.5rem] max-w-[90vw] z-[1000] shadow-2xl transition-transform duration-200"
          :class="isInspectorOpen ? 'translate-x-0' : 'translate-x-full pointer-events-none'"
        )
          FlowPanelNodeInspector(
            :visible="isInspectorOpen"
            :node="inspectorNode"
            @close="hideInspector"
            @inputChanged="handleInspectorInputChanged"
            @executionConfigChanged="handleInspectorExecutionConfigChanged"
            @aiConfigRequested="handleInspectorAiConfig"
            @actionReferenceChanged="handleInspectorActionReferenceChanged"
            @actionPreflightChanged="handleInspectorActionPreflightChanged"
          )
  
        // Node selection menu that appears when connecting
        div(
          v-if="showNodeOptions"
          class="fixed bg-themeBackground2 p-4 rounded-2xl shadow-2xl z-50 node-menu border border-themeBorder"
          :style="smartMenuPosition"
        )
          div(class="flex items-center gap-3 mb-4 pb-3 border-b border-themeBorder")
            div(class="w-6 h-6 bg-themePrimary/15 border border-themePrimary/30 rounded-md flex items-center justify-center")
              span(class="text-themePrimary text-xs font-medium") ➕
            h3(class="text-sm font-bold text-themeText") Add New Node
          
          // Search bar for node selection menu
          div(class="mb-4 relative")
            input(
              v-auto-focus
              type="text"
              v-model="nodeSelectionSearch"
              @keydown.enter.prevent="selectFirstSearchMatch"
              placeholder="Search nodes..."
              class="w-full px-3 py-2 pr-8 bg-themeBackground border border-themeBorder rounded-xl text-themeText text-sm placeholder-themeText3 focus:outline-none focus:border-themePrimary focus:ring-1 focus:ring-themePrimary/20"
            )
            button(
              v-if="nodeSelectionSearch"
              @click="nodeSelectionSearch = ''"
              class="absolute right-2.5 top-1/2 transform -translate-y-1/2 text-themeText3 hover:text-themeText transition-colors"
            )
              span(class="text-xs") ✕
          
          div(class="flow-node-picker-cascade" ref="nodePickerCascadeRef")
            template(v-if="isSelectionSearchMode")
              div(class="flow-node-picker-column flow-node-picker-column--nodes custom-scrollbar-always")
                div(class="flow-node-picker-title flex items-center justify-between")
                  span Results
                  span(class="flow-node-picker-pill") {{ searchMatchedNodes.length }}
                template(v-if="searchMatchedNodes.length > 0")
                  button(
                    v-for="type in searchMatchedNodes"
                    :key="type.name"
                    class="flow-node-picker-node w-full"
                    @click="selectNodeFromList(type.name)"
                  )
                    div(class="flow-node-picker-node-dot" :style="{ backgroundColor: nodeTypeColors[type.name] || '#6B7280' }")
                    span(class="flow-node-picker-node-label") {{ type.label }}
                    span(class="flow-node-picker-node-add") +
                div(v-else class="flow-node-picker-empty") No matching nodes

            template(v-else)
              div(
                class="flow-node-picker-column flow-node-picker-column--categories custom-scrollbar-always"
                @scroll="nodeMenuAlignTick++"
              )
                div(class="flow-node-picker-title") Categories
                button(
                  v-for="category in selectionMenuCategories"
                  :key="category.id"
                  class="flow-node-picker-item w-full"
                  :data-category-id="category.id"
                  :class="activeSelectionCategoryId === category.id ? 'is-active' : ''"
                  :ref="(el) => setCategoryItemRef(category.id, el)"
                  @click="handleSelectCategory(category.id)"
                )
                  span(class="flow-node-picker-item-icon") {{ category.icon }}
                  span(class="flow-node-picker-item-label") {{ category.label }}
                  span(class="flow-node-picker-item-chevron") ›

              div(
                v-if="showSubgroupPopout"
                class="flow-node-picker-popout flow-node-picker-popout--level2"
                :style="level2PopoutStyle"
              )
                div(
                  class="flow-node-picker-column flow-node-picker-column--subcategories custom-scrollbar-always"
                  @scroll="nodeMenuAlignTick++"
                )
                  div(class="flow-node-picker-title") {{ activeSelectionCategory?.label || 'Groups' }}
                  template(v-if="activeCategorySubgroups.length > 0")
                    button(
                      v-for="group in activeCategorySubgroups"
                      :key="group.id"
                      class="flow-node-picker-item w-full"
                      :data-subgroup-id="group.id"
                      :class="activeSelectionSubgroupId === group.id ? 'is-active' : ''"
                      :ref="(el) => setSubgroupItemRef(group.id, el)"
                      @click="handleSelectSubgroup(group.id)"
                    )
                      span(class="flow-node-picker-item-label") {{ group.label }}
                      span(class="flow-node-picker-item-count") {{ group.items.length }}
                  div(v-else class="flow-node-picker-empty") No groups

              div(
                v-if="showNodesPopoutLevel2"
                class="flow-node-picker-popout flow-node-picker-popout--level2"
                :style="level2PopoutStyle"
              )
                div(class="flow-node-picker-column flow-node-picker-column--nodes custom-scrollbar-always")
                  div(class="flow-node-picker-title flex items-center justify-between")
                    span {{ activeSelectionCategory?.label || 'Nodes' }}
                    span(class="flow-node-picker-pill") {{ visibleSelectionNodes.length }}
                  template(v-if="visibleSelectionNodes.length > 0")
                    button(
                      v-for="type in visibleSelectionNodes"
                      :key="type.name"
                      class="flow-node-picker-node w-full"
                      @click="selectNodeFromList(type.name)"
                    )
                      div(class="flow-node-picker-node-dot" :style="{ backgroundColor: nodeTypeColors[type.name] || '#6B7280' }")
                      span(class="flow-node-picker-node-label") {{ type.label }}
                      span(class="flow-node-picker-node-add") +
                  div(v-else class="flow-node-picker-empty") No matching nodes

              div(
                v-if="showNodesPopoutLevel3"
                class="flow-node-picker-popout flow-node-picker-popout--level3"
                :style="level3PopoutStyle"
              )
                div(class="flow-node-picker-column flow-node-picker-column--nodes custom-scrollbar-always")
                  div(class="flow-node-picker-title flex items-center justify-between")
                    span {{ activeSelectionSubgroup?.label || 'Nodes' }}
                    span(class="flow-node-picker-pill") {{ visibleSelectionNodes.length }}
                  template(v-if="visibleSelectionNodes.length > 0")
                    button(
                      v-for="type in visibleSelectionNodes"
                      :key="type.name"
                      class="flow-node-picker-node w-full"
                      @click="selectNodeFromList(type.name)"
                    )
                      div(class="flow-node-picker-node-dot" :style="{ backgroundColor: nodeTypeColors[type.name] || '#6B7280' }")
                      span(class="flow-node-picker-node-label") {{ type.label }}
                      span(class="flow-node-picker-node-add") +
                  div(v-else class="flow-node-picker-empty") No matching nodes
  
        // Enhanced Trigger change menu
        div(
          v-if="showTriggerOptions && triggerChangeNode"
          class="fixed bg-themeBackground2/95 backdrop-blur-xl p-5 rounded-2xl shadow-2xl z-50 trigger-menu border border-themeBorder"
          :style="{ left: triggerMenuPosition.x + 'px', top: triggerMenuPosition.y + 'px' }"
        )
          div(class="flex items-center gap-3 mb-4 pb-3 border-b border-themeBorder")
            div(class="w-6 h-6 bg-themePrimary/15 border border-themePrimary/30 rounded-md flex items-center justify-center")
              span(class="text-themePrimary text-xs font-medium") 🔄
            h3(class="text-sm font-bold text-themeText") Change Trigger Type
          div(class="space-y-1 mb-4")
            div(
              v-for="type in allTriggerTypes"
              :key="type.name"
              :class="['group p-2.5 rounded-xl flex items-center gap-2.5 cursor-pointer transition-colors duration-200 border', type.name === triggerChangeNode.type ? 'bg-themePrimary/15 border-themePrimary/50 text-themePrimary' : 'bg-themeBackground hover:bg-themeBackground3 border-themeBorder text-themeText']"
              @click="changeTriggerType(type)"
            )
              div(class="w-3 h-3 rounded-sm" :style="{ backgroundColor: nodeTypeColors[type.name] || 'hsl(var(--primary))' }")
              span(class="text-xs font-medium") {{ type.label }}
              span(v-if="type.name === triggerChangeNode.type" class="text-xs bg-themePrimary/20 text-themePrimary ml-auto px-2 py-0.5 rounded-full border border-themePrimary/30") Current
              span(v-else class="ml-auto text-themeText3 group-hover:text-themeText text-xs") ↻
          div(class="flex justify-end gap-2 pt-2")
            button(
              class="px-3.5 py-1.5 bg-themeBackground hover:bg-themeBackground3 text-themeText text-xs font-semibold rounded-xl transition-colors duration-200 border border-themeBorder cursor-pointer shadow-2xs"
              @click="closeTriggerChangeMenu"
            ) Cancel
  
      // AI Model & Template Selector Modal
      div(
        v-if="showAiModelSelector"
        class="fixed inset-0 bg-black/50 backdrop-blur-sm z-[9999] flex items-center justify-center"
        @click="closeAiModelSelector"
      )
        div(
          class="bg-themeBackground2/95 backdrop-blur-xl rounded-2xl shadow-2xl border border-themeBorder p-6 max-w-lg w-full mx-4"
          @click.stop=""
        )
          div(class="flex items-center gap-3 mb-6 pb-4 border-b border-themeBorder")
            div(class="w-8 h-8 bg-themePrimary/15 border border-themePrimary/30 rounded-xl flex items-center justify-center")
              span(class="text-themePrimary text-sm font-bold") 🤖
            h3(class="text-base font-bold text-themeText") Configure AI Node
            
          // Tab Navigation
          div(class="flex border-b border-themeBorder mb-6")
            button(
              class="px-4 py-2 text-xs font-bold rounded-t-xl transition-colors cursor-pointer"
              :class="activeConfigTab === 'model' ? 'bg-themePrimary/15 text-themePrimary border-b-2 border-themePrimary' : 'text-themeText3 hover:text-themeText'"
              @click="activeConfigTab = 'model'"
            ) Model Selection
            button(
              v-if="isTextOnlyNode(selectedAiNode)"
              class="px-4 py-2 text-xs font-bold rounded-t-xl transition-colors ml-2 cursor-pointer"
              :class="activeConfigTab === 'template' ? 'bg-themePrimary/15 text-themePrimary border-b-2 border-themePrimary' : 'text-themeText3 hover:text-themeText'"
              @click="activeConfigTab = 'template'"
            ) Prompt Template
          
          // Model Selection Tab
          div(v-if="activeConfigTab === 'model'" class="mb-6")
            div(v-if="availableAiModels.length === 0" class="p-4 bg-amber-500/10 border border-amber-500/30 rounded-xl text-center")
              p(class="text-amber-400 text-sm font-medium") No AI models available
              p(class="text-amber-400/80 text-xs mt-1") Add providers and refresh models in Settings
            div(v-else class="space-y-3 max-h-[calc(60vh-30px)] overflow-y-auto")
              div(
                v-for="model in availableAiModels"
                :key="model.id"
                class="group p-3 bg-themeBackground rounded-xl hover:bg-themeBackground3 cursor-pointer transition-colors border border-themeBorder hover:border-themePrimary/50"
                :class="selectedAiNode?.aiModel === model.id ? 'bg-themePrimary/15 border-themePrimary/60' : ''"
                @click="selectAiModel(model.id)"
              )
                div(class="flex items-center justify-between")
                  div(class="flex-1")
                    div(class="text-themeText font-medium text-xs") {{ model.name }}
                    div(class="flex items-center gap-2 text-themeText3 text-[11px] mt-1")
                      span(class="px-1.5 py-0.5 bg-themeBackground2 rounded border border-themeBorder") {{ model.provider }}
                      span(v-if="model.contextWindow") {{ (model.contextWindow / 1000).toFixed(0) }}K context
                  div(v-if="selectedAiNode?.aiModel === model.id" class="text-themePrimary ml-2 font-bold") ✓
          
          // Template Selection Tab (only for Text-Only nodes)
          div(v-if="activeConfigTab === 'template' && isTextOnlyNode(selectedAiNode)" class="space-y-3 mb-6 max-h-[calc(60vh-30px)] overflow-y-auto")
            div(class="text-xs text-themeText3 mb-4") 
              | Choose how this AI node interprets inputs and generates responses:
            
            // Template Categories
            div(v-for="category in templateCategories" :key="category" class="mb-4")
              h4(class="text-[11px] font-bold text-themeText3 uppercase tracking-wider mb-2") {{ category }}
              div(class="space-y-2")
                div(
                  v-for="template in getTemplatesByCategory(category)"
                  :key="template.id"
                  class="group p-3 bg-themeBackground rounded-xl hover:bg-themeBackground3 cursor-pointer transition-colors border border-themeBorder hover:border-themePrimary/50"
                  :class="selectedAiNode?.promptTemplate === template.id ? 'bg-themePrimary/15 border-themePrimary/60' : ''"
                  @click="selectTemplate(template.id)"
                )
                  div(class="flex items-center justify-between")
                    div(class="flex-1")
                      div(class="text-themeText font-medium text-xs") {{ template.name }}
                      div(class="text-themeText3 text-[11px] mt-1") {{ template.description }}
                    div(v-if="selectedAiNode?.promptTemplate === template.id" class="text-themePrimary ml-2 font-bold") ✓
                
          div(class="flex justify-end gap-2.5")
            button(
              class="px-4 py-2 bg-themeBackground hover:bg-themeBackground3 border border-themeBorder text-themeText text-xs font-semibold rounded-xl transition-colors cursor-pointer shadow-2xs"
              @click="closeAiModelSelector"
            ) Cancel
            button(
              class="px-4 py-2 bg-themeButton hover:bg-themeButtonHover text-white text-xs font-semibold rounded-xl transition-colors cursor-pointer shadow-xs"
              @click="saveAiConfiguration"
            ) Save Configuration

      // Flow Validation Error Modal
      div(
        v-if="showValidationErrors && validationResult"
        class="fixed inset-0 bg-black/50 backdrop-blur-sm z-[9999] flex items-center justify-center"
        @click="closeValidationModal"
      )
        div(
          class="bg-themeBackground2 rounded-2xl shadow-2xl border border-themeBorder p-6 max-w-2xl w-full mx-4"
          @click.stop=""
        )
          div(class="flex items-center gap-3 mb-6 pb-4 border-b border-themeBorder")
            div(class="w-8 h-8 bg-amber-500/15 border border-amber-500/30 rounded-xl flex items-center justify-center")
              span(class="text-amber-400 text-sm font-bold") ⚠️
            h3(class="text-base font-bold text-themeText") Flow Validation {{ validationResult.errors.length > 0 ? 'Errors' : 'Warnings' }}
            
          div(class="space-y-4 mb-6")
            // Errors
            div(v-if="validationResult.errors.length > 0")
              h4(class="text-xs font-bold text-red-400 uppercase tracking-wider mb-2") Errors (must be fixed):
              div(class="space-y-2")
                div(
                  v-for="error in validationResult.errors"
                  :key="error"
                  class="bg-red-500/10 border border-red-500/25 rounded-xl p-3 text-xs text-red-400"
                )
                  div(class="flex items-start gap-2")
                    span(class="text-red-400 mt-0.5") •
                    span {{ error }}
            
            // Warnings
            div(v-if="validationResult.warnings.length > 0")
              h4(class="text-xs font-bold text-amber-400 uppercase tracking-wider mb-2") Warnings (recommended to fix):
              div(class="space-y-2")
                div(
                  v-for="warning in validationResult.warnings"
                  :key="warning"
                  class="bg-amber-500/10 border border-amber-500/25 rounded-xl p-3 text-xs text-amber-400"
                )
                  div(class="flex items-start gap-2")
                    span(class="text-amber-400 mt-0.5") •
                    span {{ warning }}
                
          div(class="flex justify-end gap-2.5")
            button(
              class="px-4 py-2 bg-themeBackground hover:bg-themeBackground3 border border-themeBorder text-themeText text-xs font-semibold rounded-xl transition-colors cursor-pointer shadow-2xs"
              @click="closeValidationModal"
            ) Close
            
            // Show export anyway button if this is an export validation with only warnings
            button(
              v-if="pendingExportCallback && validationResult.errors.length === 0"
              class="px-4 py-2 bg-amber-500 hover:bg-amber-600 text-white text-xs font-semibold rounded-xl transition-colors cursor-pointer shadow-xs"
              @click="continueWithExport"
            ) Export Anyway
            
            // Show save anyway button if this is a save validation with only warnings
            button(
              v-if="pendingSaveCallback && validationResult.errors.length === 0"
              class="px-4 py-2 bg-themeButton hover:bg-themeButtonHover text-white text-xs font-semibold rounded-xl transition-colors cursor-pointer shadow-xs"
              @click="continueWithSave"
            ) Save Anyway
    
    // Unsaved Changes Confirmation Modal
    FlowModalUnsavedChangesModal(
      :show="showUnsavedChangesModal"
      :message="getConfirmationMessage"
      :confirmButtonText="getConfirmationButtonText"
      @confirm="confirmLoadFlow"
      @cancel="cancelLoadFlow"
    )
    
    // Edit Flow Metadata Modal
    FlowModalEditFlowMetadataModal(
      :show="showEditFlowMetadataModal"
      :currentName="currentFlowMetadata.name"
      :currentDescription="currentFlowMetadata.description"
      @save="saveFlowMetadata"
      @cancel="showEditFlowMetadataModal = false"
    )
    
    BaseModal(:is-open="showRunInputs" @close="showRunInputs = false")
      div(class="vd-modal-card p-5 max-h-[85vh] overflow-auto w-[min(34rem,90vw)] bg-themeBackground2 text-themeText")
        WidgetSettingsForm(
          :schema="runInputSchema"
          :config="runInputDefaults"
          :busy="flowStartPending"
          :error="runInputError"
          heading="Flow inputs"
          submit-label="Run flow"
          busy-label="Starting…"
          @apply="submitFlowInputs"
        )

    // Save Flow Modal
    FlowModalSaveFlowModal(
      :show="showSaveFlowModal"
      :isNewFlow="currentFlowMetadata.isNew"
      :currentName="currentFlowMetadata.name"
      :currentDescription="currentFlowMetadata.description"
      @save="handleSaveFlowFromModal"
      @cancel="handleCancelSaveFlowModal"
    )

</template>

<script setup lang="ts">
import { ref, onMounted, computed, onUnmounted, nextTick, watch } from "vue";
import SolarSettingsBold from "~icons/solar/settings-bold";
import SolarCloseCircleBold from "~icons/solar/close-circle-bold";
import HugeiconsCommandLine from "~icons/hugeicons/command-line";
import { invoke } from "@tauri-apps/api/core";
import { listen, emitTo } from "@tauri-apps/api/event";
import { registerCoreWidgets } from "../../composables/useCoreWidgets";
import { useDesktopUtils } from "../../../desktop/stores/desktopUtils";
import { useMainStore } from "../../../base/stores/Main";
import { useAIModelsStore } from "../../../base/stores/AIModels";
import { useFlowManager } from "../../../desktop/composables/useFlowManager";
import ActionInputContractEditor from '../../../desktop/components/ActionInputContractEditor.vue';
import WidgetSettingsForm from '../../../base/components/WidgetSettingsForm.vue';
import BaseModal from '../../../desktop/components/Modal/BaseModal.vue';
import type { InputContractSnapshot, InvocationScalar } from '@base/types/actionInputs';
import { useFlowUtils } from "../../composables/useFlowUtils";
import { useFlowNodes } from "../../composables/useFlowNodes";
import { useFlowConnections } from "../../composables/useFlowConnections";
import { useFlowValidation } from "../../composables/useFlowValidation";
import {
  DEFAULT_FLOW_OUTPUT_ID,
  canvasConnectionToFlowConnection,
  flowConnectionToCanvasConnection,
  getNamedFlowRouteOutputs,
  normalizeFlowConnections,
} from "../../utils/portSemantics";
import {
  buildNodeCatalog,
  canonicalNodeCategory,
  validateNodeCatalog,
} from "../../utils/nodeCatalog";
import {
  buildActionReferenceNodeType,
  parseFlowDropPayload,
} from "../../utils/actionReferenceDrop";
import {
  assertSupportedFlowDocument,
  buildFlowDocument,
  createEditorNodeIdProjection,
  flowRevision,
  flowRevisionPayload,
  persistedEditorNodeId,
} from "../../utils/flowSerialization";
import { applyNodePreviewResults } from "../../utils/nodePreviewResults";
import { buildReadOnlyRehearsal } from "../../utils/flowPreparation";
import type { FlowRehearsalReport } from "../../utils/flowPreparation";
import {
  buildFlowRecipe,
  isFlowRecipe,
  parseFlowRecipe,
} from "../../utils/flowRecipe";
import type {
  FlowActionHealth,
  FlowRecipeImportPreview,
} from "../../utils/flowRecipe";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type {
  FlowDefinition,
  FlowNodeDefinition,
  FlowCanvasConnection,
  AiNodeBehavior,
  FlowExecutionState,
  NodeExecutionConfig,
} from "../../types/flow";

// Disable transitions and attribute inheritance to avoid Vue warnings
// (Pug template has leading whitespace that creates a text node fragment)
definePageMeta({
  layout: "flow-editor",
  pageTransition: false,
  layoutTransition: false,
});

defineOptions({
  inheritAttrs: false,
});

const desktopUtils = useDesktopUtils();
const mainStore = useMainStore();
const flowManager = useFlowManager();
const route = useRoute();

// Get current window ID for window-specific notifications
const currentWindowId = "flow_editor";

// Store unlisten functions for cleanup
let unlistenMcpConnected: (() => void) | null = null;
let unlistenMcpDisconnected: (() => void) | null = null;
let unlistenPluginChanged: (() => void) | null = null;

// Destructure reactive state from flowManager for proper reactivity
const {
  currentFlow,
  savedFlows,
  activeFlowExecution,
  isLoading,
  error,
  isExecutorInitialized,
  isFlowRunning,
  canExecuteFlow,
  flowProgress,
  nodeProgress,
  saveFlow: saveFlowUnified,
  loadFlow: loadFlowUnified,
  listFlows: listFlowsUnified,
  deleteFlow: deleteFlowUnified,
  executeFlow: executeFlowUnified,
  cancelFlow: cancelFlowUnified,
  exportFlow,
  importFlow,
} = flowManager;

// Destructure utility functions from useFlowUtils
const {
  formatCategoryName,
  groupItemsByCategory,
  createSearchFilter,
  getAiModelName: utilGetAiModelName,
  getAiModelShortName: utilGetAiModelShortName,
  formatTimestamp,
  formatLogDetails,
  formatResultOutput,
} = useFlowUtils();

// Type definitions
interface Node {
  id: number;
  type: string;
  text: string;
  x: number;
  y: number;
  width?: number; // Custom node width
  height?: number; // Custom node height
  minWidth?: number; // Minimum width constraint (set at creation, stable across resizes)
  minHeight?: number; // Minimum height constraint (set at creation, stable across resizes)
  inputs?: NodeIO[];
  outputs?: NodeIO[];
  isTool?: boolean;
  toolName?: string;
  parameters?: any;
  // AI Node specific properties
  aiBehavior?: AiNodeBehavior;
  isAiNode?: boolean;
  aiModel?: string; // Selected AI model ID
  promptTemplate?: string; // Selected prompt template ID for text-only nodes
  executionState?: ExecutionState;
  executionConfig: NodeExecutionConfig;
  invocationMode?: boolean; // Compact output-only view for a referenced action
  minimized?: boolean; // Header-only UI mode
  persistedDefinition?: FlowNodeDefinition; // Same-version fields not directly edited by the canvas
  actionPreflight?: import("@base/types/actionReference").ActionPreflight;
}

type Connection = FlowCanvasConnection;

interface Point {
  x: number;
  y: number;
}

interface NodeIO {
  id: string;
  name: string;
  type: string;
  value?: any;
  attached?: boolean;
  enum?: string[];
  required?: boolean; // Whether this input is required (defaults to true for backwards compatibility)
  forceInput?: boolean; // If true, this input MUST be a connection (no widget shown)
  isDynamicType?: boolean; // If true, this output's type adapts at runtime (allows "any" to connect to specific types)
  displayOptions?: {
    show?: Record<string, any[]>; // Show this field when other fields match these values
    hide?: Record<string, any[]>; // Hide this field when other fields match these values
  };
  // Widget metadata for UI controls
  widget?: string; // 'slider', 'color', 'toggle', 'file', 'textarea', 'select', 'text', 'number'
  widgetOptions?: {
    min?: number;
    max?: number;
    step?: number;
    accept?: string[]; // File types for file picker
    multiline?: boolean; // For text areas
    rows?: number; // Number of rows for textarea
    options?: string[]; // Options for select/multiselect widgets
  };
}

// Local execution state interface (specific to flow.vue)
interface ExecutionState {
  status: "idle" | "running" | "completed" | "error";
  last_execution?: Date;
  last_inputs?: Record<string, any>; // Actual input values used during execution (after connection resolution)
  last_output?: any; // Generic output type
  error_message?: string;
}

// Simplified: Use ONLY backend nodes as single source of truth
// Remove all separate tool fetching - backend provides everything
const nodeTypes = ref<any[]>([]);
const backendFlowNodes = ref<any[]>([]);
const nodes = ref<Node[]>([]);
const connections = ref<Connection[]>([]);

const sanitizeNodeTypeItems = (items: any[]): any[] =>
  (Array.isArray(items) ? items : []).filter(
    (item) => item && typeof item === "object" && typeof item.name === "string"
  );

const getSafeNodeTypes = (): any[] => sanitizeNodeTypeItems(nodeTypes.value);

const hasExistingTriggerNode = (): boolean => {
  const triggerNodeTypeNames = new Set(
    getSafeNodeTypes()
      .filter((type) => canonicalNodeCategory(type.category) === "TRIGGER")
      .map((type) => type.name)
  );

  if (triggerNodeTypeNames.size === 0) return false;
  return nodes.value.some((node) => triggerNodeTypeNames.has(node.type));
};

// ... existing node conversion functions ...

// Load ALL items from backend - single source of truth
const loadBackendNodes = async () => {
  try {
    console.log("🔄 Loading all nodes from backend...");
    const result = await invoke("retrieve_all_flow_nodes");
    backendFlowNodes.value = result as any[];

    // Convert backend nodes to frontend format
    const convertedNodes = backendFlowNodes.value.map(convertBackendNodeToFrontend);
    const catalogErrors = validateNodeCatalog(convertedNodes);
    if (catalogErrors.length > 0) {
      console.error("Invalid flow-node catalog:", catalogErrors);
    }

    // Use only backend nodes - no other sources needed
    nodeTypes.value = convertedNodes;

    console.log("✅ Loaded all nodes from backend:", nodeTypes.value.length);
    console.log(
      "📋 All available items:",
      nodeTypes.value.map((n) => `${n.name} (${n.label}) [${n.category}]`)
    );
  } catch (error) {
    console.error("❌ Failed to load backend nodes:", error);
    nodeTypes.value = [];
  }
};

// Simplified node type colors - only from backend data
const nodeTypeColors = computed(() => {
  const colors: Record<string, string> = {};
  getSafeNodeTypes().forEach((type) => {
    colors[type.name] = type.color;
  });
  // Local virtual nodes used for Deck -> Flow drag/drop
  colors["invoke-action"] = "#d946ef";
  colors["assignment-trigger"] = "#06b6d4";
  return colors;
});

// Clean categorization - all from single source
const triggerCategoryNodeTypes = computed(() => {
  const triggers = getSafeNodeTypes().filter(
    (type) => canonicalNodeCategory(type.category) === "TRIGGER"
  );
  const hasTrigger = hasExistingTriggerNode();
  return hasTrigger ? [] : triggers;
});

const audioCategoryNodeTypes = computed(() => {
  return getSafeNodeTypes().filter(
    (type) => canonicalNodeCategory(type.category) === "AUDIO"
  );
});

const controlCategoryNodeTypes = computed(() => {
  return getSafeNodeTypes().filter(
    (type) => canonicalNodeCategory(type.category) === "CONTROL"
  );
});

const aiCategoryNodeTypes = computed(() => {
  return getSafeNodeTypes().filter(
    (type) => canonicalNodeCategory(type.category) === "AI"
  );
});

// Dynamic categories - everything that's not a standard category
const dynamicCategoryNodeTypes = computed(() => {
  const standardCategories = ["TRIGGER", "AUDIO", "CONTROL", "AI"];
  return getSafeNodeTypes().filter(
    (type) => !standardCategories.includes(canonicalNodeCategory(type.category))
  );
});

// Group dynamic categories for display
const groupedDynamicCategories = computed(() => {
  return groupItemsByCategory(dynamicCategoryNodeTypes.value, "category", "name");
});

// ... existing code ...

// Removed legacy tool fetching - all data comes from backend nodes
import { convertJsonSchemaToNodeInputs } from "../../utils/schemaInputs";

// Convert JSON Schema to frontend input format
const convertSchemaToInputs = (schema: any): NodeIO[] => {
  return convertJsonSchemaToNodeInputs(schema, {
    defaultValueResolver: getDefaultValueForProperty,
  }) as NodeIO[];
};

// Get default value for property type

// Convert backend node info to frontend format
const isValueDrivenNodeType = (nodeType: string): boolean =>
  nodeType === "get-variable" || nodeType === "set-variable";

const ensureVariableNodeInputs = (nodeType: string, inputs: NodeIO[]): NodeIO[] => {
  const nextInputs = [...inputs];

  if (nodeType === "set-variable") {
    const hasValueType = nextInputs.some((input) => input.id === "valueType");
    if (!hasValueType) {
      nextInputs.push({
        id: "valueType",
        name: "Value Type",
        type: "string",
        value: "any",
        enum: ["any", "string", "number", "boolean", "object", "array"],
        required: false,
        attached: false,
        widget: "select",
        widgetOptions: {
          options: ["any", "string", "number", "boolean", "object", "array"],
        },
      });
    }
  }

  return nextInputs;
};

const convertBackendNodeToFrontend = (toolNodeInfo: any) => {
  const inputs = ensureVariableNodeInputs(
    toolNodeInfo.id,
    convertSchemaToInputs(toolNodeInfo.parameters)
  );

  // Create one implicit default control output for ordinary nodes. Nodes that
  // publish explicit control routes own their routing surface instead.
  const outputs: NodeIO[] = [];
  const backendOutputs = Array.isArray(toolNodeInfo.outputs) ? toolNodeInfo.outputs : [];
  const hasNamedFlowRoutes = backendOutputs.some(
    (output: any) =>
      output.output_type === "flow" && output.id !== DEFAULT_FLOW_OUTPUT_ID
  );

  // Value-driven nodes are connected via value ports only.
  if (!isValueDrivenNodeType(toolNodeInfo.id) && !hasNamedFlowRoutes) {
    outputs.push({ id: DEFAULT_FLOW_OUTPUT_ID, name: "Output", type: "flow" });
  }

  // Add actual outputs from backend schema
  if (backendOutputs.length > 0) {
    for (const backendOutput of backendOutputs) {
      outputs.push({
        id: backendOutput.id,
        name: backendOutput.name,
        type: backendOutput.output_type,
      });
    }
  }

  // Use the actual backend node ID directly - no more mappings!
  const backendId = toolNodeInfo.id;

  // Determine if this is an AI node and set appropriate behavior
  const isAiNode = toolNodeInfo.type_name.toUpperCase() === "AI";

  let aiBehavior: string | undefined = undefined;

  if (isAiNode) {
    // Map AI node types to behaviors (must match Rust enum values)
    const aiBehaviorMap: Record<string, string> = {
      "ai-text-generator": "TextOnly", // Text-only mode - no tool access
      "hybrid-llm-free": "FreeAgent", // Full autonomy - can use any available tools
      "ai-orchestrator": "Orchestrator", // Orchestrator mode - restricted to connected tools
    };
    aiBehavior = aiBehaviorMap[backendId];
  }

  const convertedNode = {
    name: backendId, // Use backend ID as the identifier for execution
    label: toolNodeInfo.name, // Display name
    category: toolNodeInfo.type_name.toUpperCase(),
    category_display_name: toolNodeInfo.category_display_name,
    subcategory: toolNodeInfo.subcategory,
    color: toolNodeInfo.type_color,
    inputs,
    outputs,
    backendId: backendId, // Store backend ID for execution
    displayName: toolNodeInfo.name, // Store display name separately
    isBackendNode: true,
    isAiNode: isAiNode, // Mark AI nodes
    aiBehavior: aiBehavior, // Set AI behavior for proper routing
  };

  return convertedNode;
};

// Track which categories are expanded
const expandedCategories = ref<Record<string, boolean>>({
  TRIGGER: true,
  AUDIO: true,
  CONTROL: true,
  AI: true,
});

// Toggle category expansion
const toggleCategory = (category: string | number) => {
  const categoryKey = category.toString();
  expandedCategories.value[categoryKey] = !expandedCategories.value[categoryKey];
};

// Helper function to handle validation errors
const showValidationError = (errorMessage: string) => {
  validationResult.value = {
    isValid: false,
    errors: [errorMessage],
    warnings: [],
  };
  showValidationErrors.value = true;
};

// Update the onDrop method
const onDrop = (event: DragEvent) => {
  event.preventDefault();

  const canvas = document.getElementById("flowchart-canvas");
  if (!canvas || !event.dataTransfer) return;

  const rect = canvas.getBoundingClientRect();

  // Calculate position in canvas coordinates, considering camera transform
  const dropX = (event.clientX - rect.left - camera.value.x) / camera.value.scale;
  const dropY = (event.clientY - rect.top - camera.value.y) / camera.value.scale;

  // Try to get tool data first
  const jsonPayload = event.dataTransfer.getData("application/json");
  if (jsonPayload) {
    const deckPayload = parseFlowDropPayload(jsonPayload);

    // Deck -> Flow canonical action-reference drag/drop path.
    if (deckPayload?.event === "action-reference") {
      const invocationNodeType = buildActionReferenceNodeType(
        deckPayload,
        DEFAULT_FLOW_OUTPUT_ID,
      );
      const newNode = createNodeAtPosition(invocationNodeType, dropX, dropY);
      newNode.invocationMode = true;
      addNodeToCanvas(newNode);

      nextTick(() => {
        centerNodeOnPosition(newNode, dropX, dropY);
      });
      return;
    }

    // Keep mode payload non-destructive for now; flow trigger mapping will be added later.
    if (deckPayload?.event === "mode") {
      mainStore.addNotification(
        {
          title: "Mode Drop Not Mapped Yet",
          message: "Drag a macro/program/action item from Assignments for now.",
          type: "info",
          autoClose: true,
          duration: 3500,
          buttonText: "",
          buttonFunction: () => {},
          color: "bg-blue-500",
        },
        currentWindowId
      );
      return;
    }

    // Existing backend tool drag/drop path
    try {
      const tool = JSON.parse(jsonPayload);
      const newNode = createNodeAtPosition(tool, dropX, dropY, true);
      addNodeToCanvas(newNode);

      // Correct position after DOM update
      nextTick(() => {
        centerNodeOnPosition(newNode, dropX, dropY);
      });
      return;
    } catch (error) {
      console.error("Error parsing tool data:", error);
    }
  }

  // If no tool data, try to get node type
  const nodeType = event.dataTransfer.getData("nodeType");
  if (!nodeType) return;

  // Use the composable's addNodeOfType function (which handles validation)
  // But we need to override the position for drag & drop
  const nodeTypeInfo = nodeTypes.value.find((t) => t.name === nodeType);
  if (!nodeTypeInfo) return;

  // Check validation first - only prevent multiple trigger nodes
  const triggerNodes = findTriggerNodes();
  const isAddingTrigger = nodeTypes.value.some(
    (type) => type.name === nodeType && type.category === "TRIGGER"
  );

  if (isAddingTrigger && triggerNodes.length > 0) {
    const errorMessage = `Cannot drop trigger node "${nodeType}": Flow already has a trigger node "${triggerNodes[0]?.text || "Trigger"}". Only one trigger node is allowed per flow.`;
    showValidationError(errorMessage);
    console.warn(
      `Cannot drop trigger node ${nodeType}: A trigger node already exists in the flow`
    );
    return;
  }

  // Create node at drop position
  const newNode = createNodeAtPosition(nodeTypeInfo, dropX, dropY);
  addNodeToCanvas(newNode);

  // Correct position after DOM update
  nextTick(() => {
    centerNodeOnPosition(newNode, dropX, dropY);
  });
};

// Computed property to filter out invalid connections
const validConnections = computed(() => {
  return connections.value.filter((conn) => {
    // Ensure connection has valid types
    if (!conn.fromType || !conn.toType) {
      console.warn("Connection missing type information:", conn);
      return false;
    }

    // Ensure both nodes exist using utility function
    const fromNode = findNodeById(conn.from);
    const toNode = findNodeById(conn.to);

    if (!fromNode || !toNode) {
      console.warn("Connection references non-existent nodes:", conn);
      return false;
    }

    return true;
  });
});

const selectedNode = ref<Node | null>(null);
const selectedNodes = ref<Set<number>>(new Set());
const inspectorNode = ref<Node | null>(null);
const isInspectorOpen = ref(false);
const canvasRef = ref();
const canvasViewport = ref({ width: 1, height: 1 });
const isDragging = ref(false);
const dragOffset = ref({ x: 0, y: 0 });
const dragOccurred = ref(false);
const dragStartPosition = ref({ x: 0, y: 0 });
const baseDragThreshold = 6; // Base threshold in pixels at 1x zoom

// Resize state
const isResizing = ref(false);
const resizeNode = ref<Node | null>(null);
const resizeStartSize = ref({ width: 0, height: 0 });
const resizeStartMouse = ref({ x: 0, y: 0 });

// Connection interaction
const hoveredConnection = ref<number | null>(null);

// AI Model & Template selection
const showAiModelSelector = ref(false);
const selectedAiNode = ref<Node | null>(null);
const activeConfigTab = ref<"model" | "template">("model");

const toggleInspector = () => {
  isInspectorOpen.value = !isInspectorOpen.value;
};

const hideInspector = () => {
  isInspectorOpen.value = false;
};

const handleOpenInspector = (node: Node) => {
  selectedNode.value = node;
  if (!selectedNodes.value.has(node.id)) {
    selectedNodes.value.clear();
    selectedNodes.value.add(node.id);
  }
  inspectorNode.value = node;
  isInspectorOpen.value = true;
};

watch(selectedNode, (node) => {
  if (node) {
    inspectorNode.value = node;
    try {
      const { setContext } = useAppAutoContext();
      setContext(
        "flow_node",
        `Flow Node: ${node.text || node.type || `#${node.id}`}`,
        node,
        `Type: ${node.type}`
      );
    } catch (_) {}
  }
});

watch(
  () => nodes.value.map((node) => node.id),
  (nodeIds) => {
    if (inspectorNode.value && !nodeIds.includes(inspectorNode.value.id)) {
      inspectorNode.value = null;
    }
  }
);

// Use centralized AI models store
const aiModelsStore = useAIModelsStore();

// Get all models from all providers (computed to stay reactive with Pinia store)
const availableAiModels = computed(() => {
  const allModels: any[] = [];
  for (const [provider, models] of Object.entries(aiModelsStore.models)) {
    allModels.push(...models);
  }
  return allModels;
});

// Note: No need to listen for model updates - Pinia store automatically syncs

// Prompt templates loaded from backend
interface PromptTemplateSummary {
  id: string;
  name: string;
  description: string;
  category: string;
}

const availableTemplates = ref<PromptTemplateSummary[]>([]);

// Template utilities
const templateCategories = computed(() => {
  const categories = new Set(availableTemplates.value.map((t) => t.category));
  return Array.from(categories).sort();
});

const getTemplatesByCategory = (category: string) => {
  return availableTemplates.value.filter((t) => t.category === category);
};

const isTextOnlyNode = (node: Node | null) => {
  return node?.type === "ai-text-generator";
};

// Template utility functions
const getTemplateName = (templateId: string) => {
  const template = availableTemplates.value.find((t) => t.id === templateId);
  return template ? template.name : templateId;
};

const getTemplateShortName = (templateId: string) => {
  const template = availableTemplates.value.find((t) => t.id === templateId);
  if (!template) return templateId;

  // Create short names for common templates
  switch (templateId) {
    case "prompt_generator":
      return "Prompt Gen";
    case "creative_executor":
      return "Creative";
    case "story_writer":
      return "Story";
    case "content_only":
      return "Content";
    case "conversational":
      return "Chat";
    case "direct_execution":
      return "Direct";
    default:
      return template.name.length > 10
        ? template.name.substring(0, 10) + "..."
        : template.name;
  }
};

// AI configuration functions (enhanced)
const openAiModelSelector = (node: Node) => {
  console.log(`🤖 Opening AI configuration for node ${node.id}`);
  selectedAiNode.value = node;
  activeConfigTab.value = "model";
  showAiModelSelector.value = true;
};

const closeAiModelSelector = () => {
  console.log(`🤖 Closing AI configuration`);
  showAiModelSelector.value = false;
  selectedAiNode.value = null;
  activeConfigTab.value = "model";
};

const selectAiModel = (modelId: string) => {
  console.log(`🤖 Selecting AI model: ${modelId}`);
  if (selectedAiNode.value) {
    selectedAiNode.value.aiModel = modelId;
  }
};

const selectTemplate = (templateId: string) => {
  console.log(`📝 Selecting template: ${templateId}`);
  if (selectedAiNode.value) {
    selectedAiNode.value.promptTemplate = templateId;
  }
};

const saveAiConfiguration = () => {
  const node = selectedAiNode.value;
  if (!node) return;

  console.log(`🤖 AI Configuration saved for node ${node.id}:`, {
    model: node.aiModel,
    template: node.promptTemplate,
  });
  closeAiModelSelector();
};

// Computed drag threshold that scales with zoom level
const dragThreshold = computed(() => {
  // At higher zoom (zoomed in), we need a larger pixel threshold
  // At lower zoom (zoomed out), we need a smaller pixel threshold
  // This keeps the "world space" threshold consistent
  return baseDragThreshold * camera.value.scale;
});

// Selection state
const isSelecting = ref(false);
const selectionStart = ref<Point>({ x: 0, y: 0 });
const selectionEnd = ref<Point>({ x: 0, y: 0 });
const isCtrlPressed = ref(false);
const justFinishedSelecting = ref(false);

// Computed property for selection box style
const selectionBoxStyle = computed(() => {
  const start = selectionStart.value;
  const end = selectionEnd.value;

  // Convert world coordinates to screen coordinates using camera transform
  const screenStartX = start.x * camera.value.scale + camera.value.x;
  const screenStartY = start.y * camera.value.scale + camera.value.y;
  const screenEndX = end.x * camera.value.scale + camera.value.x;
  const screenEndY = end.y * camera.value.scale + camera.value.y;

  const left = Math.min(screenStartX, screenEndX);
  const top = Math.min(screenStartY, screenEndY);
  const width = Math.abs(screenEndX - screenStartX);
  const height = Math.abs(screenEndY - screenStartY);

  return {
    left: `${left}px`,
    top: `${top}px`,
    width: `${width}px`,
    height: `${height}px`,
  };
});

// Smart positioning for node menu to prevent cropping at screen edges
const smartMenuPosition = computed(() => {
  if (!showNodeOptions.value || !nodeOptionsPosition.value) {
    return { left: "0px", top: "0px" };
  }

  const baseWidth = 322;
  const menuWidth = baseWidth;
  const menuHeight = activeSelectionSubgroup.value ? 520 : 470;
  const padding = 16; // Minimum padding from screen edges
  const cursorOffset = 10;
  const reservedBottom = showFlowStatusPanel.value
    ? NODE_MENU_BOTTOM_RESERVED_WHEN_STATUS
    : NODE_MENU_BOTTOM_RESERVED_DEFAULT;

  let left = nodeOptionsPosition.value.clientX + cursorOffset;
  let top = nodeOptionsPosition.value.clientY + cursorOffset;

  // Get viewport dimensions
  const viewportWidth = window.innerWidth;
  const viewportHeight = window.innerHeight;
  const viewportBottomLimit = viewportHeight - reservedBottom;

  if (left + menuWidth > viewportWidth - padding) {
    left = Math.max(padding, viewportWidth - menuWidth - padding);
  }
  if (top + menuHeight > viewportBottomLimit - padding) {
    top = Math.max(padding, viewportBottomLimit - menuHeight - padding);
  }

  return {
    left: `${left}px`,
    top: `${top}px`,
  };
});

// Connection state
const isConnecting = ref(false);
const connectionStart = ref<Point>({ x: 0, y: 0 });
const connectionEnd = ref<Point>({ x: 0, y: 0 });
const connectionSource = ref<{ node: Node; type: string } | null>(null);
const savedConnectionSource = ref<{ node: Node; type: string } | null>(null);

// Add hover state for target port
const hoveredPort = ref<HTMLElement | null>(null);

// Add reactive positioning system for real-time updates
const dragUpdateTrigger = ref(0);

// Computed property for node positions that updates when nodes move
const nodePositions = computed(() => {
  // Access the trigger to make this computed property reactive to drag updates
  dragUpdateTrigger.value;

  const positions: Record<number, { x: number; y: number }> = {};
  nodes.value.forEach((node) => {
    positions[node.id] = { x: node.x, y: node.y };
  });
  return positions;
});

// Smart DOM caching for frequently accessed elements (performance optimization)
let canvasElementCache: HTMLElement | null = null;
let canvasResizeObserver: ResizeObserver | null = null;
const getCanvasElement = (): HTMLElement | null => {
  if (!canvasElementCache) {
    canvasElementCache = document.getElementById("flowchart-canvas");
  }
  return canvasElementCache;
};

// Clear cache when needed (e.g., on component unmount)
const clearDOMCache = () => {
  canvasElementCache = null;
};

const updateCanvasViewport = () => {
  const canvas = getCanvasElement();
  if (!canvas) return;

  const rect = canvas.getBoundingClientRect();
  canvasViewport.value = {
    width: Math.max(1, rect.width || 1),
    height: Math.max(1, rect.height || 1),
  };
};

const handleMinimapPanTo = (payload: { worldX: number; worldY: number }) => {
  const view = canvasViewport.value;
  camera.value.x = view.width / 2 - payload.worldX * camera.value.scale;
  camera.value.y = view.height / 2 - payload.worldY * camera.value.scale;
  triggerSmoothUpdate();
};

// Optimized connection path calculation with smart caching
const connectionPaths = computed(() => {
  // Access nodePositions to make this reactive to node movement
  nodePositions.value;
  // dragUpdateTrigger is incremented by watchers when DOM updates complete
  // (handles camera scale, nodeProgress, and other deferred updates)
  dragUpdateTrigger.value;
  // Make reactive to camera pan and zoom scale
  camera.value.x;
  camera.value.y;
  camera.value.scale;

  const paths: Record<number, string> = {};

  validConnections.value.forEach((conn) => {
    if (conn.fromType && conn.toType) {
      const startPos = getNodePosition(conn.from, conn.fromType);
      const endPos = getNodePosition(conn.to, conn.toType);
      paths[conn.id] = createSmoothBezierPath(startPos, endPos);
    }
  });
  return paths;
});

// Throttled update function for smooth performance
let updateTimeout: ReturnType<typeof setTimeout> | null = null;
const triggerDragUpdate = () => {
  if (updateTimeout) {
    clearTimeout(updateTimeout);
  }
  updateTimeout = setTimeout(() => {
    dragUpdateTrigger.value++;
  }, 8); // ~120fps for smooth updates
};

// Smart performance optimization: Use requestAnimationFrame for smooth visual updates
let animationFrameId: number | null = null;
const triggerSmoothUpdate = () => {
  if (animationFrameId) {
    cancelAnimationFrame(animationFrameId);
  }
  animationFrameId = requestAnimationFrame(() => {
    dragUpdateTrigger.value++;
    animationFrameId = null;
  });
};

// Selective throttling for expensive operations only (not affecting UX)
let expensiveOperationTimeout: ReturnType<typeof setTimeout> | null = null;
const throttleExpensiveOperation = (operation: () => void, delay: number = 16) => {
  if (expensiveOperationTimeout) {
    clearTimeout(expensiveOperationTimeout);
  }
  expensiveOperationTimeout = setTimeout(operation, delay);
};

// Add a method to force connection updates when DOM layout changes
const forceConnectionUpdate = () => {
  // Wait for DOM to update then trigger connection recalculation
  nextTick(() => {
    dragUpdateTrigger.value++;
  });
};

// Add camera state
const camera = ref({
  x: 0, // Reset to origin since we're using fixed positioning
  y: 0,
  scale: 1,
});

const isPanning = ref(false);
const panStart = ref({ x: 0, y: 0 });

// Editor Settings state
const showSettings = ref(false);
const snapToGrid = ref(true);
const showGrid = ref(true);
const gridPattern = ref<"dots" | "crosses" | "lines">("dots");

if (typeof window !== "undefined") {
  try {
    const savedPattern = localStorage.getItem("flow_editor_grid_pattern");
    if (savedPattern && ["dots", "crosses", "lines"].includes(savedPattern)) {
      gridPattern.value = savedPattern as any;
    }
  } catch {}
}

watch(gridPattern, (newPattern) => {
  if (typeof window !== "undefined") {
    try {
      localStorage.setItem("flow_editor_grid_pattern", newPattern);
    } catch {}
  }
});

// Add computed property for transform style
const transformStyle = computed(() => ({
  transform: `translate3d(${camera.value.x}px, ${camera.value.y}px, 0) scale(${camera.value.scale})`,
  transformOrigin: "0 0",
}));

// Add computed property for continuous dual-layer grid with smooth opacity cross-fade (no jumps)
const gridStyle = computed(() => {
  if (!showGrid.value) return {};

  const scale = camera.value.scale;
  const mod = (value: number, base: number) => ((value % base) + base) % base;

  // Layer 1: Fine grid (24px base world spacing)
  const fineStep = 24 * scale;
  const fineOffsetX = mod(camera.value.x, fineStep);
  const fineOffsetY = mod(camera.value.y, fineStep);

  // Smooth cross-fade for fine grid: fades out smoothly between scale 0.55 and 0.35
  const fineFade = Math.max(0, Math.min(1, (scale - 0.35) / 0.2));
  const fineAlpha = (0.5 * fineFade).toFixed(3);

  // Layer 2: Major grid (96px world spacing = exactly 4x base grid)
  const majorStep = 96 * scale;
  const majorOffsetX = mod(camera.value.x, majorStep);
  const majorOffsetY = mod(camera.value.y, majorStep);

  // Major grid stays visible down to 10% zoom with gentle attenuation
  const majorFade = Math.max(0.18, Math.min(1, (scale - 0.08) / 0.18));
  const majorAlpha = (0.6 * majorFade).toFixed(3);

  const dotRadius = Math.max(0.75, Math.min(1.2, 0.9 * Math.sqrt(Math.max(0.2, scale))));
  const fadeRadius = dotRadius + 0.6;

  if (gridPattern.value === "lines") {
    const fineLineAlpha = (0.2 * fineFade).toFixed(3);
    const majorLineAlpha = (0.3 * majorFade).toFixed(3);

    if (fineFade <= 0.01) {
      return {
        backgroundImage: `linear-gradient(to right, hsl(var(--border2) / ${majorLineAlpha}) 1px, transparent 1px), linear-gradient(to bottom, hsl(var(--border2) / ${majorLineAlpha}) 1px, transparent 1px)`,
        backgroundSize: `${majorStep}px ${majorStep}px`,
        backgroundPosition: `${majorOffsetX}px ${majorOffsetY}px`,
      };
    }

    return {
      backgroundImage: `linear-gradient(to right, hsl(var(--border2) / ${fineLineAlpha}) 1px, transparent 1px), linear-gradient(to bottom, hsl(var(--border2) / ${fineLineAlpha}) 1px, transparent 1px), linear-gradient(to right, hsl(var(--border2) / ${majorLineAlpha}) 1.5px, transparent 1.5px), linear-gradient(to bottom, hsl(var(--border2) / ${majorLineAlpha}) 1.5px, transparent 1.5px)`,
      backgroundSize: `${fineStep}px ${fineStep}px, ${fineStep}px ${fineStep}px, ${majorStep}px ${majorStep}px, ${majorStep}px ${majorStep}px`,
      backgroundPosition: `${fineOffsetX}px ${fineOffsetY}px, ${fineOffsetX}px ${fineOffsetY}px, ${majorOffsetX}px ${majorOffsetY}px, ${majorOffsetY}px ${majorOffsetY}px`,
    };
  }

  if (gridPattern.value === "crosses") {
    const arm = 3;
    const majorArm = 4;
    const svgMajor = `<svg xmlns='http://www.w3.org/2000/svg' width='${majorStep}' height='${majorStep}'><path d='M${majorStep / 2 - majorArm} ${majorStep / 2}h${majorArm * 2}M${majorStep / 2} ${majorStep / 2 - majorArm}v${majorArm * 2}' stroke='currentColor' stroke-width='1.2' opacity='${majorAlpha}'/></svg>`;

    if (fineFade <= 0.01) {
      return {
        backgroundImage: `url("data:image/svg+xml,${encodeURIComponent(svgMajor)}")`,
        backgroundPosition: `${majorOffsetX - majorStep / 2}px ${majorOffsetY - majorStep / 2}px`,
        backgroundSize: `${majorStep}px ${majorStep}px`,
        color: "hsl(var(--border2))",
      };
    }

    const svgFine = `<svg xmlns='http://www.w3.org/2000/svg' width='${fineStep}' height='${fineStep}'><path d='M${fineStep / 2 - arm} ${fineStep / 2}h${arm * 2}M${fineStep / 2} ${fineStep / 2 - arm}v${arm * 2}' stroke='currentColor' stroke-width='1' opacity='${fineAlpha}'/></svg>`;
    return {
      backgroundImage: `url("data:image/svg+xml,${encodeURIComponent(svgFine)}"), url("data:image/svg+xml,${encodeURIComponent(svgMajor)}")`,
      backgroundPosition: `${fineOffsetX - fineStep / 2}px ${fineOffsetY - fineStep / 2}px, ${majorOffsetX - majorStep / 2}px ${majorOffsetY - majorStep / 2}px`,
      backgroundSize: `${fineStep}px ${fineStep}px, ${majorStep}px ${majorStep}px`,
      color: "hsl(var(--border2))",
    };
  }

  // Default: Dual-Layer Continuous Dots
  if (fineFade <= 0.01) {
    return {
      backgroundImage: `radial-gradient(circle, hsl(var(--border2) / ${majorAlpha}) ${dotRadius}px, transparent ${fadeRadius}px)`,
      backgroundSize: `${majorStep}px ${majorStep}px`,
      backgroundPosition: `${majorOffsetX}px ${majorOffsetY}px`,
    };
  }

  return {
    backgroundImage: `radial-gradient(circle, hsl(var(--border2) / ${fineAlpha}) ${dotRadius}px, transparent ${fadeRadius}px), radial-gradient(circle, hsl(var(--border2) / ${majorAlpha}) ${dotRadius + 0.25}px, transparent ${fadeRadius + 0.25}px)`,
    backgroundSize: `${fineStep}px ${fineStep}px, ${majorStep}px ${majorStep}px`,
    backgroundPosition: `${fineOffsetX}px ${fineOffsetY}px, ${majorOffsetX}px ${majorOffsetY}px`,
  };
});

// First, add state to track hovered ports
const hoveredFlowPort = ref<{ nodeId: number; type: string } | null>(null);

// Function to update selected nodes based on selection rectangle
const updateSelection = () => {
  const newSelection = new Set<number>();

  nodes.value.forEach((node) => {
    if (isNodeInSelection(node)) {
      newSelection.add(node.id);
    }
  });

  // Update both multi-selection and primary selection
  selectedNodes.value = newSelection;

  // Set primary selected node to the first one if any are selected
  if (newSelection.size > 0) {
    const firstSelectedId = Array.from(newSelection)[0];
    selectedNode.value = firstSelectedId === undefined
      ? null
      : findNodeById(firstSelectedId) || null;
  } else {
    selectedNode.value = null;
  }
};

// Drag and drop
const startDrag = (node: Node, event: MouseEvent) => {
  // Don't start dragging if we're already resizing
  if (isResizing.value) {
    return;
  }

  // Only allow dragging with left mouse button (button 0)
  if (event.button !== 0) {
    return; // Ignore right mouse button and other buttons
  }

  // Reset drag occurred flag
  dragOccurred.value = false;

  // Record initial mouse position for threshold calculation
  dragStartPosition.value = {
    x: event.clientX,
    y: event.clientY,
  };

  // If multiple nodes are selected and we're clicking on one of them, start dragging immediately
  if (selectedNodes.value.size > 1 && selectedNodes.value.has(node.id)) {
    // Don't change selection, just start dragging all selected nodes
    isDragging.value = true;
    selectedNode.value = node; // Make this the primary node for drag calculations
  } else if (selectedNodes.value.has(node.id)) {
    // Single node already selected - just start dragging
    isDragging.value = true;
    selectedNode.value = node;
  } else {
    // Clicking on a non-selected node - select it first, then start dragging
    selectNode(node, event);
    isDragging.value = true;
    selectedNode.value = node;
  }

  // Calculate the initial offset considering the camera transform
  const canvas = document.getElementById("flowchart-canvas");
  if (!canvas) return;

  const rect = canvas.getBoundingClientRect();
  const nodeX = node.x * camera.value.scale + camera.value.x;
  const nodeY = node.y * camera.value.scale + camera.value.y;

  dragOffset.value = {
    x: event.clientX - nodeX,
    y: event.clientY - nodeY,
  };

  // Attach global mouse events to prevent losing drag when hovering over other nodes
  document.addEventListener("mousemove", handleGlobalDrag);
  document.addEventListener("mouseup", handleGlobalDragEnd);

  // Prevent text selection during drag
  event.preventDefault();
};

// Helper function to snap coordinates to grid
const snapToGridCoord = (coord: number, gridSize: number = 20): number => {
  if (!snapToGrid.value) return coord;
  return Math.round(coord / gridSize) * gridSize;
};

const handleGlobalDrag = (event: MouseEvent) => {
  if (isDragging.value && selectedNode.value) {
    // Calculate distance moved from initial position
    const deltaX = event.clientX - dragStartPosition.value.x;
    const deltaY = event.clientY - dragStartPosition.value.y;
    const distance = Math.sqrt(deltaX * deltaX + deltaY * deltaY);

    // Only proceed with dragging if movement exceeds threshold
    if (distance > dragThreshold.value) {
      // Mark that dragging has occurred
      dragOccurred.value = true;

      // Calculate new position considering camera transform
      let newX =
        (event.clientX - dragOffset.value.x - camera.value.x) / camera.value.scale;
      let newY =
        (event.clientY - dragOffset.value.y - camera.value.y) / camera.value.scale;

      // Apply snap to grid if enabled
      newX = snapToGridCoord(newX);
      newY = snapToGridCoord(newY);

      // Calculate the movement delta
      const deltaNodeX = newX - selectedNode.value.x;
      const deltaNodeY = newY - selectedNode.value.y;

      // Move the primary selected node IMMEDIATELY (no delay for UX)
      selectedNode.value.x = newX;
      selectedNode.value.y = newY;

      // Move all other selected nodes by the same delta IMMEDIATELY
      if (selectedNodes.value.size > 1) {
        nodes.value.forEach((node) => {
          if (selectedNodes.value.has(node.id) && node.id !== selectedNode.value!.id) {
            // Apply snap to grid to other nodes as well
            node.x = snapToGridCoord(node.x + deltaNodeX);
            node.y = snapToGridCoord(node.y + deltaNodeY);
          }
        });
      }

      // Use smart update for visual connection updates (performance optimization)
      triggerSmoothUpdate();
    }
    // If movement is below threshold, don't move nodes or set dragOccurred
  }
};

const handleGlobalDragEnd = () => {
  if (isDragging.value) {
    stopDrag();
  }
};

const dragNode = (event: MouseEvent) => {
  // This function is now handled by handleGlobalDrag
  // Keeping it for compatibility but it's no longer the primary drag handler
};

const stopDrag = () => {
  isDragging.value = false;

  // Remove global event listeners
  document.removeEventListener("mousemove", handleGlobalDrag);
  document.removeEventListener("mouseup", handleGlobalDragEnd);

  // Trigger connection update after drag ends to catch up on any deferred updates
  nextTick(() => {
    dragUpdateTrigger.value++;
  });
};

// Resize management
const startResize = (node: Node, event: MouseEvent) => {
  // Only allow resizing with left mouse button
  if (event.button !== 0) return;

  event.preventDefault();
  event.stopPropagation();

  isResizing.value = true;
  resizeNode.value = node;

  // Get the node's actual current size from DOM
  const nodeElement = document.querySelector(
    `[data-node-id="${node.id}"]`
  ) as HTMLElement;
  if (nodeElement) {
    const innerDiv = nodeElement.querySelector("div") as HTMLElement;
    if (innerDiv) {
      // Get actual rendered size accounting for camera scale
      const rect = innerDiv.getBoundingClientRect();
      const scale = camera.value.scale;

      const currentWidth = node.width || rect.width / scale;
      const currentHeight = node.height || rect.height / scale;

      resizeStartSize.value = {
        width: currentWidth,
        height: currentHeight,
      };
    }
  }

  resizeStartMouse.value = {
    x: event.clientX,
    y: event.clientY,
  };

  // Attach global mouse events
  document.addEventListener("mousemove", handleGlobalResize);
  document.addEventListener("mouseup", handleGlobalResizeEnd);
};

const handleGlobalResize = (event: MouseEvent) => {
  if (!isResizing.value || !resizeNode.value) return;

  // Calculate delta from start position, accounting for camera scale
  const deltaX = (event.clientX - resizeStartMouse.value.x) / camera.value.scale;
  const deltaY = (event.clientY - resizeStartMouse.value.y) / camera.value.scale;

  // Use stored minimum constraints (set at node creation) for stable min/max across resizes
  // Falls back to resize start size for backward compatibility with loaded flows
  const minWidth = resizeNode.value.minWidth || resizeStartSize.value.width;
  const minHeight = resizeNode.value.minHeight || resizeStartSize.value.height;

  // Calculate new size - enforce minimum constraints but allow growth
  const newWidth = Math.max(minWidth, resizeStartSize.value.width + deltaX);
  const newHeight = Math.max(minHeight, resizeStartSize.value.height + deltaY);

  // Update the reactive data - CSS transition is disabled via resizing-node class
  const nodeIndex = nodes.value.findIndex((n) => n.id === resizeNode.value!.id);
  const resizedNode = nodeIndex === -1 ? undefined : nodes.value[nodeIndex];
  if (resizedNode) {
    resizedNode.width = newWidth;
    resizedNode.height = newHeight;
  }
};

const handleGlobalResizeEnd = () => {
  if (isResizing.value) {
    stopResize();
  }
};

const stopResize = () => {
  isResizing.value = false;
  resizeNode.value = null;

  // Remove global event listeners
  document.removeEventListener("mousemove", handleGlobalResize);
  document.removeEventListener("mouseup", handleGlobalResizeEnd);
};

// Connection management
const startConnection = (node: Node, event: MouseEvent, type: string) => {
  // Only allow starting a connection if we're not already connecting
  if (isConnecting.value) return;

  event.preventDefault();
  event.stopPropagation();

  isConnecting.value = true;
  connectionSource.value = { node, type };

  const portElement = event.currentTarget as HTMLElement;
  const portRect = portElement.getBoundingClientRect();

  // Calculate the center point of the port (screen coordinates)
  const portCenterX = portRect.left + portRect.width / 2;
  const portCenterY = portRect.top + portRect.height / 2;

  // Set the start position to the port center position (screen coordinates)
  connectionStart.value = {
    x: portCenterX,
    y: portCenterY,
  };

  // Set the end position to the current mouse position (screen coordinates)
  connectionEnd.value = {
    x: event.clientX,
    y: event.clientY,
  };
};

const cancelConnection = () => {
  if (!isConnecting.value) return;

  isConnecting.value = false;
  connectionSource.value = null;
  clearAllPortHovers();
};

// Then update the updateConnectionLine function to better detect hovering over ports
const updateConnectionLine = (event: MouseEvent) => {
  if (!isConnecting.value || !connectionSource.value) return;

  // IMMEDIATE connection end position update (critical for UX) - use screen coordinates
  connectionEnd.value = { x: event.clientX, y: event.clientY };

  // Smart throttling for expensive port detection (performance optimization)
  throttleExpensiveOperation(() => {
    performPortDetection(event);
  }, 4); // Very fast throttling to maintain responsiveness
};

// Separate expensive port detection logic for smart throttling
const performPortDetection = (event: MouseEvent) => {
  // Check if we're hovering over any compatible input port
  let foundPort = false;
  const sourceType = connectionSource.value?.type;
  if (!sourceType) return;

  // Determine the type of port we're looking for based on the source type
  let targetSelector = "";
  if (sourceType.startsWith("flow-output")) {
    targetSelector = ".flow-input-port";
  } else if (sourceType.includes("value-output")) {
    targetSelector = ".value-input-port";
  } else if (sourceType === "flow-input") {
    targetSelector = ".flow-output-port";
  } else if (sourceType.includes("value-input")) {
    targetSelector = ".value-output-port";
  } else {
    // Not a valid connection source
    return;
  }

  // Increase detection threshold to make it easier to connect
  const threshold = sourceType.includes("flow") ? 40 : 25;

  nodes.value.forEach((node) => {
    if (node.id === connectionSource.value?.node.id) return; // Skip source node

    const nodeElement = document.querySelector(`[data-node-id="${node.id}"]`);
    if (nodeElement) {
      // Find all potential target ports
      const portElements = nodeElement.querySelectorAll(targetSelector);

      portElements.forEach((portElement) => {
        const inputPort = portElement as HTMLElement;
        const portRect = inputPort.getBoundingClientRect();
        const portCenterX = portRect.left + portRect.width / 2;
        const portCenterY = portRect.top + portRect.height / 2;

        // Calculate distance using screen coordinates
        const distance = Math.sqrt(
          Math.pow(event.clientX - portCenterX, 2) +
            Math.pow(event.clientY - portCenterY, 2)
        );

        // Check if cursor is within threshold distance of the port
        if (distance < threshold) {
          // Found a compatible port within threshold
          foundPort = true;

          // Set new hover state using centralized function
          setPortHover(inputPort);

          // IMMEDIATE visual feedback for connection snap (screen coordinates)
          connectionEnd.value = { x: portCenterX, y: portCenterY };

          // While linking, also update the hoveredFlowPort state if it's a flow port
          if (
            inputPort.classList.contains("flow-input-port") ||
            inputPort.classList.contains("flow-output-port")
          ) {
            // Find the node ID this port belongs to
            let nodeElement = inputPort;
            while (nodeElement && !nodeElement.hasAttribute("data-node-id")) {
              nodeElement = nodeElement.parentElement as HTMLElement;
            }

            if (nodeElement) {
              const nodeId = parseInt(nodeElement.getAttribute("data-node-id") || "0");
              const portType = inputPort.classList.contains("flow-input-port")
                ? "flow-input"
                : "flow-output";

              // hoveredFlowPort.value = { nodeId, type: portType };
            }
          }
        } else if (hoveredPort.value === inputPort) {
          // If we were hovering over this port but moved away
          clearPortHover();

          // Also reset hoveredFlowPort if applicable
          if (
            hoveredFlowPort.value &&
            (inputPort.classList.contains("flow-input-port") ||
              inputPort.classList.contains("flow-output-port"))
          ) {
            hoveredFlowPort.value = null;
          }
        }
      });
    }
  });

  if (!foundPort && hoveredPort.value) {
    clearPortHover();

    // Reset hoveredFlowPort if we're not hovering over any port
    hoveredFlowPort.value = null;
  }
};

const endConnection = (event: MouseEvent) => {
  if (!isConnecting.value || !connectionSource.value) return;

  // Use screen coordinates directly since everything is fixed positioned now
  const x = event.clientX;
  const y = event.clientY;

  // If we have a hovered port, create a connection to it
  if (hoveredPort.value) {
    // Find the node element that contains this port
    let nodeElement = hoveredPort.value;
    while (nodeElement && !nodeElement.hasAttribute("data-node-id")) {
      nodeElement = nodeElement.parentElement as HTMLElement;
    }

    if (nodeElement) {
      const targetNodeId = parseInt(nodeElement.getAttribute("data-node-id") || "0");
      const targetNode = findNodeById(targetNodeId);
      const sourceNode = connectionSource.value.node;
      const sourcePortType = connectionSource.value.type;

      // Determine target port type
      let targetPortType = "";
      if (hoveredPort.value.classList.contains("flow-input-port")) {
        targetPortType = "flow-input";
      } else if (hoveredPort.value.classList.contains("flow-output-port")) {
        const routeElement = hoveredPort.value.closest("[data-flow-output-id]");
        const routeId = routeElement?.getAttribute("data-flow-output-id");
        targetPortType = routeId ? `flow-output-${routeId}` : "flow-output";
      } else if (hoveredPort.value.classList.contains("value-input-port")) {
        // Extract input ID from any row carrying input metadata.
        const inputElement = hoveredPort.value.closest("[data-input-id]");
        if (inputElement) {
          const inputId = inputElement.getAttribute("data-input-id");
          if (inputId) {
            targetPortType = "value-input-" + inputId;
          }
        }
      } else if (hoveredPort.value.classList.contains("value-output-port")) {
        // Extract output ID from any row carrying output metadata.
        const outputElement = hoveredPort.value.closest("[data-output-id]");
        if (outputElement) {
          const outputId = outputElement.getAttribute("data-output-id");
          if (outputId) {
            targetPortType = "value-output-" + outputId;
          }
        }
      }

      // Validate that we have both nodes and port types
      if (!targetNode || !sourceNode || !sourcePortType || !targetPortType) {
        console.warn("Invalid connection parameters:", {
          targetNode: !!targetNode,
          sourceNode: !!sourceNode,
          sourcePortType,
          targetPortType,
        });

        // Reset connection state
        isConnecting.value = false;
        connectionSource.value = null;
        if (hoveredPort.value) {
          hoveredPort.value.classList.remove("hovered-port");
          hoveredPort.value = null;
        }
        return;
      }

      if (targetNodeId !== sourceNode.id) {
        const created = addConnectionWithValidation(
          sourceNode,
          targetNode,
          sourcePortType,
          targetPortType
        );

        if (created) {
          // Trigger variable node type sync on connection
          canvasRef.value?.onConnectionChange(sourceNode.id, targetNodeId);
        }

        // Reset connection state
        isConnecting.value = false;
        connectionSource.value = null;
        hoveredPort.value.classList.remove("hovered-port");
        hoveredPort.value = null;
        return;
      }
    }
  }

  // If we're not hovering over any valid target port, show the node selection menu
  console.log("Showing node selection menu at", x, y);

  // Save connection source for when a node is selected from the menu
  savedConnectionSource.value = {
    node: connectionSource.value.node,
    type: connectionSource.value.type,
  };

  openNodeOptionsAtClientPosition(event.clientX, event.clientY);

  // Reset connection state
  isConnecting.value = false;
  connectionSource.value = null;
  if (hoveredPort.value) {
    hoveredPort.value.classList.remove("hovered-port");
    hoveredPort.value = null;
  }
};

// Helper function to create temporary connection path
const createTemporaryPath = (start: Point, end: Point) => {
  // Use the same smooth bezier curve logic as permanent connections
  // Scale the offset with camera zoom to maintain consistent curve appearance
  const offset = 80 * camera.value.scale;

  // Create simple bezier curve for temporary connection
  const cp1x = start.x + offset;
  const cp1y = start.y;
  const cp2x = end.x - offset;
  const cp2y = end.y;

  return `M ${start.x} ${start.y} C ${cp1x} ${cp1y}, ${cp2x} ${cp2y}, ${end.x} ${end.y}`;
};

// Add helper function to get node dimensions
const getNodeDimensions = (nodeId: number) => {
  const node = findNodeById(nodeId);

  // Better default dimensions based on node type
  let width = 200; // Default width
  let height = 140; // Default height
  // Adjust defaults for trigger nodes which have more outputs
  if (node && isTriggerNode(node.type)) {
    width = 320; // Wider for trigger nodes
    height = 180; // Taller for multiple outputs
  }

  try {
    // Try to get actual dimensions from DOM
    const nodeElem = document.querySelector(`[data-node-id="${nodeId}"]`);

    if (nodeElem) {
      const rect = nodeElem.getBoundingClientRect();
      width = rect.width / camera.value.scale;
      height = rect.height / camera.value.scale;
    }
  } catch (e) {
    console.error("Error getting node dimensions:", e);
  }

  return { width, height };
};

// Helper function to find node ID from connection point
const findNodeIdFromPoint = (point: Point, isSource: boolean): number => {
  // Try to get the connection being drawn
  if (connectionSource.value && isSource) {
    return connectionSource.value.node.id;
  }

  // Fallback: use the most recent connection
  const conn = connections.value[connections.value.length - 1];
  if (conn) {
    return isSource ? conn.from : conn.to;
  }

  return 0;
};

// Update the calculateVerticalOffset function to consider port position
const calculateVerticalOffset = (start: any, end: any) => {
  const offset = 32; // 2rem in pixels
  const margin = 32; // 2rem margin around nodes

  // Get port positions
  const startPortPosition = start.portPosition || "right";
  const endPortPosition = end.portPosition || "left";

  // Get source and target node IDs
  const sourceId = findNodeIdFromPoint(start, true);
  const targetId = findNodeIdFromPoint(end, false);

  // Get dynamic dimensions
  const sourceDims = getNodeDimensions(sourceId);
  const targetDims = getNodeDimensions(targetId);

  const sourceHeight = sourceDims.height;
  const targetHeight = targetDims.height;

  // Adjust vertical offset based on port positions
  let verticalOffset = Math.max(sourceHeight, targetHeight) / 3 + 0;

  // Increase offset for top/bottom ports to create more space
  if (
    startPortPosition === "top" ||
    startPortPosition === "bottom" ||
    endPortPosition === "top" ||
    endPortPosition === "bottom"
  ) {
    verticalOffset += 0;
  }

  // If both ports are on the same vertical edge (both top or both bottom)
  if (
    (startPortPosition === "top" && endPortPosition === "top") ||
    (startPortPosition === "bottom" && endPortPosition === "bottom")
  ) {
    verticalOffset += 0 * 2;
  }

  return verticalOffset;
};

// Helper function to calculate port position
const calculatePortPosition = (port: HTMLElement, nodeElement: Element, type: string) => {
  const rect = port.getBoundingClientRect();
  const nodeRect = nodeElement.getBoundingClientRect();
  const canvas = document.getElementById("flowchart-canvas");

  if (!canvas) {
    return { x: 0, y: 0, portPosition: "right" };
  }

  const canvasRect = canvas.getBoundingClientRect();
  // Calculate the center point of the port
  const portCenterX = rect.left + rect.width / 2;
  const portCenterY = rect.top + rect.height / 2;

  // Calculate port position relative to node
  const portPositionInNode = {
    x: (portCenterX - nodeRect.left) / camera.value.scale,
    y: (portCenterY - nodeRect.top) / camera.value.scale,
  };

  // Determine port position (top, bottom, left, right)
  const nodeWidth = nodeRect.width / camera.value.scale;
  const nodeHeight = nodeRect.height / camera.value.scale;

  let portPosition = "";
  // Check if port is closer to top/bottom or left/right
  if (portPositionInNode.y < nodeHeight * 0.25) {
    portPosition = "top";
  } else if (portPositionInNode.y > nodeHeight * 0.75) {
    portPosition = "bottom";
  } else if (portPositionInNode.x < nodeWidth * 0.25) {
    portPosition = "left";
  } else if (portPositionInNode.x > nodeWidth * 0.75) {
    portPosition = "right";
  } else {
    // Default based on connection type
    portPosition = type && type.includes("input") ? "left" : "right";
  }

  // Convert to canvas coordinates
  return {
    x: (portCenterX - canvasRect.left - camera.value.x) / camera.value.scale,
    y: (portCenterY - canvasRect.top - camera.value.y) / camera.value.scale,
    portPosition, // Add port position information
  };
};

// Update the createOrthogonalPath function to handle vertical port connections better
const createOrthogonalPath = (start: any, end: any) => {
  const offset = 32; // 2rem in pixels
  const margin = 32; // 2rem margin around nodes

  // Get node IDs and dimensions
  const sourceId = findNodeIdFromPoint(start, true);
  const targetId = findNodeIdFromPoint(end, false);
  const sourceNode = getNodeDimensions(sourceId);
  const targetNode = getNodeDimensions(targetId);

  // Check port positions
  const startPortPosition = start.portPosition || "right";
  const endPortPosition = end.portPosition || "left";

  // Calculate initial direction based on port position
  let startOffset = { x: 0, y: 0 };
  let endOffset = { x: 0, y: 0 };

  // Set direction offsets based on port position
  if (startPortPosition === "right") {
    startOffset.x = offset;
  } else if (startPortPosition === "left") {
    startOffset.x = -offset;
  } else if (startPortPosition === "top") {
    startOffset.y = -offset;
  } else if (startPortPosition === "bottom") {
    startOffset.y = offset;
  }

  if (endPortPosition === "right") {
    endOffset.x = offset;
  } else if (endPortPosition === "left") {
    endOffset.x = -offset;
  } else if (endPortPosition === "top") {
    endOffset.y = -offset;
  } else if (endPortPosition === "bottom") {
    endOffset.y = offset;
  }

  // Create initial points with correct offsets
  const startPoint = {
    x: start.x + startOffset.x,
    y: start.y + startOffset.y,
  };

  const endPoint = {
    x: end.x + endOffset.x,
    y: end.y + endOffset.y,
  };

  // Calculate node boundaries using dynamic dimensions
  const sourceRight = start.x + sourceNode.width;
  const sourceLeft = start.x;
  const targetRight = end.x + targetNode.width;
  const targetLeft = end.x;

  // Special case: Vertical to vertical port connections (top to top, bottom to bottom, etc.)
  if (
    (startPortPosition === "top" || startPortPosition === "bottom") &&
    (endPortPosition === "top" || endPortPosition === "bottom")
  ) {
    // Use a three-segment path with increased vertical offset
    const verticalDistance = Math.abs(start.y - end.y);
    const midY = (start.y + end.y) / 2;

    // Calculate more extreme vertical point if nodes are close together vertically
    const adjustedMidY =
      verticalDistance < margin * 2
        ? startPortPosition === "top"
          ? Math.min(start.y, end.y) - margin * 2
          : Math.max(start.y, end.y) + margin * 2
        : midY;

    return `M ${start.x} ${start.y} 
               L ${start.x} ${start.y + startOffset.y}
               L ${start.x} ${adjustedMidY}
               L ${end.x} ${adjustedMidY}
               L ${end.x} ${end.y + endOffset.y}
               L ${end.x} ${end.y}`;
  }

  // Special case: Top/bottom to left/right port connections
  if (
    (startPortPosition === "top" || startPortPosition === "bottom") &&
    (endPortPosition === "left" || endPortPosition === "right")
  ) {
    // Create a path with appropriate vertical and horizontal segments
    return `M ${start.x} ${start.y} 
               L ${start.x} ${start.y + startOffset.y}
               L ${end.x + endOffset.x} ${start.y + startOffset.y}
               L ${end.x + endOffset.x} ${end.y}
               L ${end.x} ${end.y}`;
  }

  // Special case: Left/right to top/bottom port connections
  if (
    (startPortPosition === "left" || startPortPosition === "right") &&
    (endPortPosition === "top" || endPortPosition === "bottom")
  ) {
    // Create a path with appropriate vertical and horizontal segments
    return `M ${start.x} ${start.y} 
               L ${start.x + startOffset.x} ${start.y}
               L ${start.x + startOffset.x} ${end.y + endOffset.y}
               L ${end.x} ${end.y + endOffset.y}
               L ${end.x} ${end.y}`;
  }

  // Determine if nodes would overlap horizontally
  const horizontalOverlap =
    startPoint.x > targetLeft - margin && endPoint.x < sourceRight + margin;

  // Use vertical offset strategy if ports are on sides and nodes overlap horizontally
  if (
    horizontalOverlap &&
    (startPortPosition === "left" || startPortPosition === "right") &&
    (endPortPosition === "left" || endPortPosition === "right")
  ) {
    // Use fixed vertical offset to avoid node overlap
    const verticalOffset = margin + calculateVerticalOffset(start, end);
    const isTargetBelow = start.y < end.y;

    const verticalPoint1 = {
      x: startPoint.x,
      y: isTargetBelow ? start.y + verticalOffset : start.y - verticalOffset,
    };

    const horizontalPoint = {
      x: endPoint.x,
      y: verticalPoint1.y,
    };

    return `M ${start.x} ${start.y} 
               L ${startPoint.x} ${start.y} 
               L ${startPoint.x} ${verticalPoint1.y}
               L ${endPoint.x} ${horizontalPoint.y}
               L ${endPoint.x} ${end.y}
               L ${end.x} ${end.y}`;
  }

  // If nodes are at close vertical positions, use direct horizontal connection
  if (
    Math.abs(start.y - end.y) < 32 &&
    (startPortPosition === "left" || startPortPosition === "right") &&
    (endPortPosition === "left" || endPortPosition === "right")
  ) {
    return `M ${start.x} ${start.y} 
               L ${startPoint.x} ${start.y} 
               L ${endPoint.x} ${end.y}
               L ${end.x} ${end.y}`;
  }

  // Default case: Create a path with a break in the middle
  const midX = (startPoint.x + endPoint.x) / 2;

  return `M ${start.x} ${start.y} 
             L ${startPoint.x} ${start.y} 
             L ${midX} ${start.y}
             L ${midX} ${end.y}
             L ${endPoint.x} ${end.y}
             L ${end.x} ${end.y}`;
};

// Simplify the startPan method to use right mouse for selection, left for panning
const startPan = (event: MouseEvent) => {
  // Check if the event originated from the settings area - if so, ignore completely
  const clickTarget = event.target as Element;
  const isFromSettingsArea =
    clickTarget.closest(".settings-dropdown") ||
    clickTarget.closest('button[class*="bg-gradient-to-r from-gray-700"]');

  if (isFromSettingsArea) {
    return; // Exit early, don't process this event
  }

  // Shift + right mouse button (2) for box selection
  // Plain right click opens the node picker context menu.
  if (event.button === 2) {
    if (!event.shiftKey) {
      return;
    }

    // Start selection
    isSelecting.value = true;

    const canvas = document.getElementById("flowchart-canvas");
    if (canvas) {
      const rect = canvas.getBoundingClientRect();
      const x = (event.clientX - rect.left - camera.value.x) / camera.value.scale;
      const y = (event.clientY - rect.top - camera.value.y) / camera.value.scale;

      selectionStart.value = { x, y };
      selectionEnd.value = { x, y };
    }
  }
  // Left mouse button (0) for panning
  else if (event.button === 0 && !isConnecting.value && !isDragging.value) {
    // Start panning
    // If mousedown is directly on the canvas, unselect nodes
    if (event.target === event.currentTarget) {
      clearSelection();
    }

    isPanning.value = true;
    panStart.value = {
      x: event.clientX - camera.value.x,
      y: event.clientY - camera.value.y,
    };
  }
};

// Add a dedicated method to unselect nodes
const unselectNodes = (event?: MouseEvent) => {
  // Don't unselect if we just finished a selection operation
  if (justFinishedSelecting.value) {
    return;
  }

  // If no event provided (e.g., from Canvas emit), just clear selection
  if (!event) {
    clearSelection();
    return;
  }

  // Only proceed if the click wasn't on a node
  const clickedOnNode = event.composedPath().some((el) => {
    return el instanceof HTMLElement && el.classList.contains("node");
  });

  if (!clickedOnNode) {
    // Clicking empty space always clears selection
    clearSelection();
  }
};

// Update mousemove handler to include panning and connection
const handleMouseMove = (event: MouseEvent) => {
  if (isSelecting.value) {
    // IMMEDIATE update for selection box (critical UX)
    const canvas = document.getElementById("flowchart-canvas");
    if (canvas) {
      const rect = canvas.getBoundingClientRect();
      const x = (event.clientX - rect.left - camera.value.x) / camera.value.scale;
      const y = (event.clientY - rect.top - camera.value.y) / camera.value.scale;

      selectionEnd.value = { x, y };
      updateSelection(); // Immediate selection update
    }
  } else if (isPanning.value) {
    // IMMEDIATE camera update for responsive panning
    camera.value.x = event.clientX - panStart.value.x;
    camera.value.y = event.clientY - panStart.value.y;

    // Smart throttling only for expensive connection updates
    throttleExpensiveOperation(() => {
      triggerSmoothUpdate();
    }, 8);
  } else if (isConnecting.value) {
    // IMMEDIATE connection line update (critical for UX)
    updateConnectionLine(event);
  }
};

// Update mouseup handler to include connection finalization
const handleMouseUp = (event: MouseEvent) => {
  if (isSelecting.value) {
    // Complete selection
    isSelecting.value = false;
    // Final selection update
    updateSelection();

    // Set flag to prevent immediate unselection
    justFinishedSelecting.value = true;
    setTimeout(() => {
      justFinishedSelecting.value = false;
    }, 100); // Small delay to prevent immediate unselection
  } else if (isConnecting.value) {
    endConnection(event);
  } else if (isPanning.value) {
    stopPan();
  }
};

// Add zoom handler
const handleZoom = (event: WheelEvent) => {
  // Prevent zooming when connecting or dragging
  if (isConnecting.value || isDragging.value) return;

  const delta = event.deltaY;
  const zoomIntensity = 0.001;
  const zoom = 1 - delta * zoomIntensity;

  // Limit zoom range (0.5x to 3x)
  const newScale = Math.min(Math.max(camera.value.scale * zoom, 0.1), 5);

  // Get the canvas element using cached version
  const canvas = getCanvasElement();
  if (!canvas) return;

  // Get the canvas position and dimensions
  const rect = canvas.getBoundingClientRect();

  // Calculate mouse position relative to canvas
  const mouseX = event.clientX - rect.left;
  const mouseY = event.clientY - rect.top;

  // Calculate the point in the canvas that should stay under the cursor
  const pointX = (mouseX - camera.value.x) / camera.value.scale;
  const pointY = (mouseY - camera.value.y) / camera.value.scale;

  // Calculate new camera position to keep the point under the cursor
  const newX = mouseX - pointX * newScale;
  const newY = mouseY - pointY * newScale;

  // IMMEDIATE camera update for responsive zoom
  camera.value = {
    x: newX,
    y: newY,
    scale: newScale,
  };

  // Smart throttling for expensive connection updates after zoom
  throttleExpensiveOperation(() => {
    forceConnectionUpdate();
  }, 16);
};

// Update the nodeOptionsPosition to include client coordinates
const nodeOptionsPosition = ref<{
  x: number;
  y: number;
  clientX: number;
  clientY: number;
}>({
  x: 0,
  y: 0,
  clientX: 0,
  clientY: 0,
});

const nodePickerCascadeRef = ref<HTMLElement | null>(null);
const categoryItemRefs: Record<string, HTMLElement> = {};
const subgroupItemRefs: Record<string, HTMLElement> = {};
const nodeMenuAlignTick = ref(0);
const NODE_MENU_VIEW_PADDING = 16;
const NODE_MENU_POPOUT_GAP = 8;
const NODE_MENU_POPOUT_MAX_WIDTH = 246;
const NODE_MENU_BOTTOM_RESERVED_WHEN_STATUS = 96;
const NODE_MENU_BOTTOM_RESERVED_DEFAULT = 20;

const setCategoryItemRef = (categoryId: string, el: unknown) => {
  if (el instanceof HTMLElement) {
    categoryItemRefs[categoryId] = el;
  } else {
    delete categoryItemRefs[categoryId];
  }
};

const setSubgroupItemRef = (groupId: string, el: unknown) => {
  if (el instanceof HTMLElement) {
    subgroupItemRefs[groupId] = el;
  } else {
    delete subgroupItemRefs[groupId];
  }
};

const resolveCategoryAnchorElement = (categoryId: string | null): HTMLElement | undefined => {
  if (!categoryId) return undefined;
  const fromRef = categoryItemRefs[categoryId];
  if (fromRef) return fromRef;

  const cascade = nodePickerCascadeRef.value;
  if (!cascade) return undefined;
  return Array.from(cascade.querySelectorAll("[data-category-id]")).find(
    (el) => (el as HTMLElement).dataset.categoryId === categoryId
  ) as HTMLElement | undefined;
};

const resolveSubgroupAnchorElement = (groupId: string | null): HTMLElement | undefined => {
  if (!groupId) return undefined;
  const fromRef = subgroupItemRefs[groupId];
  if (fromRef) return fromRef;

  const cascade = nodePickerCascadeRef.value;
  if (!cascade) return undefined;
  return Array.from(cascade.querySelectorAll("[data-subgroup-id]")).find(
    (el) => (el as HTMLElement).dataset.subgroupId === groupId
  ) as HTMLElement | undefined;
};

const estimatePopoutHeight = (itemCount: number): number => {
  const normalizedCount = Math.max(0, itemCount || 0);
  return Math.min(348, Math.max(120, 54 + normalizedCount * 34));
};

const getNodePickerMetrics = () => {
  nodeMenuAlignTick.value;
  const cascade = nodePickerCascadeRef.value;
  const cascadeWidth = cascade?.clientWidth || 302;
  const popoutWidth = Math.min(window.innerWidth * 0.8, NODE_MENU_POPOUT_MAX_WIDTH);
  const menuLeft = Number.parseFloat((smartMenuPosition.value.left || "0").replace("px", "")) || 0;

  return {
    cascadeWidth,
    popoutWidth,
    menuLeft,
    gap: NODE_MENU_POPOUT_GAP,
    viewportPadding: NODE_MENU_VIEW_PADDING,
  };
};

const level2LeftPx = computed(() => {
  const { cascadeWidth, popoutWidth, menuLeft, gap, viewportPadding } = getNodePickerMetrics();
  const rightLeft = cascadeWidth + gap;
  const leftLeft = -popoutWidth - gap;

  const fitsRight = menuLeft + rightLeft + popoutWidth <= window.innerWidth - viewportPadding;
  const fitsLeft = menuLeft + leftLeft >= viewportPadding;
  if (fitsRight) return rightLeft;
  if (fitsLeft) return leftLeft;

  const spaceRight = window.innerWidth - viewportPadding - (menuLeft + cascadeWidth);
  const spaceLeft = menuLeft - viewportPadding;
  return spaceRight >= spaceLeft ? rightLeft : leftLeft;
});

const level3LeftPx = computed(() => {
  const { popoutWidth, menuLeft, gap, viewportPadding } = getNodePickerMetrics();
  const level2Left = level2LeftPx.value;
  const rightLeft = level2Left + popoutWidth + gap;
  const leftLeft = level2Left - popoutWidth - gap;

  const fitsRight = menuLeft + rightLeft + popoutWidth <= window.innerWidth - viewportPadding;
  const fitsLeft = menuLeft + leftLeft >= viewportPadding;
  if (fitsRight) return rightLeft;
  if (fitsLeft) return leftLeft;

  const level2PanelLeft = menuLeft + level2Left;
  const level2PanelRight = level2PanelLeft + popoutWidth;
  const spaceRight = window.innerWidth - viewportPadding - level2PanelRight;
  const spaceLeft = level2PanelLeft - viewportPadding;
  return spaceRight >= spaceLeft ? rightLeft : leftLeft;
});

const getPopoutTopFromElement = (el: HTMLElement | undefined, estimatedHeight: number): number => {
  nodeMenuAlignTick.value;
  const cascade = nodePickerCascadeRef.value;
  if (!cascade || !el) return 0;

  const cascadeRect = cascade.getBoundingClientRect();
  const itemRect = el.getBoundingClientRect();
  const rowTop = itemRect.top - cascadeRect.top;
  const rowHeight = itemRect.height || 32;
  const menuTop = Number.parseFloat((smartMenuPosition.value.top || "0").replace("px", "")) || 0;
  const viewportTopPadding = 16;
  const viewportBottomPadding = 16;
  const reservedBottom = showFlowStatusPanel.value
    ? NODE_MENU_BOTTOM_RESERVED_WHEN_STATUS
    : NODE_MENU_BOTTOM_RESERVED_DEFAULT;
  const viewportBottomLimit = window.innerHeight - viewportBottomPadding - reservedBottom;

  const downTop = Math.max(0, rowTop - 2);
  const upTop = Math.max(0, rowTop + rowHeight - estimatedHeight + 2);

  const downBottomY = menuTop + downTop + estimatedHeight;
  const upTopY = menuTop + upTop;
  const anchorViewportY = menuTop + rowTop;
  const availableDown = viewportBottomLimit - anchorViewportY;
  const availableUp = anchorViewportY - viewportTopPadding;

  const fitsDown = downBottomY <= viewportBottomLimit;
  const fitsUp = upTopY >= viewportTopPadding;

  const viewportMax = Math.max(
    0,
    viewportBottomLimit - menuTop - estimatedHeight
  );
  const clampTop = (value: number) => Math.min(Math.max(0, value), viewportMax);

  // Priority: if one direction fully fits, use it.
  if (fitsDown) return clampTop(downTop);
  if (fitsUp) return clampTop(upTop);

  // If neither fits fully, use the side with more room.
  return availableUp > availableDown ? clampTop(upTop) : clampTop(downTop);
};

function openNodeOptionsAtClientPosition(clientX: number, clientY: number) {
  const canvas = document.getElementById("flowchart-canvas");
  if (canvas) {
    const rect = canvas.getBoundingClientRect();
    const worldX = (clientX - rect.left - camera.value.x) / camera.value.scale;
    const worldY = (clientY - rect.top - camera.value.y) / camera.value.scale;

    nodeOptionsPosition.value = {
      x: worldX,
      y: worldY,
      clientX,
      clientY,
    };
  } else {
    nodeOptionsPosition.value = {
      x: 0,
      y: 0,
      clientX,
      clientY,
    };
  }

  activeSelectionCategoryId.value = null;
  activeSelectionSubgroupId.value = null;

  showNodeOptions.value = true;
}

// Update selectNodeFromList to include inputs and outputs
const selectNodeFromList = (type: string) => {
  const nodeType = findNodeTypeByName(type);

  if (!nodeType) {
    console.error(`Node type ${type} not found`);
    return;
  }

  // Check if we can add this trigger node
  if (!canAddTriggerNode(type)) {
    console.warn(
      `Cannot add trigger node ${type}: A trigger node already exists in the flow`
    );
    return;
  }

  // Create new node at the position stored in nodeOptionsPosition
  const newNode = createNodeAtPosition(
    nodeType,
    nodeOptionsPosition.value.x,
    nodeOptionsPosition.value.y
  );
  addNodeToCanvas(newNode);

  // If we have a saved connection source, create a connection to the new node
  if (savedConnectionSource.value) {
    const sourceNode = savedConnectionSource.value.node;
    const sourcePortType = savedConnectionSource.value.type;
    let targetPortType: string | null = null;

    // Helper to safely parse port IDs with hyphens
    const stripPortPrefix = (portType: string, prefix: string) =>
      portType.startsWith(prefix) ? portType.slice(prefix.length) : "";

    // Determine appropriate target port based on source port type
    if (sourcePortType.startsWith("flow-output") && !isValueDrivenNodeType(newNode.type)) {
      targetPortType = "flow-input";
    } else if (sourcePortType.startsWith("value-output-")) {
      const sourceOutputId = stripPortPrefix(sourcePortType, "value-output-");
      const sourceOutput = sourceNode.outputs?.find((output) => output.id === sourceOutputId);
      const targetInputs = newNode.inputs ?? [];

      // Prefer exact type match first, then fallback to wildcard `any` input.
      // Avoid auto-connecting across implicit conversion types (e.g. string -> number),
      // because that feels like a wrong target selection in UI.
      let matchedInput =
        sourceOutput &&
        targetInputs.find(
          (input) =>
            input.type === sourceOutput.type &&
            validateNodeConnection(
              sourceNode,
              newNode,
              sourcePortType,
              `value-input-${input.id}`
            )
        );

      if (!matchedInput) {
        matchedInput = targetInputs.find(
          (input) =>
            input.type === "any" &&
            validateNodeConnection(
              sourceNode,
              newNode,
              sourcePortType,
              `value-input-${input.id}`
            )
        );
      }

      if (matchedInput) {
        targetPortType = `value-input-${matchedInput.id}`;
      }
    }

    if (targetPortType) {
      addConnectionWithValidation(sourceNode, newNode, sourcePortType, targetPortType);
    } else if (sourcePortType === "flow-input" && !isValueDrivenNodeType(newNode.type)) {
      const namedRoutes = getNamedFlowRouteOutputs(newNode.outputs);
      const hasDefaultRoute = (newNode.outputs || []).some(
        (output) => output.type === "flow" && output.id === DEFAULT_FLOW_OUTPUT_ID,
      );
      const newOutputPort =
        hasDefaultRoute
          ? "flow-output"
          : namedRoutes.length === 1
            ? `flow-output-${namedRoutes[0]!.id}`
            : null;
      if (newOutputPort) {
        addConnectionWithValidation(newNode, sourceNode, newOutputPort, sourcePortType);
      } else {
        console.warn("New node has multiple control routes; choose the route to connect", {
          sourceNode: sourceNode.id,
          targetNode: newNode.id,
          routes: namedRoutes.map((route) => route.id),
        });
      }
    } else if (sourcePortType.startsWith("value-input-")) {
      const sourceInputId = stripPortPrefix(sourcePortType, "value-input-");
      const sourceInput = sourceNode.inputs?.find((input) => input.id === sourceInputId);
      const candidateOutputs = (newNode.outputs ?? []).filter((output) => output.type !== "flow");
      const matchedOutput = candidateOutputs.find((output) =>
        sourceInput
          ? validateNodeConnection(
              newNode,
              sourceNode,
              `value-output-${output.id}`,
              sourcePortType
            )
          : false
      );
      if (matchedOutput) {
        addConnectionWithValidation(
          newNode,
          sourceNode,
          `value-output-${matchedOutput.id}`,
          sourcePortType
        );
      } else {
        console.warn("No compatible auto-connection output found for new node", {
          sourceNode: sourceNode.id,
          sourcePortType,
          targetNode: newNode.id,
          targetNodeType: newNode.type,
        });
      }
    } else {
      console.warn("No compatible auto-connection target found for new node", {
        sourceNode: sourceNode.id,
        sourcePortType,
        targetNode: newNode.id,
        targetNodeType: newNode.type,
      });
    }

    // Clear saved connection source
    savedConnectionSource.value = null;
  }

  // Close node options
  closeNodeOptions();
};

// Add the necessary variables
const showNodeOptions = ref(false);
const showTriggerOptions = ref(false);
const triggerChangeNode = ref<Node | null>(null);
const triggerMenuPosition = ref({ x: 0, y: 0 });

// Computed property to get all trigger types (not filtered)
const allTriggerTypes = computed(() => {
  return getSafeNodeTypes().filter((type) => type.category === "TRIGGER");
});

// Add closeNodeOptions function
const closeNodeOptions = () => {
  showNodeOptions.value = false;
  savedConnectionSource.value = null;
  // Reset search term when closing the menu
  nodeSelectionSearch.value = "";
  activeSelectionCategoryId.value = null;
  activeSelectionSubgroupId.value = null;
  for (const k in categoryItemRefs) delete categoryItemRefs[k];
  for (const k in subgroupItemRefs) delete subgroupItemRefs[k];
  nodeMenuAlignTick.value++;
};

// Functions for trigger changing
const showTriggerChangeMenu = (node: Node) => {
  triggerChangeNode.value = node;

  // Show the menu first to be able to measure it
  showTriggerOptions.value = true;

  // Wait for next tick to ensure the menu is rendered
  nextTick(() => {
    // Get the actual DOM element for precise positioning
    const nodeElement = document.querySelector(`[data-node-id="${node.id}"]`);
    const menuElement = document.querySelector(".trigger-menu");

    if (nodeElement && menuElement) {
      const nodeRect = nodeElement.getBoundingClientRect();
      const menuRect = menuElement.getBoundingClientRect();

      const arrowWidth = 8; // Width of the arrow

      triggerMenuPosition.value = {
        x: nodeRect.left - menuRect.width - arrowWidth - 20, // Use actual menu width
        y: nodeRect.top, // Align with the top of the actual DOM element
      };
    } else {
      // Fallback to calculated position if DOM elements not found
      const canvas = document.getElementById("flowchart-canvas");
      if (canvas) {
        const rect = canvas.getBoundingClientRect();
        const nodeX = node.x * camera.value.scale + camera.value.x + rect.left;
        const nodeY = node.y * camera.value.scale + camera.value.y + rect.top;

        const menuWidth = 280; // Fallback width
        const arrowWidth = 8;

        triggerMenuPosition.value = {
          x: nodeX - menuWidth - arrowWidth,
          y: nodeY,
        };
      }
    }
  });
};

const closeTriggerChangeMenu = () => {
  showTriggerOptions.value = false;
  triggerChangeNode.value = null;
};

const changeTriggerType = (newTriggerType: any) => {
  if (!triggerChangeNode.value) return;

  const node = triggerChangeNode.value;
  const oldType = node.type;
  const oldOutputs = [...(node.outputs ?? [])];

  // Store existing connections to preserve them
  const existingConnections = connections.value.filter(
    (conn) => conn.from === node.id || conn.to === node.id
  );

  // Update the node type and related properties
  node.type = newTriggerType.name;
  node.text = newTriggerType.label;
  node.inputs = newTriggerType.inputs ? [...newTriggerType.inputs] : [];
  node.outputs = newTriggerType.outputs ? [...newTriggerType.outputs] : [];

  // Preserve all existing connections - they should remain valid
  // Flow connections use 'flow-output' and 'flow-input' types, not value-output types
  existingConnections.forEach((conn) => {
    // For flow connections from this node, keep them as flow-output
    if (conn.from === node.id && conn.kind === "control") {
      // Flow connections should remain as flow-output, not change to value-output
      // No changes needed - the connection type is already correct
    }

    // For flow connections to this node, keep them as flow-input
    if (conn.to === node.id && conn.kind === "control") {
      // Flow connections should remain as flow-input
      // No changes needed - the connection type is already correct
    }

    // For value connections, try to map them to equivalent ports in the new trigger type
    if (conn.from === node.id && conn.kind === "data") {
      const oldOutput = oldOutputs.find((output) => output.id === conn.fromOutput);

      if (oldOutput) {
        // Try to find a similar output in the new trigger type
        const newOutput = node.outputs?.find(
          (output) => output.name === oldOutput.name || output.type === oldOutput.type
        );

        if (newOutput) {
          conn.fromOutput = newOutput.id;
          conn.fromType = "value-output-" + newOutput.id;
        }
      }
    }
  });

  // Force connection update to refresh positions
  forceConnectionUpdate();

  // Close the menu
  closeTriggerChangeMenu();

  console.log(`Changed trigger from ${oldType} to ${newTriggerType.label}`);
};

// Add event listener to close the node options menu when clicking outside
const handleClickOutside = (event: MouseEvent) => {
  const clickTarget = event.target as Element;
  // Handle settings menu
  if (showSettings.value) {
    // Check if click is on settings button or inside settings dropdown
    const isSettingsButton = clickTarget.closest("#settings-button");
    const isSettingsDropdown = clickTarget.closest(".settings-menu");

    if (!isSettingsButton && !isSettingsDropdown) {
      showSettings.value = false;
    }
  }

  // Handle node options menu
  if (showNodeOptions.value) {
    const menu = document.querySelector(".node-menu");
    if (menu && !menu.contains(clickTarget)) {
      closeNodeOptions();
    }
  }

  // Handle trigger options menu
  if (showTriggerOptions.value) {
    const triggerMenu = document.querySelector(".trigger-menu");
    if (triggerMenu && !triggerMenu.contains(clickTarget)) {
      closeTriggerChangeMenu();
    }
  }
};

const isTypingIntoInput = (target: EventTarget | null): boolean => {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;

  const tag = target.tagName?.toLowerCase();
  if (tag === "input" || tag === "textarea" || tag === "select") return true;

  return Boolean(target.closest('input, textarea, select, [contenteditable="true"]'));
};

// Add a method to handle keyboard events
const handleKeyDown = (event: KeyboardEvent) => {
  // Track Ctrl key
  if (event.key === "Control") {
    isCtrlPressed.value = true;
  }

  if (
    !event.ctrlKey &&
    !event.metaKey &&
    !event.altKey &&
    (event.key === "i" || event.key === "I") &&
    !isTypingIntoInput(event.target)
  ) {
    event.preventDefault();
    toggleInspector();
    return;
  }

  // Delete the selected node when Delete key is pressed
  if (event.key === "Delete") {
    deleteSelection();
  }

  // Clear selection with Escape
  if (event.key === "Escape") {
    clearSelection();
  }
};

const handleKeyUp = (event: KeyboardEvent) => {
  // Track Ctrl key release
  if (event.key === "Control") {
    isCtrlPressed.value = false;
  }
};

// Helper function to check if a node is within the selection rectangle
const isNodeInSelection = (node: Node): boolean => {
  const start = selectionStart.value;
  const end = selectionEnd.value;

  const selectionLeft = Math.min(start.x, end.x);
  const selectionTop = Math.min(start.y, end.y);
  const selectionRight = Math.max(start.x, end.x);
  const selectionBottom = Math.max(start.y, end.y);

  // Get node dimensions
  const nodeDimensions = getNodeDimensions(node.id);
  const nodeLeft = node.x;
  const nodeTop = node.y;
  const nodeRight = node.x + nodeDimensions.width;
  const nodeBottom = node.y + nodeDimensions.height;

  // More generous selection - node is selected if any part overlaps with selection
  // OR if the selection box is small but touches the node center
  const hasOverlap = !(
    nodeRight < selectionLeft ||
    nodeLeft > selectionRight ||
    nodeBottom < selectionTop ||
    nodeTop > selectionBottom
  );

  // For small selection boxes, check if they're near the node center
  const selectionWidth = Math.abs(selectionRight - selectionLeft);
  const selectionHeight = Math.abs(selectionBottom - selectionTop);
  const isSmallSelection = selectionWidth < 50 && selectionHeight < 50;

  if (isSmallSelection) {
    const nodeCenterX = nodeLeft + nodeDimensions.width / 2;
    const nodeCenterY = nodeTop + nodeDimensions.height / 2;
    const selectionCenterX = (selectionLeft + selectionRight) / 2;
    const selectionCenterY = (selectionTop + selectionBottom) / 2;

    // Check if selection center is close to node center
    const distance = Math.sqrt(
      Math.pow(nodeCenterX - selectionCenterX, 2) +
        Math.pow(nodeCenterY - selectionCenterY, 2)
    );

    return distance < Math.max(nodeDimensions.width, nodeDimensions.height) / 2 + 25;
  }

  return hasOverlap;
};

// Add back the stopPan function that was accidentally removed
const stopPan = () => {
  isPanning.value = false;
};

// These are VortiDeck AI-node presets, not MCP prompt resources. Keep their
// ownership explicit until remote MCP prompts have a real product surface.
const loadAvailableTemplates = async () => {
  availableTemplates.value = [
    {
      id: "conversational",
      name: "Conversational",
      description: "Natural conversation style",
      category: "Conversation",
    },
    {
      id: "direct_execution",
      name: "Direct Execution",
      description: "Execute the task directly",
      category: "Task Execution",
    },
  ];
};

// Initialize
onMounted(async () => {
  try {
    // Register core widgets FIRST (before anything else)
    registerCoreWidgets();

    // Show window
    await getCurrentWindow().show();

    // Register MCP listeners FIRST to catch any servers connecting during app launch
    unlistenMcpConnected = await listen("mcp-server-connected", async (event) => {
      console.log("📡 MCP server connected, refreshing node library:", event);
      await loadBackendNodes();
    });

    unlistenMcpDisconnected = await listen("mcp-server-disconnected", async (event) => {
      console.log("📡 MCP server disconnected, refreshing node library:", event);
      await loadBackendNodes();
    });

    // Listen for plugin changes (hot reload, install, uninstall)
    unlistenPluginChanged = await listen("plugin-changed", async (event) => {
      console.log("🔌 Plugin changed, refreshing node library:", event);
      await loadBackendNodes();
    });

    // Load all nodes from backend - initial load
    await loadBackendNodes();

    // 🔄 Retry after 1.5 seconds to catch MCP tools that connect during app launch
    // This handles the race condition where MCP connects before the flow page loads
    setTimeout(async () => {
      console.log("🔄 Refreshing node library (catching any MCP tools from app launch)");
      await loadBackendNodes();
    }, 1500);

    // Load available templates
    await loadAvailableTemplates();

    // Initialize all categories as expanded
    expandedCategories.value["TRIGGER"] = true;
    expandedCategories.value["CONTROL"] = true;
    expandedCategories.value["AI"] = true;
    expandedCategories.value["AUDIO"] = true;

    // Add event listener to close the node options menu when clicking outside
    document.addEventListener("mousedown", handleClickOutside);

    // Add keydown and keyup event listeners for keyboard shortcuts
    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("keyup", handleKeyUp);
    window.addEventListener("resize", updateCanvasViewport);

    await nextTick();
    updateCanvasViewport();
    const canvas = getCanvasElement();
    if (canvas && typeof ResizeObserver !== "undefined") {
      canvasResizeObserver = new ResizeObserver(() => {
        updateCanvasViewport();
      });
      canvasResizeObserver.observe(canvas);
    }

    // Initialize flow executor
    await flowManager.initializeExecutor();
    console.log("✅ Flow executor initialized successfully");
  } catch (error) {
    console.error("❌ Failed to initialize flow executor:", error);
  }
});

// Clean up event listener when component is unmounted
onUnmounted(() => {
  document.removeEventListener("mousedown", handleClickOutside);

  window.removeEventListener("keydown", handleKeyDown);
  window.removeEventListener("keyup", handleKeyUp);
  window.removeEventListener("resize", updateCanvasViewport);
  clearDOMCache();
  if (canvasResizeObserver) {
    canvasResizeObserver.disconnect();
    canvasResizeObserver = null;
  }

  // Clean up performance optimization timers and animation frames
  if (updateTimeout) {
    clearTimeout(updateTimeout);
  }
  if (animationFrameId) {
    cancelAnimationFrame(animationFrameId);
  }
  if (expensiveOperationTimeout) {
    clearTimeout(expensiveOperationTimeout);
  }

  // Clean up MCP event listeners
  if (unlistenMcpConnected) {
    unlistenMcpConnected();
  }
  if (unlistenMcpDisconnected) {
    unlistenMcpDisconnected();
  }

  // Clean up plugin event listener
  if (unlistenPluginChanged) {
    unlistenPluginChanged();
  }
});

//todo:

// Connection Path Instability: During zoom operations, connection paths between nodes shift position slightly. This happens because the current implementation:
// Uses a mix of DOM-based positioning and mathematical calculations
// Doesn't maintain consistent coordinate systems between world space and screen space
// Has potential rounding errors from fractional pixel positions
// May recalculate positions inconsistently after zoom operations

// Add state for panels & flows
const showFlowStatusPanel = ref(true);
const showRuntimeOperations = ref(false);
const flowStartPending = ref(false);
const showPreparationModal = ref(false);
const preparationMode = ref<"rehearsal" | "import">("rehearsal");
const preparationLoading = ref(false);
const preparationError = ref("");
const rehearsalReport = ref<FlowRehearsalReport | null>(null);
const recipeImportPreview = ref<FlowRecipeImportPreview | null>(null);
const durableRecoveryRuns = ref<FlowExecutionState[]>([]);
const recoveryDecisionPending = ref(false);
const recoveryDecisionError = ref("");
const showNodeLibrary = ref(true); // Node library is shown by default
const flows = ref<any[]>([]); // Saved flows from database
const temporaryFlows = ref<any[]>([]); // Temporary/unsaved flows

// Add unsaved changes detection
const hasUnsavedChanges = ref(false);
const showUnsavedChangesModal = ref(false);
const pendingFlowToLoad = ref<any>(null);
const pendingFlowToDelete = ref<any>(null); // Flow pending deletion
const pendingAction = ref<"load" | "new" | "delete" | "recipe-import" | null>(null); // Track pending action type
const lastSavedState = ref<string>("");

// Computed properties for confirmation modal
const getConfirmationMessage = computed(() => {
  if (pendingAction.value === "new") {
    return "Creating a new flow will discard your current progress. Are you sure you want to continue?";
  }
  if (pendingAction.value === "delete") {
    return `Are you sure you want to delete the flow "${pendingFlowToDelete.value?.name}"? This action cannot be undone.`;
  }
  if (pendingAction.value === "recipe-import") {
    return "Importing this recipe will replace the unsaved flow currently on the canvas. Continue?";
  }
  return "Loading a new flow will discard your current progress. Are you sure you want to continue?";
});

const getConfirmationButtonText = computed(() => {
  if (pendingAction.value === "new") return "Create New Flow";
  if (pendingAction.value === "delete") return "Delete Flow";
  if (pendingAction.value === "recipe-import") return "Import Recipe";
  return "Load Flow";
});

// Add edit flow metadata modal state
const showEditFlowMetadataModal = ref(false);

// Add save flow modal state
const showSaveFlowModal = ref(false);
const pendingSaveAsOriginalState = ref(false);

// Track current flow metadata
const currentFlowMetadata = ref<{
  id: string | null;
  name: string | null;
  description: string | null;
  isNew: boolean;
  isActive: boolean;
}>({
  id: null,
  name: null,
  description: null,
  isNew: true,
  isActive: false,
});
const loadedFlowDefinition = ref<FlowDefinition | null>(null);

// Native saving may protect literal credentials with immutable references.
// Keep the editor's next revision and HTTP fields aligned with that persisted
// document without rebuilding nodes or changing geometry/selection.
async function acceptSavedFlow(flowId: string) {
  const saved = await invoke<FlowDefinition>('load_flow', { flowId });
  loadedFlowDefinition.value = saved;
  const definitions = new Map(saved.nodes.map(node => [node.id, node]));
  for (const node of nodes.value) {
    const definition = definitions.get(persistedEditorNodeId(node));
    if (!definition) continue;
    node.persistedDefinition = definition;
    if (definition.nodeType !== 'http_request') continue;
    for (const input of node.inputs || []) {
      const key = input.id || input.name;
      if ((key === 'authentication' || key === 'headers') && definition.inputs[key] !== undefined) {
        input.value = structuredClone(definition.inputs[key]);
      }
    }
  }
}

// Track changes to detect unsaved state
const updateUnsavedChanges = () => {
  try {
    const currentState = serializeEditableFlowState();
    hasUnsavedChanges.value = currentState !== lastSavedState.value;
  } catch (error) {
    console.error("Error checking unsaved changes:", error);
  }
};

// Update saved state when flow is saved or loaded
const updateSavedState = () => {
  try {
    lastSavedState.value = serializeEditableFlowState();
    hasUnsavedChanges.value = false;
  } catch (error) {
    console.error("Error updating saved state:", error);
  }
};

// Create a new flow (reset metadata) - with unsaved changes protection
const createNewFlow = () => {
  if (hasUnsavedChanges.value) {
    // Show confirmation modal for new flow
    pendingAction.value = "new";
    showUnsavedChangesModal.value = true;
  } else {
    performNewFlow();
  }
};

// Actually create the new flow
const performNewFlow = () => {
  // Generate permanent ID for the new flow
  const flowId = `flow-${Date.now()}`;

  currentFlowMetadata.value = {
    id: flowId,
    name: "Untitled Flow",
    description: null,
    isNew: true,
    isActive: false,
  };
  loadedFlowDefinition.value = null;

  // Add temporary flow to the flows list for visual indication
  const tempFlow = {
    id: flowId,
    name: "Untitled Flow",
    description: "",
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    version: "1.0.0",
    isTemporary: true,
    isUnsaved: true,
  };

  // Remove ALL existing temporary flows first (only one temp flow allowed at a time)
  temporaryFlows.value = [];
  // Add the new temporary flow
  temporaryFlows.value.push(tempFlow);

  nodes.value = [];
  connections.value = [];
  clearSelection();
  updateSavedState();
};

// Initialize the default flow entry when the editor opens
const initializeDefaultFlow = () => {
  // Only create if we don't already have a temporary flow and metadata indicates new flow
  if (temporaryFlows.value.length === 0 && currentFlowMetadata.value.isNew) {
    const flowId = `flow-${Date.now()}`;

    // Update current flow metadata to include the flow ID
    currentFlowMetadata.value = {
      id: flowId,
      name: "Untitled Flow",
      description: null,
      isNew: true,
      isActive: false,
    };

    // Add temporary flow to the flows list for visual indication
    const tempFlow = {
      id: flowId,
      name: "Untitled Flow",
      description: "",
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      version: "1.0.0",
      isTemporary: true,
      isUnsaved: true,
    };

    // Add the temporary flow at the beginning
    temporaryFlows.value.unshift(tempFlow);

    console.log("✅ Default temporary flow created for flows menu");
  }
};

// ============================================================================
// VARIABLE PANEL HANDLERS
// ============================================================================

// When user selects a variable from the Variables panel
const onSelectVariable = (variableName: string) => {
  // If a GetVariable node is selected, set its variableName input
  if (selectedNode.value) {
    const node = selectedNode.value;
    if (
      node.type.toLowerCase() === "get-variable" ||
      node.type.toLowerCase() === "getvariable" ||
      node.type === "GetVariable"
    ) {
      const input = node.inputs?.find(
        (i: any) => i.id === "variableName" || i.name === "Variable Name"
      );
      if (input) {
        input.value = variableName;
        console.log(`✅ Set variable name "${variableName}" on GetVariable node`);

        // Trigger variable node type sync on name change
        canvasRef.value?.onVariableNameChange(node.id);
      }
    }
  }
};

// When user wants to highlight/locate a SetVariable node
const onHighlightNode = (nodeId: number) => {
  const node = nodes.value.find((n) => n.id === nodeId);
  if (node) {
    // Select the node
    selectNode(node);
    // Center camera on the node
    camera.value = {
      x: -node.x + window.innerWidth / 2 - 150,
      y: -node.y + window.innerHeight / 2 - 50,
      scale: camera.value.scale,
    };
    console.log(`📍 Highlighted node ${nodeId}`);
  }
};

// Extract variable names from SetVariable nodes for GetVariable dropdowns
const extractVariableNames = (): string[] => {
  const names: string[] = [];
  for (const node of nodes.value) {
    if (
      node.type.toLowerCase() === "set-variable" ||
      node.type.toLowerCase() === "setvariable" ||
      node.type === "SetVariable"
    ) {
      const input = node.inputs?.find(
        (i: any) => i.id === "variableName" || i.name === "Variable Name"
      );
      if (input?.value && typeof input.value === "string" && input.value.trim()) {
        names.push(input.value.trim());
      }
    }
  }
  return [...new Set(names)]; // unique names
};

// Update GetVariable nodes with available variable names as enum options
const updateGetVariableEnums = () => {
  const variableNames = extractVariableNames();

  for (const node of nodes.value) {
    if (
      node.type.toLowerCase() === "get-variable" ||
      node.type.toLowerCase() === "getvariable" ||
      node.type === "GetVariable"
    ) {
      const input = node.inputs?.find(
        (i: any) => i.id === "variableName" || i.name === "Variable Name"
      );
      if (input) {
        // Keep currently selected value available even if no Set Variable currently defines it.
        const currentValue =
          typeof input.value === "string" && input.value.trim().length > 0
            ? input.value.trim()
            : null;

        const options = currentValue
          ? Array.from(new Set([currentValue, ...variableNames]))
          : variableNames;

        // Use a dropdown when we have known variables, fallback to text otherwise.
        (input as any).enum = options.length > 0 ? options : undefined;
        input.widget = options.length > 0 ? "select" : "text";
        input.widgetOptions = {
          ...(input.widgetOptions || {}),
          options,
        };
      }
    }
  }
};

const handleCanvasContextMenu = (event: MouseEvent) => {
  // Don't open node menu while using box-selection gesture.
  if (event.shiftKey || isSelecting.value) {
    return;
  }

  // Ignore if this was the tail end of a drag-select interaction.
  if (justFinishedSelecting.value) return;

  const clickTarget = event.target as Element | null;
  if (!clickTarget) return;

  // Empty-canvas context menu only.
  if (clickTarget.closest(".node") || clickTarget.closest(".flow-node-actions-menu")) {
    return;
  }

  if (isConnecting.value) {
    cancelConnection();
  }

  savedConnectionSource.value = null;
  openNodeOptionsAtClientPosition(event.clientX, event.clientY);
};

// Add toggle functions
const toggleSettings = () => {
  showSettings.value = !showSettings.value;
};

const toggleSnapToGrid = () => {
  snapToGrid.value = !snapToGrid.value;
};

const toggleShowGrid = () => {
  showGrid.value = !showGrid.value;
};

const closeFlowStatusPanel = () => {
  showFlowStatusPanel.value = false;

  // Clear the execution state if flow is completed/failed/cancelled
  if (activeFlowExecution.value && !isFlowRunning.value) {
    console.log("🧹 Clearing completed flow execution state");
    activeFlowExecution.value = null;
  }
};

const failActiveRecovery = async () => {
  const runId = activeFlowExecution.value?.runId || activeFlowExecution.value?.flowId;
  if (!runId) return;
  recoveryDecisionPending.value = true;
  recoveryDecisionError.value = "";
  try {
    await invoke("fail_flow_recovery_run", {
      runId,
      reason: "Operator selected fail safely from the flow editor",
    });
    durableRecoveryRuns.value = await invoke("list_flow_recovery_runs") as FlowExecutionState[];
    if (durableRecoveryRuns.value.length > 0) {
      activeFlowExecution.value = durableRecoveryRuns.value[0] ?? null;
    } else {
      const status = await invoke("get_flow_execution_status", { flowId: runId });
      activeFlowExecution.value = status as typeof activeFlowExecution.value;
    }
  } catch (cause) {
    recoveryDecisionError.value = `Could not fail the recovery run safely: ${String(cause)}`;
  } finally {
    recoveryDecisionPending.value = false;
  }
};

const resumeActiveRecovery = async () => {
  const runId = activeFlowExecution.value?.runId || activeFlowExecution.value?.flowId;
  if (!runId) return;
  recoveryDecisionPending.value = true;
  recoveryDecisionError.value = "";
  try {
    await invoke("resume_flow_recovery_run", { runId });
    durableRecoveryRuns.value = await invoke("list_flow_recovery_runs") as FlowExecutionState[];
    const status = await invoke("get_flow_execution_status", { flowId: runId });
    activeFlowExecution.value = status as typeof activeFlowExecution.value;
  } catch (recoveryError) {
    recoveryDecisionError.value = `Safe resume was rejected: ${String(recoveryError)}`;
    console.error("Recovery resume was rejected because it is not proven safe:", recoveryError);
  } finally {
    recoveryDecisionPending.value = false;
  }
};

// // Auto-show status panel when flow starts

const handleDeleteClick = (node: Node) => {
  // This action comes from explicit node menu click; don't gate on drag state.
  deleteNode(node);
};

const toggleNodeMinimized = (nodeId: number) => {
  const node = nodes.value.find((item) => item.id === nodeId);
  if (!node) return;
  node.minimized = !node.minimized;
  // Recompute paths after DOM/layout settles so anchors are measured correctly.
  nextTick(() => {
    forceConnectionUpdate();
    requestAnimationFrame(() => {
      forceConnectionUpdate();
    });
  });
};

const handleAiModelClick = (node: Node) => {
  // This action comes from explicit node menu click; don't gate on drag state.
  openAiModelSelector(node);
};

// Search state variables
const nodeLibrarySearch = ref("");
const nodeSelectionSearch = ref("");
const isSelectionSearchMode = computed(() => nodeSelectionSearch.value.trim().length > 0);

const normalizeSearchText = (value: unknown): string =>
  String(value || "")
    .toLowerCase()
    .trim();

const tokenizeSearchText = (value: string): string[] =>
  value
    .split(/[\s_\-:/|.]+/)
    .map((part) => part.trim())
    .filter(Boolean);

const selectionSearchCandidates = computed(() => {
  const merged = hasExistingTriggerNode()
    ? getSafeNodeTypes().filter((item) => item.category !== "TRIGGER")
    : getSafeNodeTypes();
  const uniqueByName = new Map<string, any>();
  for (const item of merged) {
    if (!item?.name) continue;
    if (!uniqueByName.has(item.name)) uniqueByName.set(item.name, item);
  }

  return Array.from(uniqueByName.values());
});

const scoreNodeForSearch = (nodeType: any, fullQuery: string, queryTerms: string[]): number => {
  if (!fullQuery || queryTerms.length === 0) return 0;

  const name = normalizeSearchText(nodeType.name);
  const label = normalizeSearchText(nodeType.label);
  const category = normalizeSearchText(nodeType.category);
  const categoryDisplay = normalizeSearchText(nodeType.category_display_name);
  const subcategory = normalizeSearchText(nodeType.subcategory);
  const description = normalizeSearchText(nodeType.description);

  const fields = [label, name, categoryDisplay, subcategory, category, description];
  const tokenFields = [
    ...tokenizeSearchText(label),
    ...tokenizeSearchText(name),
    ...tokenizeSearchText(categoryDisplay),
    ...tokenizeSearchText(subcategory),
    ...tokenizeSearchText(category),
  ];

  let score = 0;
  let matchedTerms = 0;

  if (label === fullQuery || name === fullQuery) score += 600;
  else if (label.startsWith(fullQuery)) score += 420;
  else if (name.startsWith(fullQuery)) score += 380;
  else if (tokenFields.some((token) => token.startsWith(fullQuery))) score += 280;

  if (categoryDisplay.startsWith(fullQuery) || category.startsWith(fullQuery)) score += 120;

  for (const term of queryTerms) {
    let termScore = 0;

    if (label === term || name === term) termScore = Math.max(termScore, 180);
    if (label.startsWith(term)) termScore = Math.max(termScore, 150);
    if (name.startsWith(term)) termScore = Math.max(termScore, 140);
    if (tokenFields.some((token) => token.startsWith(term))) termScore = Math.max(termScore, 110);
    if (fields.some((field) => field.includes(term))) termScore = Math.max(termScore, 80);
    if (description.includes(term)) termScore = Math.max(termScore, 45);

    if (termScore > 0) matchedTerms++;
    score += termScore;
  }

  if (matchedTerms === 0) return 0;
  score += matchedTerms * 25;

  return score;
};

const searchMatchedNodes = computed(() => {
  if (!isSelectionSearchMode.value) return [];

  const fullQuery = normalizeSearchText(nodeSelectionSearch.value);
  const queryTerms = tokenizeSearchText(fullQuery).slice(0, 6);
  if (!fullQuery || queryTerms.length === 0) return [];

  return selectionSearchCandidates.value
    .map((nodeType) => ({
      nodeType,
      score: scoreNodeForSearch(nodeType, fullQuery, queryTerms),
    }))
    .filter((item) => item.score > 0)
    .sort((a, b) => {
      if (b.score !== a.score) return b.score - a.score;
      const labelA = String(a.nodeType.label || a.nodeType.name || "").toLowerCase();
      const labelB = String(b.nodeType.label || b.nodeType.name || "").toLowerCase();
      return labelA.localeCompare(labelB);
    })
    .slice(0, 40)
    .map((item) => item.nodeType);
});

const selectFirstSearchMatch = () => {
  if (!isSelectionSearchMode.value) return;
  const first = searchMatchedNodes.value[0];
  if (!first?.name) return;
  selectNodeFromList(first.name);
};

// Filtered computed properties for node library
const filteredTriggerNodes = computed(() => {
  if (!nodeLibrarySearch.value) return triggerCategoryNodeTypes.value;
  return createSearchFilter(triggerCategoryNodeTypes.value, nodeLibrarySearch.value);
});

const filteredControlNodes = computed(() => {
  if (!nodeLibrarySearch.value) return controlCategoryNodeTypes.value;
  return createSearchFilter(controlCategoryNodeTypes.value, nodeLibrarySearch.value);
});

const filteredAiNodes = computed(() => {
  if (!nodeLibrarySearch.value) return aiCategoryNodeTypes.value;
  return createSearchFilter(aiCategoryNodeTypes.value, nodeLibrarySearch.value);
});

const filteredAudioNodes = computed(() => {
  if (!nodeLibrarySearch.value) return audioCategoryNodeTypes.value;
  return createSearchFilter(audioCategoryNodeTypes.value, nodeLibrarySearch.value);
});

const activeSelectionCategoryId = ref<string | null>(null);
const activeSelectionSubgroupId = ref<string | null>(null);

const selectionMenuCategories = computed(() => buildNodeCatalog(selectionSearchCandidates.value));

const activeSelectionCategory = computed(() => {
  if (selectionMenuCategories.value.length === 0) return null;
  return selectionMenuCategories.value.find(
    (category) => category.id === activeSelectionCategoryId.value
  ) || null;
});

const activeCategorySubgroups = computed<Array<{ id: string; label: string; items: any[] }>>(() => {
  const category = activeSelectionCategory.value;
  if (!category) return [];
  return category.groups;
});

const activeSelectionSubgroup = computed(() => {
  if (activeCategorySubgroups.value.length === 0) return null;
  return activeCategorySubgroups.value.find(
    (group) => group.id === activeSelectionSubgroupId.value
  ) || null;
});

const shouldSkipSubgroupLevel = computed(() => {
  if (!activeSelectionCategory.value) return false;
  if (activeCategorySubgroups.value.length !== 1) return false;

  const onlyGroupId = String(activeCategorySubgroups.value[0]?.id || "").toLowerCase();
  return onlyGroupId === "general";
});

const showSubgroupPopout = computed(() =>
  Boolean(activeSelectionCategory.value && !shouldSkipSubgroupLevel.value)
);

const showNodesPopoutLevel2 = computed(() =>
  Boolean(activeSelectionCategory.value && shouldSkipSubgroupLevel.value)
);

const showNodesPopoutLevel3 = computed(() =>
  Boolean(!shouldSkipSubgroupLevel.value && activeSelectionSubgroup.value)
);

const visibleSelectionNodes = computed(() => {
  if (shouldSkipSubgroupLevel.value) {
    return sanitizeNodeTypeItems(activeCategorySubgroups.value[0]?.items || []);
  }
  return sanitizeNodeTypeItems(activeSelectionSubgroup.value?.items || []);
});

const level2PopoutStyle = computed(() => {
  const selectedCategoryId = activeSelectionCategoryId.value;
  const anchor = resolveCategoryAnchorElement(selectedCategoryId);
  const estimatedHeight = estimatePopoutHeight(
    shouldSkipSubgroupLevel.value
      ? visibleSelectionNodes.value.length
      : activeCategorySubgroups.value.length
  );
  return {
    left: `${level2LeftPx.value}px`,
    top: `${getPopoutTopFromElement(anchor, estimatedHeight)}px`,
  };
});

const level3PopoutStyle = computed(() => {
  const selectedSubgroupId = activeSelectionSubgroupId.value;
  const anchor = resolveSubgroupAnchorElement(selectedSubgroupId);
  const estimatedHeight = estimatePopoutHeight(visibleSelectionNodes.value.length);
  return {
    left: `${level3LeftPx.value}px`,
    top: `${getPopoutTopFromElement(anchor, estimatedHeight)}px`,
  };
});

const handleSelectCategory = (categoryId: string) => {
  activeSelectionCategoryId.value = categoryId;
  activeSelectionSubgroupId.value = null;
  nodeMenuAlignTick.value++;
};

const handleSelectSubgroup = (groupId: string) => {
  activeSelectionSubgroupId.value = groupId;
  nodeMenuAlignTick.value++;
};

watch(selectionMenuCategories, (categories) => {
  if (categories.length === 0) {
    activeSelectionCategoryId.value = null;
    activeSelectionSubgroupId.value = null;
    return;
  }

  const selectedExists = categories.some(
    (category) => category.id === activeSelectionCategoryId.value
  );
  if (!selectedExists) {
    activeSelectionSubgroupId.value = null;
    activeSelectionCategoryId.value = null;
  }
});

watch(activeSelectionCategoryId, () => {
  activeSelectionSubgroupId.value = null;
  nodeMenuAlignTick.value++;
});

watch(activeSelectionSubgroupId, () => {
  nodeMenuAlignTick.value++;
});

// Combine saved flows with temporary flows for display in NodeLibrary
const allFlowsWithTemporary = computed(() => {
  return [...temporaryFlows.value, ...flows.value];
});

const clearNodeLibrarySearch = () => {
  nodeLibrarySearch.value = "";
};

// Centralized port hover management
const setPortHover = (port: HTMLElement) => {
  // Clear any previous hover states
  if (hoveredPort.value && hoveredPort.value !== port) {
    hoveredPort.value.classList.remove("hovered-port");
  }

  // Set new hover state
  hoveredPort.value = port;
  port.classList.add("hovered-port");
};

const clearPortHover = () => {
  if (hoveredPort.value) {
    hoveredPort.value.classList.remove("hovered-port");
    hoveredPort.value = null;
  }
};

const clearAllPortHovers = () => {
  // Remove hover classes from all ports
  document.querySelectorAll(".hovered-port").forEach((port) => {
    port.classList.remove("hovered-port");
  });
  hoveredPort.value = null;
};

// Centralized function to clear all selections
const clearAllSelections = () => {
  clearSelection();
  clearAllPortHovers();
};

// ============================================================================
// BACKEND FLOW EXECUTION LOGIC
// ============================================================================

// Removed legacy frontend execution - using backend only

// ============================================================================
// UNIFIED FLOW EXECUTION LOGIC
// ============================================================================

// Use the unified flow manager

// Initialize executor on mount
onMounted(async () => {
  try {
    await flowManager.initializeExecutor();
    console.log("✅ Flow executor initialized successfully");
    durableRecoveryRuns.value = await invoke("list_flow_recovery_runs") as FlowExecutionState[];
    if (durableRecoveryRuns.value.length > 0) {
      activeFlowExecution.value = durableRecoveryRuns.value[0] ?? null;
      showFlowStatusPanel.value = true;
      console.warn("⚠️ A workflow requires recovery review", durableRecoveryRuns.value[0]);
    }
  } catch (error) {
    console.error("❌ Failed to initialize flow executor:", error);
  }

  // Load flows from database
  await loadFlows();

  // Listen for flow updates from deck page
  await listen("flows_updated", async (event) => {
    console.log("📡 Flows updated event received in flow editor:", event);
    await loadFlows();
  });

  // Listen for requests to load flows from deck page
  await listen<Record<string, any>>("load_flow_request", async (event) => {
    console.log("📡 Load flow request received:", event);
    const flowToLoad = event.payload;

    if (hasUnsavedChanges.value) {
      // Show confirmation modal, marking this as a deck page flow
      pendingFlowToLoad.value = { ...flowToLoad, source: "deck" };
      pendingAction.value = "load";
      showUnsavedChangesModal.value = true;
    } else {
      // Load directly
      await loadFlowFromDeckPage(flowToLoad);
    }
  });

  const initialFlowId = Array.isArray(route.query.flowId)
    ? route.query.flowId[0]
    : route.query.flowId;
  if (typeof initialFlowId === "string" && initialFlowId.length > 0) {
    await loadFlowFromDeckPage({ id: initialFlowId, name: "Selected flow" });
  } else {
    // Initialize saved state and create the default temporary flow only when no
    // saved flow was requested by the window that opened the editor.
    updateSavedState();
    initializeDefaultFlow();
  }
});

// Loading a persisted flow performs several reactive normalization passes (for
// example, rebuilding variable dropdown options). Those are hydration work,
// not user edits, so dirty-state tracking must wait until they have settled.
const isHydratingFlow = ref(false);

// Dirty state uses the same semantic document as Run/Save/Export. The revision
// payload deliberately excludes only the volatile updatedAt timestamp.
const serializeEditableFlowState = () =>
  flowRevisionPayload(createCanonicalFlowDocument());

const createCanonicalFlowDocument = (options: {
  newIdentity?: boolean;
  id?: string;
  name?: string;
  description?: string;
} = {}): FlowDefinition => {
  const triggerNode = findTriggerNodes()[0];
  const id = options.id
    || (options.newIdentity ? `flow-${Date.now()}` : currentFlowMetadata.value.id)
    || `flow-${Date.now()}`;
  const persistedIdByCanvasId = new Map(
    nodes.value.map((node) => [node.id, persistedEditorNodeId(node)]),
  );
  const resolveCanvasNodeId = (canvasId: number): string => {
    const persistedId = persistedIdByCanvasId.get(canvasId);
    if (!persistedId) throw new Error(`Connection references missing canvas node "${canvasId}".`);
    return persistedId;
  };

  return buildFlowDocument({
    id,
    name: options.name ?? currentFlowMetadata.value.name ?? "Untitled Flow",
    description:
      options.description ?? currentFlowMetadata.value.description ?? undefined,
    isActive: currentFlowMetadata.value.isActive,
    nodes: nodes.value,
    connections: connections.value.map((connection) =>
      canvasConnectionToFlowConnection(connection, resolveCanvasNodeId)
    ),
    startNodeId: triggerNode
      ? persistedEditorNodeId(triggerNode)
      : nodes.value[0]
        ? persistedEditorNodeId(nodes.value[0])
        : "1",
    canvasSettings: {
      zoom: Number(camera.value.scale),
      panX: Number(camera.value.x),
      panY: Number(camera.value.y),
      gridEnabled: true,
      snapToGrid: false,
    },
    now: new Date().toISOString(),
    baseFlow: loadedFlowDefinition.value,
    newIdentity: options.newIdentity,
  });
};

const validateCurrentFlowDocument = (forExecution: boolean) => {
  const editorValidation = forExecution
    ? validateFlowForExecution()
    : validateFlowForSaving();
  const documentValidation = flowManager.validateFlow(createCanonicalFlowDocument());

  return {
    isValid: editorValidation.isValid && documentValidation.isValid,
    errors: [...new Set([...editorValidation.errors, ...documentValidation.errors])],
    warnings: [...new Set([...editorValidation.warnings, ...documentValidation.warnings])],
  };
};

const refreshFlowActionHealth = async (flow: FlowDefinition): Promise<FlowActionHealth[]> => {
  const health = await invoke<FlowActionHealth[]>("preflight_flow_actions", { flow });
  const byNodeId = new Map(health.map((item) => [item.nodeId, item.preflight]));
  for (const node of nodes.value) {
    if (node.type === "invoke-action") {
      node.actionPreflight = byNodeId.get(node.persistedDefinition?.id ?? String(node.id));
    }
  }
  return health;
};

const prepareCurrentFlow = async (): Promise<FlowRehearsalReport> => {
  const flow = createCanonicalFlowDocument();
  const actionHealth = await refreshFlowActionHealth(flow);
  const graph = validateCurrentFlowDocument(true);
  return buildReadOnlyRehearsal(flow, graph, actionHealth, await flowRevision(flow));
};

const rehearseFlowSafely = async () => {
  preparationMode.value = "rehearsal";
  preparationError.value = "";
  rehearsalReport.value = null;
  recipeImportPreview.value = null;
  preparationLoading.value = true;
  showPreparationModal.value = true;
  try {
    rehearsalReport.value = await prepareCurrentFlow();
  } catch (error) {
    preparationError.value = `Rehearsal failed: ${String(error)}`;
  } finally {
    preparationLoading.value = false;
  }
};

// Execute flow using unified format - SIMPLIFIED (No manual conversions)
const showRunInputs = ref(false);
const runInputSchema = ref<Record<string, unknown>>({});
const runInputDefaults = ref<Record<string, unknown>>({});
const runInputError = ref('');
const submitFlowInputs = async (values: Record<string, unknown>) => {
  if (Object.values(values).some(value => value !== null && !['string', 'number', 'boolean'].includes(typeof value))) {
    runInputError.value = 'Flow inputs must be strings, numbers, booleans or null.';
    return;
  }
  await executeFlowWithUnified(values as Record<string, InvocationScalar>);
};

const executeFlowWithUnified = async (inputs?: Record<string, InvocationScalar>) => {
  if (isFlowRunning.value || flowStartPending.value) {
    console.warn("Flow is already executing");
    return;
  }

  if (nodes.value.length === 0) {
    console.warn("No nodes to execute");
    mainStore.addNotification(
      {
        title: "Cannot Execute Flow",
        message: "No nodes to execute! Please add some nodes to the flow first.",
        type: "warning",
        autoClose: true,
        duration: 5000,
        buttonText: "",
        buttonFunction: () => {},
        color: "bg-orange-500",
      },
      currentWindowId
    );
    return;
  }

  flowStartPending.value = true;
  try {
    const unifiedFlow = createCanonicalFlowDocument();
    if (inputs === undefined) {
      const snapshot = await invoke<InputContractSnapshot>('get_action_input_contract', {
        reference: { kind: 'flow', id: unifiedFlow.id },
      });
      if (snapshot.contract && Object.keys(snapshot.contract.inputSchema.properties).length) {
        runInputSchema.value = snapshot.contract.inputSchema;
        runInputDefaults.value = Object.fromEntries(Object.entries(snapshot.contract.inputSchema.properties)
          .filter(([, schema]) => Object.hasOwn(schema, 'default')).map(([name, schema]) => [name, schema.default]));
        runInputError.value = '';
        showRunInputs.value = true;
        return;
      }
    }
    const actionHealth = await refreshFlowActionHealth(unifiedFlow);
    const validation = validateCurrentFlowDocument(true);
    const report = buildReadOnlyRehearsal(
      unifiedFlow,
      validation,
      actionHealth,
      await flowRevision(unifiedFlow),
    );

    if (!report.ready) {
      preparationMode.value = "rehearsal";
      preparationError.value = "";
      preparationLoading.value = false;
      recipeImportPreview.value = null;
      rehearsalReport.value = report;
      showPreparationModal.value = true;
      console.error("Flow preparation failed:", report.blockers);
      return;
    }

    if (report.warnings.length > 0) {
      console.warn("Flow preparation warnings:", report.warnings);
    }

    console.log("🚀 Executing flow:", unifiedFlow.id);

    // Execute flow using unified format (serde handles conversion automatically)
    currentFlow.value = unifiedFlow;
    await flowManager.executeFlow(inputs);
    showRunInputs.value = false;

    console.log("✅ Flow execution started successfully");
  } catch (error) {
    runInputError.value = String(error);
    console.error("❌ Flow execution failed:", error);
  } finally {
    flowStartPending.value = false;
  }
};

const runPreparedFlow = async () => {
  closePreparationModal();
  await executeFlowWithUnified();
};

// ============================================================================
// LEGACY COMPATIBILITY - REMOVE WHEN MIGRATION IS COMPLETE
// ============================================================================

// ============================================================================
// ENHANCED NODE STATUS FUNCTIONS
// ============================================================================

// Get node execution status from the active flow execution
const getNodeStatus = (node: Node): string => {
  if (!activeFlowExecution.value) return "idle";

  const nodeResult = activeFlowExecution.value.nodeResults[persistedEditorNodeId(node)];
  if (nodeResult) {
    switch (nodeResult.status) {
      case "Pending":
        return "pending";
      case "Running":
        return "running";
      case "Completed":
        return "completed";
      case "Failed":
        return "failed";
      case "Skipped":
        return "skipped";
      default:
        return "idle";
    }
  }

  // Check if this is the current node being executed
  if (activeFlowExecution.value.currentNode === persistedEditorNodeId(node)) {
    return "running";
  }

  // Check if node is in completed list
  if (activeFlowExecution.value.completedNodes.includes(persistedEditorNodeId(node))) {
    return "completed";
  }

  // Check if node is in failed list
  if (activeFlowExecution.value.failedNodes.includes(persistedEditorNodeId(node))) {
    return "failed";
  }

  return "idle";
};

// Get CSS classes for node status indicator with Orchestrator awareness
const getNodeStatusClasses = (node: Node): string => {
  const status = getNodeStatus(node);
  const baseClasses = "w-2 h-2 rounded-full";
  const isOrchestratorManaged = isNodeConnectedToOrchestrator(node.id);

  // Add orchestrator-managed class for styling
  let extraClasses = "";
  if (isOrchestratorManaged) {
    extraClasses += " orchestrator-managed";

    // Check if this node was executed by AI (skipped independent execution)
    if (activeFlowExecution.value) {
      const nodeResult = activeFlowExecution.value.nodeResults[persistedEditorNodeId(node)];
      if (nodeResult?.outputs?.skipped === true) {
        extraClasses += " status-orchestrator-executed";
      }
    }
  }

  switch (status) {
    case "pending":
      return `${baseClasses} bg-yellow-400 animate-pulse${extraClasses}`;
    case "running":
      return `${baseClasses} bg-orange-400 animate-ping${extraClasses}`;
    case "completed":
      return `${baseClasses} bg-green-400${extraClasses}`;
    case "failed":
      return `${baseClasses} bg-red-400${extraClasses}`;
    case "skipped":
      return `${baseClasses} bg-gray-400${extraClasses}`;
    default:
      return `${baseClasses} bg-gray-500${extraClasses}`;
  }
};

// Get status title for tooltip with Orchestrator information
const getNodeStatusTitle = (node: Node): string => {
  const status = getNodeStatus(node);
  const isOrchestratorManaged = isNodeConnectedToOrchestrator(node.id);

  if (isOrchestratorManaged) {
    const orchestratorNode = getSourceOrchestratorNode(node.id);
    const orchestratorName = orchestratorNode?.text || "AI Orchestrator";

    // Check if this node was executed by AI
    if (activeFlowExecution.value) {
      const nodeResult = activeFlowExecution.value.nodeResults[persistedEditorNodeId(node)];
      if (nodeResult?.outputs?.skipped === true) {
        return `Executed by ${orchestratorName} (not run independently)`;
      }
    }

    // Add orchestrator info to status
    switch (status) {
      case "pending":
        return `Managed by ${orchestratorName} - Waiting to execute`;
      case "running":
        return `Managed by ${orchestratorName} - Currently executing`;
      case "completed":
        return `Managed by ${orchestratorName} - Execution completed successfully`;
      case "failed":
        return `Managed by ${orchestratorName} - Execution failed`;
      case "skipped":
        return `Managed by ${orchestratorName} - Execution skipped`;
      default:
        return `Managed by ${orchestratorName} - Ready to execute`;
    }
  }

  // Standard status messages for non-orchestrator nodes
  switch (status) {
    case "pending":
      return "Waiting to execute";
    case "running":
      return "Currently executing";
    case "completed":
      return "Execution completed successfully";
    case "failed":
      return "Execution failed";
    case "skipped":
      return "Execution skipped";
    default:
      return "Ready to execute";
  }
};

// Get node execution time
const getNodeExecutionTime = (node: Node): number | null => {
  if (!activeFlowExecution.value) return null;

  const nodeResult = activeFlowExecution.value.nodeResults[persistedEditorNodeId(node)];
  return nodeResult?.executionTimeMs || null;
};

// Format execution time for display
const formatExecutionTime = (timeMs: number | null): string => {
  if (timeMs === null || timeMs === undefined) {
    return "N/A";
  }

  if (timeMs < 1000) {
    return `${timeMs}ms`;
  } else if (timeMs < 60000) {
    return `${(timeMs / 1000).toFixed(1)}s`;
  } else {
    const minutes = Math.floor(timeMs / 60000);
    const seconds = Math.floor((timeMs % 60000) / 1000);
    return `${minutes}m ${seconds}s`;
  }
};

// Get node retry count
const getNodeRetryCount = (node: Node): number => {
  if (!activeFlowExecution.value) return 0;

  const nodeResult = activeFlowExecution.value.nodeResults[persistedEditorNodeId(node)];
  return nodeResult?.retryCount || 0;
};

// Check if node is running concurrently with other nodes
const isNodeRunningConcurrently = (node: Node): boolean => {
  if (!activeFlowExecution.value) return false;

  // Count how many nodes are currently running
  const runningNodes = Object.values(activeFlowExecution.value.nodeResults).filter(
    (result) => result.status === "Running"
  ).length;

  // If more than one node is running, and this node is one of them, it's concurrent
  return runningNodes > 1 && getNodeStatus(node) === "running";
};

// Get concurrent execution settings
const getConcurrentExecutionInfo = computed(() => {
  if (!activeFlowExecution.value) return null;

  const settings = activeFlowExecution.value.flowDefinition.metadata.executionSettings;
  const runningCount = Object.values(activeFlowExecution.value.nodeResults).filter(
    (result) => result.status === "Running"
  ).length;

  return {
    maxConcurrent: settings.maxConcurrentNodes,
    currentlyRunning: runningCount,
    canRunMore: runningCount < settings.maxConcurrentNodes,
  };
});

watch(activeFlowExecution, (newExecution, oldExecution) => {
  if (newExecution) {
    showFlowStatusPanel.value = true;

    // Update node execution state with actual input/output values from execution
    // Match durable runtime IDs to their canvas projections, including imported flows.
    if (newExecution.nodeResults) {
      applyNodePreviewResults(nodes.value, newExecution.nodeResults);
    }

    // Check if flow just completed
    if (
      oldExecution &&
      (oldExecution.status === "Running" || oldExecution.status === "Pending") &&
      (newExecution.status === "Completed" ||
        newExecution.status === "Failed" ||
        newExecution.status === "Cancelled")
    ) {
      console.log(`🏁 Flow completed with status: ${newExecution.status}`);

      // // Optionally auto-hide panel after 5 seconds for successful completions
      // if (newExecution.status === "Completed") {
      //   setTimeout(() => {
      //     // Only auto-hide if user hasn't manually closed it and flow status hasn't changed
      //     if (
      //       showFlowStatusPanel.value &&
      //       activeFlowExecution.value?.status === "Completed" &&
      //       activeFlowExecution.value?.flowId === newExecution.flowId
      //     ) {
      //       console.log("🔄 Auto-hiding completed flow status panel");
      //       showFlowStatusPanel.value = false;
      //     }
      //   }, 5000);
      // }
    }
  }
});

// Watch for nodes starting/stopping progress (backend throttles actual emissions)
watch(
  () => nodeProgress.value.size,
  () => {
    // Skip expensive recalculations during active drag operations
    if (isDragging.value || isPanning.value) {
      return;
    }
    // Trigger connection update when nodes start/stop showing progress
    nextTick(() => {
      dragUpdateTrigger.value++;
    });
  }
);

// Watch camera scale to ensure connections update smoothly during zoom
watch(
  () => camera.value.scale,
  () => {
    // Skip during active drag/pan operations
    if (isDragging.value || isPanning.value) {
      return;
    }
    // Use requestAnimationFrame for smooth visual updates during zoom
    requestAnimationFrame(() => {
      nextTick(() => {
        dragUpdateTrigger.value++;
      });
    });
  }
);

console.log("Connections:", connections.value);

// Add watchers for unsaved changes detection
watch(
  [nodes, connections],
  () => {
    if (isHydratingFlow.value) return;
    updateUnsavedChanges();
  },
  { deep: true }
);

// Watch for SetVariable node changes to update GetVariable dropdowns
// Use a computed to extract just variable names (avoids infinite loop from deep watch)
const setVariableNames = computed(() => extractVariableNames());

watch(setVariableNames, () => {
  // Only update GetVariable enums when variable names actually change
  updateGetVariableEnums();
});

// ============================================================================
// IMPORT/EXPORT FUNCTIONALITY
// ============================================================================

const fileInput = ref<HTMLInputElement | null>(null);

// Add export callback state
const pendingExportCallback = ref<(() => void) | null>(null);

// Add save callback state
const pendingSaveCallback = ref<(() => void) | null>(null);

const exportFlowToFile = () => {
  try {
    if (nodes.value.length === 0) {
      console.warn("⚠️ No nodes to export");
      mainStore.addNotification({
        title: "Cannot Export Flow",
        message: "No nodes to export! Please add some nodes to the flow first.",
        type: "warning",
        autoClose: true,
        duration: 5000,
        buttonText: "",
        buttonFunction: () => {},
        color: "bg-orange-500",
      });
      return;
    }

    // Validate flow before export (less strict than execution)
    const validation = validateCurrentFlowDocument(false);
    if (!validation.isValid || validation.warnings.length > 0) {
      validationResult.value = validation;
      showValidationErrors.value = true;

      // Store the export callback to continue after modal closes
      pendingExportCallback.value = () => {
        performExport();
      };

      console.log("❌ Flow validation found issues, showing modal");
      return;
    }

    // If validation passes, export directly
    performExport();
  } catch (error) {
    console.error("❌ Export failed:", error);
    mainStore.addNotification({
      title: "Export Failed",
      message: `Export failed: ${error}`,
      type: "error",
      autoClose: true,
      duration: 7000,
      buttonText: "",
      buttonFunction: () => {},
      color: "bg-red-500",
    });
  }
};

// Separate function to perform the actual export
const performExport = () => {
  try {
    // Create unified flow for export
    const unifiedFlow = createCanonicalFlowDocument();

    // Use flowManager's export functionality
    const jsonString = flowManager.exportFlow(unifiedFlow);

    // Create and download file
    const blob = new Blob([jsonString], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `${unifiedFlow.name.replace(/\s+/g, "_")}_${Date.now()}.json`;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);

    console.log("✅ Flow exported successfully");
  } catch (error) {
    console.error("❌ Export failed:", error);
    mainStore.addNotification({
      title: "Export Failed",
      message: `Export failed: ${error}`,
      type: "error",
      autoClose: true,
      duration: 7000,
      buttonText: "",
      buttonFunction: () => {},
      color: "bg-red-500",
    });
  }
};

const downloadJsonDocument = (value: unknown, fileName: string) => {
  const blob = new Blob([JSON.stringify(value, null, 2)], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = fileName;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
};

const exportFlowRecipe = async () => {
  if (nodes.value.length === 0) {
    mainStore.addNotification({
      title: "Cannot Export Recipe",
      message: "Add at least one node before exporting a recipe.",
      type: "warning",
      autoClose: true,
      duration: 5000,
      buttonText: "",
      buttonFunction: () => {},
      color: "bg-orange-500",
    });
    return;
  }

  try {
    const flow = createCanonicalFlowDocument();
    const actionHealth = await refreshFlowActionHealth(flow);
    const report = buildReadOnlyRehearsal(
      flow,
      validateCurrentFlowDocument(true),
      actionHealth,
      await flowRevision(flow),
    );
    if (!report.ready) {
      preparationMode.value = "rehearsal";
      preparationError.value = "";
      preparationLoading.value = false;
      recipeImportPreview.value = null;
      rehearsalReport.value = report;
      showPreparationModal.value = true;
      return;
    }

    const now = new Date().toISOString();
    const recipe = buildFlowRecipe(flow, actionHealth, {
      recipeId: flow.id,
      recipeVersion: flow.version,
      now,
    });
    const safeName = flow.name.trim().replace(/[^a-z0-9]+/gi, "_").replace(/^_+|_+$/g, "")
      || "flow_recipe";
    downloadJsonDocument(recipe, `${safeName}.vdrecipe.json`);
  } catch (error) {
    mainStore.addNotification({
      title: "Recipe Export Failed",
      message: String(error),
      type: "error",
      autoClose: true,
      duration: 7000,
      buttonText: "",
      buttonFunction: () => {},
      color: "bg-red-500",
    });
  }
};

const closePreparationModal = () => {
  showPreparationModal.value = false;
  preparationLoading.value = false;
  preparationError.value = "";
};

const performRecipeImport = async () => {
  if (!recipeImportPreview.value) return;
  const preview = recipeImportPreview.value;
  closePreparationModal();
  await loadImportedFlowToUI(preview.flow);
  loadedFlowDefinition.value = null;
  currentFlowMetadata.value.isNew = true;
  currentFlowMetadata.value.isActive = false;
  lastSavedState.value = "";
  updateUnsavedChanges();
  await refreshFlowActionHealth(createCanonicalFlowDocument());
  recipeImportPreview.value = null;
};

const confirmRecipeImport = async () => {
  if (!recipeImportPreview.value) return;
  if (hasUnsavedChanges.value) {
    closePreparationModal();
    pendingAction.value = "recipe-import";
    showUnsavedChangesModal.value = true;
    return;
  }
  await performRecipeImport();
};

const handleImportFromHeader = async (file: File) => {
  try {
    // Read file content
    const fileContent = await new Promise<string>((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = (e) => resolve(e.target?.result as string);
      reader.onerror = () => reject(new Error("Failed to read file"));
      reader.readAsText(file);
    });

    console.log("📁 File content loaded:", fileContent.substring(0, 200) + "...");

    if (isFlowRecipe(fileContent)) {
      recipeImportPreview.value = parseFlowRecipe(fileContent, {
        id: `flow-${Date.now()}`,
        now: new Date().toISOString(),
      });
      preparationMode.value = "import";
      preparationError.value = "";
      preparationLoading.value = false;
      rehearsalReport.value = null;
      showPreparationModal.value = true;
      return;
    }

    // Use flowManager's raw JSON import functionality with validation
    const importedFlow = flowManager.importFlow(fileContent);

    console.log("✅ Flow imported successfully:", importedFlow.id);

    // Convert imported flow back to UI format and load it
    await loadImportedFlowToUI(importedFlow);

    console.log(
      `Successfully imported flow: "${importedFlow.name}" with ${importedFlow.nodes.length} nodes and ${importedFlow.connections.length} connections`
    );
  } catch (error) {
    console.error("❌ Import failed:", error);
    mainStore.addNotification({
      title: "Import Failed",
      message: `Import failed: ${error}`,
      type: "error",
      autoClose: true,
      duration: 7000,
      buttonText: "",
      buttonFunction: () => {},
      color: "bg-red-500",
    });
  }
};

/**
 * Enhance node inputs with schema metadata from backend
 * Fixes widget rendering issue when loading saved flows
 *
 * @param nodeType - The type/id of the node
 * @param savedInputs - Input values from saved flow (Record<string, any>)
 * @param schemaMap - Optional pre-built Map for O(1) lookups instead of O(n) searches
 * @returns Enhanced inputs with widget metadata
 */
const enhanceNodeInputsWithSchema = (
  nodeType: string,
  savedInputs: Record<string, any>,
  schemaMap?: Map<string, any>
): NodeIO[] => {
  // Find the backend node schema for this node type
  // Use Map lookup (O(1)) if provided, otherwise fall back to array search (O(n))
  const backendNode = schemaMap
    ? schemaMap.get(nodeType)
    : backendFlowNodes.value.find((node) => node.id === nodeType);

  if (!backendNode || !backendNode.parameters) {
    // Fallback: create basic inputs if schema not found
    console.warn(`⚠️ No schema found for node type: ${nodeType}. Using basic inputs.`);
    const fallbackInputs = Object.entries(savedInputs).map(([id, value]) => ({
      id,
      name: id,
      type: "string",
      value: value,
      attached: false,
    })) as NodeIO[];
    return ensureVariableNodeInputs(nodeType, fallbackInputs);
  }

  // Use the existing convertSchemaToInputs to get full input definitions with widget metadata
  const schemaInputs = ensureVariableNodeInputs(
    nodeType,
    convertSchemaToInputs(backendNode.parameters)
  );

  // Merge saved values with schema definitions
  return schemaInputs.map((schemaInput) => ({
    ...schemaInput,
    // Preserve the saved value, or use schema default if no saved value exists
    value: savedInputs.hasOwnProperty(schemaInput.id)
      ? savedInputs[schemaInput.id]
      : schemaInput.value,
    // Inputs are not attached when loading from DB (connections will be restored separately)
    attached: false,
  }));
};

const loadImportedFlowToUI = async (flow: FlowDefinition) => {
  isHydratingFlow.value = true;

  try {
    assertSupportedFlowDocument(flow);
    // Decode legacy imports once at the boundary. Everything below this point
    // works with semantic control/data edges only.
    flow.connections = normalizeFlowConnections(flow.connections as unknown[]);
    loadedFlowDefinition.value = flow;

    // Clear current flow
    nodes.value = [];
    connections.value = [];
    clearSelection();

    // Update current flow metadata
    currentFlowMetadata.value = {
      id: flow.id,
      name: flow.name,
      description: flow.description || null,
      isNew: false,
      isActive: flow.isActive ?? false,
    };

    console.log("🔄 Converting imported flow to UI format...");

    // 🚀 Performance: Build schema lookup map once for O(1) lookups instead of O(n) searches per node
    const schemaMap = new Map(backendFlowNodes.value.map((node) => [node.id, node]));
    const idProjection = createEditorNodeIdProjection(flow.nodes);

    // Convert FlowNodeDefinition back to UI Node format
    const importedNodes: Node[] = flow.nodes.map((flowNode) => {
      const nodeId = idProjection.canvasIdFor(flowNode.id);
      const customProperties = flowNode.uiData?.customProperties || {};
      const backendNode = schemaMap.get(flowNode.nodeType);
      const backendHasNamedFlowRoutes = Array.isArray(backendNode?.outputs)
        ? backendNode.outputs.some(
            (output: any) =>
              output.output_type === "flow" && output.id !== DEFAULT_FLOW_OUTPUT_ID
          )
        : false;
      const normalizedOutputs = flowNode.outputs.filter(
        (output) =>
          !(
            backendHasNamedFlowRoutes &&
            output.outputType === "flow" &&
            output.id === DEFAULT_FLOW_OUTPUT_ID
          )
      );
      const actionReference =
        customProperties.actionReference || flowNode.inputs?.actionReference || null;
      const isInvocationMode = Boolean(customProperties.invocationMode || actionReference);
      const isMinimized = Boolean(customProperties.minimized);
      const executionConfig: NodeExecutionConfig = {
        timeoutMs: flowNode.executionConfig?.timeoutMs,
        retryCount: flowNode.executionConfig?.retryCount ?? 0,
        retryDelayMs: flowNode.executionConfig?.retryDelayMs ?? 1000,
        skipOnError: flowNode.executionConfig?.skipOnError ?? false,
        parallelExecution: flowNode.executionConfig?.parallelExecution ?? false,
      };

      return {
        id: nodeId,
        type: flowNode.nodeType,
        text: flowNode.name,
        x: flowNode.position[0],
        y: flowNode.position[1],
        // 🔧 FIX: Enhance inputs with schema metadata for proper widget rendering
        inputs: enhanceNodeInputsWithSchema(
          flowNode.nodeType,
          flowNode.inputs || {},
          schemaMap
        ),
        outputs: withExecutionPolicyOutputs(
          normalizedOutputs.map((output) => ({
            id: output.id,
            name: output.name,
            type: output.outputType,
          })),
          executionConfig,
        ),
        isTool: flowNode.toolName ? true : false,
        toolName: flowNode.toolName,
        isAiNode: flowNode.aiBehavior ? true : false,
        aiBehavior: flowNode.aiBehavior,
        aiModel: flowNode.aiModel,
        promptTemplate: flowNode.promptTemplate,
        parameters: actionReference ? { actionReference } : undefined,
        invocationMode: isInvocationMode,
        minimized: isMinimized,
        executionConfig,
        persistedDefinition: flowNode,
        executionState: {
          status: "idle",
          last_execution: undefined,
          last_output: undefined,
          error_message: undefined,
        },
      };
    });

    // Convert FlowConnection back to UI Connection format
    const importedConnections: Connection[] = flow.connections.map((flowConn, index) =>
      flowConnectionToCanvasConnection(flowConn, index + 1, idProjection.canvasIdFor)
    );

    // Load into UI
    nodes.value = importedNodes;
    connections.value = importedConnections;

    // Restore camera position
    if (flow.metadata.canvasSettings) {
      camera.value = {
        x: flow.metadata.canvasSettings.panX,
        y: flow.metadata.canvasSettings.panY,
        scale: flow.metadata.canvasSettings.zoom,
      };
    }

    // Let the array replacements and derived variable metadata settle before
    // capturing the canonical saved snapshot.
    await nextTick();
    updateGetVariableEnums();
    await nextTick();
    forceConnectionUpdate();

    console.log("🎨 Flow loaded into UI successfully");
    console.log("📊 Loaded nodes:", nodes.value.length);
    console.log("🔗 Loaded connections:", connections.value.length);

    // Capture the fully hydrated form, not the intermediate database form.
    updateSavedState();
  } catch (error) {
    console.error("❌ Failed to load imported flow into UI:", error);
    throw new Error(`Failed to load flow into UI: ${error}`);
  } finally {
    isHydratingFlow.value = false;
  }
};

// ============================================================================
// SAVE/LOAD FLOW LOGIC
// ============================================================================

// Save flow to database (smart save - overwrites existing or creates new)
const saveFlow = async () => {
  // Reset Save As state to ensure normal save keeps the same ID
  pendingSaveAsOriginalState.value = false;

  try {
    if (nodes.value.length === 0) {
      console.warn("⚠️ No nodes to save");
      mainStore.addNotification(
        {
          title: "Cannot Save Flow",
          message: "No nodes to save! Please add some nodes to the flow first.",
          type: "warning",
          autoClose: true,
          duration: 5000,
          buttonText: "",
          buttonFunction: () => {},
          color: "bg-orange-500",
        },
        currentWindowId
      );
      return;
    }

    // Validate flow before saving
    const validation = validateCurrentFlowDocument(false);
    if (!validation.isValid || validation.warnings.length > 0) {
      validationResult.value = validation;
      showValidationErrors.value = true;

      // Store the save callback to continue after modal closes
      pendingSaveCallback.value = () => {
        showSaveFlowModal.value = true;
      };

      console.log("❌ Flow validation found issues, showing modal");
      return;
    }

    // If validation passes, show save modal
    showSaveFlowModal.value = true;
  } catch (error) {
    console.error("❌ Save flow failed:", error);
    mainStore.addNotification({
      title: "Save Failed",
      message: `Save failed: ${error}`,
      type: "error",
      autoClose: true,
      duration: 7000,
      buttonText: "",
      buttonFunction: () => {},
      color: "bg-red-500",
    });
  }
};

// Save As - always create a new flow with new name
const saveFlowAs = async () => {
  try {
    if (nodes.value.length === 0) {
      console.warn("⚠️ No nodes to save");
      mainStore.addNotification(
        {
          title: "Cannot Save As",
          message: "No nodes to save! Please add some nodes to the flow first.",
          type: "warning",
          autoClose: true,
          duration: 5000,
          buttonText: "",
          buttonFunction: () => {},
          color: "bg-orange-500",
        },
        currentWindowId
      );
      return;
    }

    // Validate flow before saving
    const validation = validateCurrentFlowDocument(false);
    if (!validation.isValid || validation.warnings.length > 0) {
      validationResult.value = validation;
      showValidationErrors.value = true;

      // Store the save as callback to continue after modal closes
      pendingSaveCallback.value = () => {
        // For Save As, we want to show the modal as a new flow
        const originalIsNew = currentFlowMetadata.value.isNew;
        currentFlowMetadata.value.isNew = true;
        showSaveFlowModal.value = true;
        // We'll restore the original state after saving
        pendingSaveAsOriginalState.value = originalIsNew;
      };

      console.log("❌ Flow validation found issues, showing modal");
      return;
    }

    // If validation passes, show save modal as new flow
    const originalIsNew = currentFlowMetadata.value.isNew;
    currentFlowMetadata.value.isNew = true;
    showSaveFlowModal.value = true;
    // We'll restore the original state after saving
    pendingSaveAsOriginalState.value = originalIsNew;
  } catch (error) {
    console.error("❌ Save As failed:", error);
    mainStore.addNotification({
      title: "Save As Failed",
      message: `Save As failed: ${error}`,
      type: "error",
      autoClose: true,
      duration: 7000,
      buttonText: "",
      buttonFunction: () => {},
      color: "bg-red-500",
    });
  }
};

// Load flow from database
const loadFlowFromDatabase = async (flow: any) => {
  try {
    // Don't load temporary flows - they're already in the editor
    if (flow.isTemporary) {
      console.log("📝 Temporary flow is already loaded in editor");
      return;
    }

    // Check for unsaved changes before loading
    if (hasUnsavedChanges.value) {
      // Show confirmation modal
      pendingFlowToLoad.value = flow;
      pendingAction.value = "load";
      showUnsavedChangesModal.value = true;
      return;
    }

    // Load directly if no unsaved changes
    await performFlowLoad(flow);
  } catch (error) {
    console.error("❌ Load flow failed:", error);
    mainStore.addNotification({
      title: "Load Failed",
      message: `Load failed: ${error}`,
      type: "error",
      autoClose: true,
      duration: 7000,
      buttonText: "",
      buttonFunction: () => {},
      color: "bg-red-500",
    });
  }
};

// Actually perform the flow load (extracted for reuse)
const performFlowLoad = async (flow: any) => {
  try {
    console.log("📁 Loading flow from database:", flow.name);

    // Load flow definition from database
    const flowDefinition = await invoke<FlowDefinition>("load_flow", {
      flowId: flow.id,
    });
    console.log("✅ Flow loaded from database:", flowDefinition.id);

    // Remove any temporary flows since we're loading a saved one
    temporaryFlows.value = [];

    // Load into UI
    await loadImportedFlowToUI(flowDefinition);

    console.log(`Successfully loaded flow: "${flowDefinition.name}"`);
  } catch (error) {
    console.error("❌ Load flow failed:", error);
    mainStore.addNotification({
      title: "Load Failed",
      message: `Load failed: ${error}`,
      type: "error",
      autoClose: true,
      duration: 7000,
      buttonText: "",
      buttonFunction: () => {},
      color: "bg-red-500",
    });
  }
};

// Delete flow from database - show confirmation modal
const deleteFlowFromDatabase = (flow: any) => {
  pendingFlowToDelete.value = flow;
  pendingAction.value = "delete";
  showUnsavedChangesModal.value = true;
};

// Actually perform the delete
const performDeleteFlow = async () => {
  const flow = pendingFlowToDelete.value;
  if (!flow) return;

  try {
    await invoke("delete_flow", {
      flowId: flow.id,
    });

    console.log("✅ Flow deleted successfully");
    mainStore.addNotification({
      title: "Flow Deleted Successfully",
      message: `Flow "${flow.name}" deleted successfully!`,
      type: "success",
      autoClose: true,
      duration: 4000,
      buttonText: "",
      buttonFunction: () => {},
      color: "bg-green-500",
    });

    // Refresh flows list
    await loadFlows();

    // Emit event to main window to update deck flows menu
    await emitTo("main", "flows_updated", { action: "deleted", flow: flow });
  } catch (error) {
    console.error("❌ Delete flow failed:", error);
    mainStore.addNotification({
      title: "Delete Failed",
      message: `Delete failed: ${error}`,
      type: "error",
      autoClose: true,
      duration: 7000,
      buttonText: "",
      buttonFunction: () => {},
      color: "bg-red-500",
    });
  }
};

// Load flows from database
const loadFlows = async () => {
  try {
    const flowsList = await invoke<any[]>("list_flows");
    flows.value = flowsList || [];
    console.log("✅ Flows loaded from database:", flows.value.length);
  } catch (error) {
    console.error("❌ Load flows failed:", error);
    flows.value = [];
  }
};

// Load flow from deck page request
const loadFlowFromDeckPage = async (flowToLoad: any) => {
  try {
    console.log("📁 Loading flow from deck page:", flowToLoad.name);

    // Load flow definition from database
    const flowDefinition = await invoke<FlowDefinition>("load_flow", {
      flowId: flowToLoad.id,
    });
    console.log("✅ Flow loaded from database:", flowDefinition.id);

    // Load into UI
    await loadImportedFlowToUI(flowDefinition);

    console.log(`Successfully loaded flow: "${flowDefinition.name}"`);

    // updateSavedState() is now called inside loadImportedFlowToUI
  } catch (error) {
    console.error("❌ Load flow from deck page failed:", error);
    mainStore.addNotification({
      title: "Load Failed",
      message: `Load failed: ${error}`,
      type: "error",
      autoClose: true,
      duration: 7000,
      buttonText: "",
      buttonFunction: () => {},
      color: "bg-red-500",
    });
  }
};

// Confirmation modal handlers
const confirmLoadFlow = async () => {
  showUnsavedChangesModal.value = false;

  // Handle new flow action
  if (pendingAction.value === "new") {
    pendingAction.value = null;
    performNewFlow();
    return;
  }

  // Handle delete flow action
  if (pendingAction.value === "delete") {
    pendingAction.value = null;
    await performDeleteFlow();
    pendingFlowToDelete.value = null;
    return;
  }

  if (pendingAction.value === "recipe-import") {
    pendingAction.value = null;
    await performRecipeImport();
    return;
  }

  // Handle load flow action
  if (pendingFlowToLoad.value) {
    // Check if this is a deck page flow load or a regular flow load
    if (pendingFlowToLoad.value.source === "deck") {
      await loadFlowFromDeckPage(pendingFlowToLoad.value);
    } else {
      await performFlowLoad(pendingFlowToLoad.value);
      // updateSavedState() is now called inside loadImportedFlowToUI
    }
    pendingFlowToLoad.value = null;
  }
  pendingAction.value = null;
};

const cancelLoadFlow = () => {
  if (pendingAction.value === "recipe-import") {
    recipeImportPreview.value = null;
  }
  showUnsavedChangesModal.value = false;
  pendingFlowToLoad.value = null;
  pendingFlowToDelete.value = null;
  pendingAction.value = null;
};

// Edit Flow Metadata functionality
const editFlowInfo = () => {
  showEditFlowMetadataModal.value = true;
};

const saveFlowMetadata = async (newName: string, newDescription: string) => {
  try {
    showEditFlowMetadataModal.value = false;

    // Update local metadata
    currentFlowMetadata.value.name = newName;
    currentFlowMetadata.value.description = newDescription;

    // If this is not a new flow, save the metadata to database
    if (!currentFlowMetadata.value.isNew && currentFlowMetadata.value.id) {
      // Create updated flow definition
      const unifiedFlow = createCanonicalFlowDocument({
        name: newName,
        description: newDescription,
      });
      const expectedRevision = loadedFlowDefinition.value
        ? await flowRevision(loadedFlowDefinition.value)
        : null;

      // Save to database
      await invoke("save_flow", {
        flow: unifiedFlow,
        expectedRevision,
      });
      await acceptSavedFlow(unifiedFlow.id);

      console.log("✅ Flow metadata updated successfully");

      // Refresh flows list to show updated name
      await loadFlows();

      // Update temporary flows if this flow exists there
      const tempFlowIndex = temporaryFlows.value.findIndex(
        (f) => f.id === currentFlowMetadata.value.id
      );
      if (tempFlowIndex !== -1) {
        temporaryFlows.value[tempFlowIndex].name = newName;
        temporaryFlows.value[tempFlowIndex].description = newDescription;
      }

      // Emit event to main window to update deck flows menu
      await emitTo("main", "flows_updated", { action: "updated", flow: unifiedFlow });
    } else {
      // For new flows, just update the temporary flow entry
      const tempFlowIndex = temporaryFlows.value.findIndex(
        (f) => f.id === currentFlowMetadata.value.id
      );
      if (tempFlowIndex !== -1) {
        temporaryFlows.value[tempFlowIndex].name = newName;
        temporaryFlows.value[tempFlowIndex].description = newDescription;
      }
    }

    console.log(`Flow metadata updated: "${newName}"`);
  } catch (error) {
    console.error("❌ Failed to update flow metadata:", error);
    mainStore.addNotification({
      title: "Update Failed",
      message: `Failed to update flow metadata: ${error}`,
      type: "error",
      autoClose: true,
      duration: 7000,
      buttonText: "",
      buttonFunction: () => {},
      color: "bg-red-500",
    });
  }
};

// Handle cancel save flow modal
const handleCancelSaveFlowModal = () => {
  showSaveFlowModal.value = false;
  pendingSaveAsOriginalState.value = false; // Reset Save As state on cancel
};

// Handle save flow from modal
const handleSaveFlowFromModal = async (name: string, description: string) => {
  try {
    showSaveFlowModal.value = false;

    // Check if this is a Save As operation
    const isSaveAs = pendingSaveAsOriginalState.value !== false;

    // Store the original state before updating metadata
    const wasNewFlow = currentFlowMetadata.value.isNew;
    const oldFlowId = currentFlowMetadata.value.id;

    // Create unified flow for saving
    const unifiedFlow = createCanonicalFlowDocument({
      newIdentity: isSaveAs,
      name,
      description,
    });
    const expectedRevision = !isSaveAs && loadedFlowDefinition.value
      ? await flowRevision(loadedFlowDefinition.value)
      : null;

    // Save flow to database
    await invoke("save_flow", {
      flow: unifiedFlow,
      expectedRevision,
    });
    await acceptSavedFlow(unifiedFlow.id);

    // Update current flow metadata
    currentFlowMetadata.value = {
      id: unifiedFlow.id,
      name: name,
      description: description,
      isNew: false,
      isActive: unifiedFlow.isActive ?? false,
    };

    console.log("✅ Flow saved successfully");
    mainStore.addNotification(
      {
        title: "Flow Saved Successfully",
        message: `Flow "${name}" saved successfully!`,
        type: "success",
        autoClose: true,
        duration: 4000,
        buttonText: "",
        buttonFunction: () => {},
        color: "bg-green-500",
      },
      currentWindowId
    );

    // Remove temporary flow if it was temporary (use original state)
    if (wasNewFlow || isSaveAs) {
      temporaryFlows.value = temporaryFlows.value.filter((f) => f.id !== oldFlowId);
    }

    // Refresh flows list
    await loadFlows();

    // Update saved state after successful save
    updateSavedState();

    // Emit event to main window to update deck flows menu
    const actionType = wasNewFlow || isSaveAs ? "created" : "updated";
    await emitTo("main", "flows_updated", { action: actionType, flow: unifiedFlow });

    // Restore original state if this was a Save As operation
    if (isSaveAs) {
      pendingSaveAsOriginalState.value = false;
    }
  } catch (error) {
    console.error("❌ Save flow failed:", error);
    mainStore.addNotification(
      {
        title: "Save Failed",
        message: `Save failed: ${error}`,
        type: "error",
        autoClose: true,
        duration: 7000,
        buttonText: "",
        buttonFunction: () => {},
        color: "bg-red-500",
      },
      currentWindowId
    );
  }
};

// Flows are loaded in the main onMounted block above

// Execution History Modal
const showExecutionHistory = ref(false);

// Add drag and drop for nodes (from node library)
const onDragStart = (event: DragEvent, nodeTypeName: string) => {
  if (event.dataTransfer) {
    // Store the node type name (not JSON) - this will be handled by the nodeType path in onDrop
    event.dataTransfer.setData("nodeType", nodeTypeName);
    event.dataTransfer.effectAllowed = "copy";

    // Also set a text version for compatibility
    event.dataTransfer.setData("text/plain", nodeTypeName);

    console.log(`🎯 Dragging node type: ${nodeTypeName}`);
  }
};

const handleInspectorInputChanged = (payload: { node: Node; input: NodeIO }) => {
  const { node, input } = payload;

  if (
    (node.type === "set-variable" || node.type === "get-variable") &&
    (input.id === "variableName" || input.id === "valueType" || input.id === "value")
  ) {
    canvasRef.value?.onVariableNameChange(node.id);
    if (input.id === "variableName") {
      updateGetVariableEnums();
    }
  }
};

const handleInspectorActionReferenceChanged = (payload: { node: Node; stableId: string }) => {
  const reference = { kind: "saved_action", id: payload.stableId };
  payload.node.parameters = {
    ...payload.node.parameters,
    actionReference: reference,
  };
  const serializedInput = payload.node.inputs?.find((input) => input.id === "actionReference");
  if (serializedInput) serializedInput.value = reference;
  payload.node.actionPreflight = undefined;
  updateUnsavedChanges();
};

const handleInspectorActionPreflightChanged = (payload: {
  node: Node;
  preflight: import("@base/types/actionReference").ActionPreflight;
}) => {
  payload.node.actionPreflight = payload.preflight;
};

const ERROR_ROUTE_ID = "error";
const ERROR_MESSAGE_OUTPUT_ID = "errorMessage";
const ERROR_DETAILS_OUTPUT_ID = "errorDetails";
const EXECUTION_POLICY_OUTPUT_IDS = new Set([
  ERROR_ROUTE_ID,
  ERROR_MESSAGE_OUTPUT_ID,
  ERROR_DETAILS_OUTPUT_ID,
]);

const withExecutionPolicyOutputs = (
  outputs: NodeIO[],
  executionConfig: NodeExecutionConfig,
): NodeIO[] => {
  const next = outputs.filter((output) => !EXECUTION_POLICY_OUTPUT_IDS.has(output.id));
  if (!executionConfig.skipOnError) return next;
  return [
    ...next,
    { id: ERROR_ROUTE_ID, name: "On Error", type: "flow" },
    { id: ERROR_MESSAGE_OUTPUT_ID, name: "Error Message", type: "string" },
    { id: ERROR_DETAILS_OUTPUT_ID, name: "Error Details", type: "object" },
  ];
};

const handleInspectorExecutionConfigChanged = (payload: {
  node: Node;
  executionConfig: NodeExecutionConfig;
}) => {
  const { node, executionConfig } = payload;
  const disablingErrorRoute = node.executionConfig.skipOnError && !executionConfig.skipOnError;
  const hasConnectedErrorRoute = connections.value.some(
    (connection) =>
      connection.kind === "control"
      && connection.from === node.id
      && connection.route === ERROR_ROUTE_ID,
  );

  if (disablingErrorRoute && hasConnectedErrorRoute) {
    mainStore.addNotification(
      {
        title: "Error route is connected",
        message: "Disconnect the On Error branch before changing failure handling to Stop flow.",
        type: "warning",
        autoClose: true,
        duration: 5000,
        buttonText: "",
        buttonFunction: () => {},
        color: "bg-orange-500",
      },
      currentWindowId,
    );
    return;
  }

  node.executionConfig = { ...executionConfig };
  node.outputs = withExecutionPolicyOutputs(node.outputs || [], executionConfig);

  forceConnectionUpdate();
  updateUnsavedChanges();
};

const handleInspectorAiConfig = (node: Node) => {
  if (!node) return;
  openAiModelSelector(node);
};

// Helper functions for AI model display
const getAiModelName = (modelId: string) => {
  return utilGetAiModelName(modelId, availableAiModels.value);
};

const getAiModelShortName = (modelId: string) => {
  return utilGetAiModelShortName(modelId, availableAiModels.value);
};

// ============================================================================
// ORCHESTRATOR CONNECTION DETECTION
// ============================================================================

// Check if a node is connected to an Orchestrator AI node
const isNodeConnectedToOrchestrator = (nodeId: number): boolean => {
  return connections.value.some((conn) => {
    if (conn.kind === "control" && conn.to === nodeId) {
      const sourceNode = findNodeById(conn.from);
      return sourceNode?.isAiNode === true && sourceNode?.aiBehavior === "Orchestrator";
    }
    return false;
  });
};

// Check if a connection is from an Orchestrator AI node (ONLY for flow connections)
const isOrchestratorConnection = (conn: Connection): boolean => {
  // Only apply orchestrator styling to flow connections, not data connections
  if (conn.kind !== "control") {
    return false;
  }

  const sourceNode = findNodeById(conn.from);
  const isOrchestrator =
    sourceNode?.isAiNode === true && sourceNode?.aiBehavior === "Orchestrator";

  // Debug logging for troubleshooting
  if (sourceNode?.isAiNode) {
    console.log(`🔍 Checking FLOW connection from AI node ${sourceNode.text}:`, {
      isAiNode: sourceNode.isAiNode,
      aiBehavior: sourceNode.aiBehavior,
      fromType: conn.fromType,
      isOrchestrator: isOrchestrator,
      nodeType: sourceNode.type,
    });
  }

  return isOrchestrator;
};

// Get the source Orchestrator node for a target node (if any)
const getSourceOrchestratorNode = (nodeId: number): Node | null => {
  const conn = connections.value.find((c) => c.kind === "control" && c.to === nodeId);
  if (conn) {
    const sourceNode = findNodeById(conn.from);
    if (sourceNode?.isAiNode && sourceNode?.aiBehavior === "Orchestrator") {
      return sourceNode;
    }
  }
  return null;
};

// Initialize useFlowConnections first (it has fewer dependencies)
const {
  findConnectionsByNode,
  findConnectionsByPort,
  validateNodeConnection,
  addConnectionWithValidation,
  deleteConnection: baseDeleteConnection,
  getNodePosition,
  createSmoothBezierPath,
  getConnectionColor: baseGetConnectionColor,
  getConnectionStrokeWidth: baseGetConnectionStrokeWidth,
  getAnimationDuration,
  getConnectionMidpoint,
} = useFlowConnections(
  {
    nodes,
    connections,
    camera,
  },
  {
    forceConnectionUpdate: () => {
      nextTick(() => {
        dragUpdateTrigger.value++;
      });
    },
    findNodeById: (nodeId: number) => nodes.value.find((n) => n.id === nodeId),
    findNodeInputById: (node: any, inputId: string) =>
      node.inputs?.find((input: any) => input.id === inputId),
    findNodeOutputById: (node: any, outputId: string) =>
      node.outputs?.find((output: any) => output.id === outputId),
    isTriggerNode: (nodeType: string) =>
      nodeTypes.value.some(
        (type) => type.name === nodeType && type.category === "TRIGGER"
      ),
  }
);

// Wrapper for deleteConnection to trigger variable node type sync
const deleteConnection = (connectionId: number) => {
  // Find the connection before deleting to get source and target info
  const connection = connections.value.find((c) => c.id === connectionId);

  // Call base delete function
  baseDeleteConnection(connectionId);

  // Trigger variable node type sync if connection existed
  if (connection) {
    canvasRef.value?.onConnectionChange(connection.from, connection.to);
  }
};

// ============================================================================
// ENHANCED CONNECTION STYLING FOR ORCHESTRATOR
// ============================================================================

// Enhanced connection color with Orchestrator detection
const getConnectionColor = (conn: Connection): string => {
  if (isOrchestratorConnection(conn)) {
    return "url(#orchestratorGradient)"; // Special gradient for Orchestrator connections
  }
  return baseGetConnectionColor(conn);
};

// Enhanced connection stroke width with Orchestrator detection
const getConnectionStrokeWidth = (conn: Connection): number => {
  if (isOrchestratorConnection(conn)) {
    return baseGetConnectionStrokeWidth(conn) + 0.5; // Just slightly thicker than normal, fixed
  }
  return baseGetConnectionStrokeWidth(conn);
};

// Initialize flow validation
const {
  validateFlowForExecution,
  validateFlowForSaving,
  findTriggerNodes,
  findStartingNodes,
  findDisconnectedNodes,
  getFlowSummary,
} = useFlowValidation({
  nodes,
  connections,
  nodeTypes,
});

// Initialize useFlowNodes with connection functions as dependencies
const {
  createNodeAtPosition,
  addNodeToCanvas,
  deleteNode,
  selectNode,
  clearSelection,
  deleteSelection,
  clearAll,
  addNodeOfType,
  isTriggerNode,
  canAddTriggerNode,
  findNodeById,
  findNodeInputById,
  findNodeOutputById,
  findNodeTypeByName,
  isAiNode,
  centerNodeOnPosition,
  getDefaultValueForProperty,
} = useFlowNodes(
  {
    nodes,
    connections,
    nodeTypes,
    selectedNodes,
    selectedNode,
    hoveredFlowPort,
    isCtrlPressed,
  },
  {
    forceConnectionUpdate: () => {
      // Wait for DOM to update then trigger connection recalculation
      nextTick(() => {
        dragUpdateTrigger.value++;
      });
    },
    findConnectionsByNode,
    findNodeInputById: (node: any, inputId: string) =>
      node.inputs?.find((input: any) => input.id === inputId),
    // Provide validation dependencies
    findTriggerNodes,
    showValidationError: (errorMessage: string) => {
      validationResult.value = {
        isValid: false,
        errors: [errorMessage],
        warnings: [],
      };
      showValidationErrors.value = true;
    },
  }
);

// Add validation state
const showValidationErrors = ref(false);
const validationResult = ref<{
  isValid: boolean;
  errors: string[];
  warnings: string[];
} | null>(null);

const closeValidationModal = () => {
  showValidationErrors.value = false;
};

const continueWithExport = () => {
  showValidationErrors.value = false;
  pendingExportCallback.value?.();
};

const continueWithSave = () => {
  showValidationErrors.value = false;
  pendingSaveCallback.value?.();
};
</script>

<style lang="scss">
@use "./flow.scss";
</style>
