<template lang="pug">
div(
  id="flowchart-canvas"
  class="flow-canvas-shell flex-1 relative border-0"
  :style="gridStyle"
  @dragover.prevent
  @drop="onDrop"
  @mousemove="handleMouseMove"
  @mouseup="handleMouseUp"
  @mousedown.stop="handleCanvasMouseDown"
  @contextmenu.prevent="handleCanvasContextMenu"
  @wheel.prevent="handleZoom"
  @keydown.escape="cancelConnection"
  tabindex="0"
)
  // Background element to capture clicks
  div(
    class="absolute top-0 left-0 w-full h-full z-0 " 
    @click="unselectNodes"
  )

  // Selection box - positioned relative to canvas, not transform container
  div(
    v-if="isSelecting"
    class="absolute border-2 border-blue-500 bg-blue-500 bg-opacity-20 pointer-events-none z-15"
    :style="selectionBoxStyle"
  )

  // Temporary connection line (fixed positioned, no more transform container)
  svg(
    v-if="isConnecting"
    class="fixed top-0 left-0 w-screen h-screen pointer-events-none z-15"
    :style="svgConnectionStyles"
  )
    path(
      class="connection-temp"
      :d="createTemporaryPath(connectionStart, connectionEnd)"
      fill="none"
      stroke="#4B5563"
      :stroke-width="Math.max(1.2, 2 * zoomRatio)"
      :stroke-dasharray="`${Math.max(3, 5 * zoomRatio)},${Math.max(3, 5 * zoomRatio)}`"
    )

  // Nodes (fixed positioned, manually transformed)
  div(
    v-for="node in nodes"
    :key="node.id"
    class="node flow-node fixed z-20"
    :data-node-id="node.id"
    :data-plugin-id="node.type && node.type.startsWith('plugin.') ? node.type.substring(7) : null"
    :class="{ 'selected-node': selectedNodes.has(node.id), 'primary-selected': selectedNode && selectedNode.id === node.id, 'dragging-node': isDragging && selectedNode && selectedNode.id === node.id, 'resizing-node': isResizing && resizingNodeId === node.id, 'orchestrator-managed': isNodeManagedByOrchestrator(node) }"
    :style="{ left: (node.x * camera.scale + camera.x) + 'px', top: (node.y * camera.scale + camera.y) + 'px', transform: `scale(${camera.scale})`, transformOrigin: '0 0' }"
    @mousedown="startDrag(node, $event)"
    @dblclick.stop="handleToggleMinimized(node)"
  )
    // ── Assignment compact card ──────────────────────────────────────
    // Outer wrapper: overflow-visible so ports can stick out
    div(
      v-if="node.invocationMode"
      v-node-init="node"
      class="flow-node-card flow-node-card--assignment relative overflow-visible"
      :style="{ width: node.width ? node.width + 'px' : undefined, height: node.height ? node.height + 'px' : undefined }"
    )
      // Inner card shell: overflow-hidden so progress bar clips to rounded corners
      div(class="rounded-2xl bg-themeBackground2 shadow-2xl border border-themeBorder overflow-hidden relative")
        // Card body
        div(class="pl-4 pr-1.5 pt-2.5 pb-3 min-w-[150px]")
          // Top row: badge label + status dot + actions menu trigger
          div(class="flex items-center justify-between gap-2 mb-2")
            div(class="flex items-center gap-1")
              SolarPlayBold(class="w-2.5 h-2.5 text-white/25 flex-shrink-0")
              span(class="text-[9px] font-bold uppercase tracking-widest text-white/30") {{ getNodeKindLabel(node) }}
            div(class="flex items-center gap-1.5")
              span(
                v-if="node.actionPreflight"
                class="px-1.5 py-0.5 rounded-full text-[8px] font-extrabold uppercase tracking-wide"
                :class="getActionPreflightClasses(node.actionPreflight.status)"
                :title="getActionPreflightTitle(node.actionPreflight)"
              ) {{ getActionPreflightLabel(node.actionPreflight.status) }}
              div(
                class="flow-node-status-dot"
                :class="getNodeStatusClasses(node)"
                :title="getNodeStatusTitle(node)"
              )
              button(
                class="flex items-center justify-center p-1.5 -mr-1 text-white/30 hover:text-white/60 transition-colors"
                :class="openNodeActionsMenuId === node.id ? 'text-white/60' : ''"
                @mousedown.stop
                @click.stop="toggleNodeActionsMenu(node.id)"
              )
                span(class="flow-node-actions-dots" aria-hidden="true")
                  span(class="flow-node-actions-dot")
                  span(class="flow-node-actions-dot")
                  span(class="flow-node-actions-dot")
          // Big title
          div(class="pr-4 text-white font-semibold text-sm leading-tight mb-2.5 truncate") {{ node.text || node.type }}
          // Output value ports — flush at right edge inside the card
          div(class="flex flex-col gap-2")
            div(
              v-for="output in (node.outputs || []).filter(o => o.type !== 'flow')"
              :key="output.id"
              class="flex items-center justify-end gap-1.5"
              :data-output-id="output.id"
            )
              span(class="text-[10px] text-white/35 leading-none") {{ output.name }}
              div(
                class="value-output-port w-3 h-3 rounded-full border-2 cursor-pointer transition-colors flex-shrink-0"
                :style="{ borderColor: getTypeColor(output.type), backgroundColor: isPortConnected(node.id, 'value-output-' + output.id) ? getTypeColor(output.type) : 'transparent' }"
                @mousedown.stop="startConnection(node, $event, 'value-output-' + output.id)"
              )
        // Progress bar — clipped properly by parent overflow-hidden
        div(class="absolute bottom-0 left-0 right-0 h-[2px] pointer-events-none")
          div(class="absolute inset-0 bg-gradient-to-r from-cyan-300/20 via-blue-300/10 to-purple-300/20")
          div(
            class="relative h-full bg-gradient-to-r from-cyan-400 via-blue-500 to-purple-500 transition-all duration-300"
            :class="(getNodeProgress(node)?.progress || 0) > 0 ? 'opacity-100' : 'opacity-0'"
            :style="{ width: (getNodeProgress(node)?.progress || 0) + '%' }"
          )
      // Flow ports — outside inner shell, positioned relative to outer wrapper
      div(class="absolute left-0 top-1/2 -translate-x-1/2 -translate-y-1/2 z-10")
        button(
          class="flow-input-port block bg-transparent border-0 outline-none p-0"
          @mousedown.stop="startConnection(node, $event, 'flow-input')"
          @mouseenter="setHoveredFlowPort({nodeId: node.id, type: 'flow-input'})"
          @mouseleave="setHoveredFlowPort(null)"
        )
          SolarPlayBold(
            v-if="isPortConnected(node.id, 'flow-input')"
            class="relative w-5 h-5 text-white"
          )
          SolarPlayBroken(
            v-else-if="hoveredFlowPort && hoveredFlowPort.nodeId === node.id && hoveredFlowPort.type === 'flow-input'"
            class="relative w-5 h-5 text-white"
          )
          SolarPlayBroken(v-else class="relative w-5 h-5 text-white/70")
      div(v-if="hasDefaultFlowOutput(node)" class="absolute right-0 top-1/2 translate-x-1/2 -translate-y-1/2 z-10")
        button(
          class="flow-output-port block bg-transparent border-0 outline-none p-0"
          @mousedown.stop="startFlowOutputConnection(node, $event)"
          @mouseenter="setHoveredFlowPort({nodeId: node.id, type: 'flow-output'})"
          @mouseleave="isConnecting ? '' : setHoveredFlowPort(null)"
        )
          SolarPlayBold(
            v-if="isPortConnected(node.id, 'flow-output')"
            class="relative w-5 h-5 text-white"
          )
          SolarPlayBroken(
            v-else-if="hoveredFlowPort && hoveredFlowPort.nodeId === node.id && hoveredFlowPort.type === 'flow-output'"
            class="relative w-5 h-5 text-white"
          )
          SolarPlayBroken(v-else class="relative w-5 h-5 text-white/70")
      // Actions dropdown
      div(
        v-if="openNodeActionsMenuId === node.id"
        class="flow-node-actions-menu absolute right-2 top-9 min-w-[10rem] z-40"
        @mousedown.stop
        @click.stop
      )
        button(
          class="flow-node-actions-item w-full text-left"
          @mousedown.stop
          @click.stop="openNodeInspector(node)"
        ) Configure node
        button(
          class="flow-node-actions-item flow-node-actions-item--delete w-full text-left"
          @mousedown.stop
          @click.stop="handleNodeMenuDelete(node)"
        ) Delete node

    // ── Normal full node card ─────────────────────────────────────────
    div(
      v-else
      v-node-init="node"
      :class="['flow-node-card flow-node-card--compact relative flex flex-col bg-themeBackground2 rounded-2xl shadow-2xl border border-themeBorder overflow-visible max-w-[34rem]', node.minimized ? 'flow-node-card--minimized' : '', isTriggerNode(node.type) ? 'w-auto' : (node.outputs && node.outputs.filter(o => o.type !== 'flow').length > 0 ? 'w-auto min-w-80' : 'w-auto min-w-52')]"
      :style="{ width: node.width ? node.width + 'px' : undefined, height: !node.minimized && node.height ? node.height + 'px' : undefined }"
    )
      // Node header - enhanced with gradients
      div(
        class="flow-node-card-header py-3 px-4 font-bold text-white flex justify-between items-center gap-x-3 relative overflow-hidden"
        :style="{ '--node-accent': node.type && nodeTypeColors[node.type] ? nodeTypeColors[node.type] : '#3B82F6' }"
        @mousedown="startDrag(node, $event)"
        @dblclick.stop.prevent="handleToggleMinimized(node)"
      )
        div(class="flow-node-header-main flex items-center gap-2 relative z-10 min-w-0")
          span(class="flow-node-kind-chip") {{ getNodeKindLabel(node) }}
          span(class="flow-node-title truncate") {{ node.text || node.type }}
          span(v-if="node.minimized" class="flow-node-minimized-badge") {{ getVisibleInputCount(node) }} in · {{ getVisibleOutputCount(node) }} out
          div(
            class="flow-node-status-dot"
            :class="getNodeStatusClasses(node)"
            :title="getNodeStatusTitle(node)"
          )
          div(
            v-if="isNodeManagedByOrchestrator(node)"
            class="flow-node-ai-managed-dot"
            :title="`Managed by ${getOrchestratorNodeName(node)}`"
          )
        div(class="flex items-center gap-2 relative z-20")
          button(
            class="flow-node-actions-trigger text-white/80 hover:text-white transition-all duration-200"
            :class="openNodeActionsMenuId === node.id ? 'is-open' : ''"
            aria-label="Node actions"
            @mousedown.stop
            @click.stop="toggleNodeActionsMenu(node.id)"
          )
            span(class="flow-node-actions-dots" aria-hidden="true")
              span(class="flow-node-actions-dot")
              span(class="flow-node-actions-dot")
              span(class="flow-node-actions-dot")

        // ComfyUI-style progress bar - subtle idle track + active fill
        div(
          class="absolute bottom-0 left-0 right-0 h-[2px] overflow-hidden pointer-events-none"
        )
          div(class="absolute inset-0 bg-gradient-to-r from-cyan-300/20 via-blue-300/10 to-purple-300/20")
          div(
            class="relative h-full bg-gradient-to-r from-cyan-400 via-blue-500 to-purple-500 transition-all duration-300"
            :class="(getNodeProgress(node)?.progress || 0) > 0 ? 'opacity-100' : 'opacity-0'"
            :style="{ width: (getNodeProgress(node)?.progress || 0) + '%' }"
          )

      div(
        v-if="openNodeActionsMenuId === node.id"
        class="flow-node-actions-menu absolute right-3 top-12 min-w-[10rem] z-40"
        @mousedown.stop
        @click.stop
      )
        button(
          class="flow-node-actions-item w-full text-left"
          @mousedown.stop
          @click.stop="openNodeInspector(node)"
        ) Configure node
        button(
          v-if="isAiNode(node)"
          class="flow-node-actions-item flow-node-actions-item--config w-full text-left"
          @mousedown.stop
          @click.stop="handleAiModelClick(node, true)"
        ) Configure AI
        button(
          class="flow-node-actions-item w-full text-left"
          @mousedown.stop
          @click.stop="handleToggleMinimized(node)"
        ) {{ node.minimized ? "Expand node" : "Minimize node" }}
        button(
          class="flow-node-actions-item flow-node-actions-item--delete w-full text-left"
          @mousedown.stop
          @click.stop="handleNodeMenuDelete(node)"
        ) Delete node

        // Enhanced node content
      div(
        v-if="!node.minimized"
        class="flow-node-content flex-1 p-4"
        :class="isPreviewNode(node.type) ? 'flow-node-content--preview' : ''"
      )
        // Controls section with better styling
        div(
          v-if="isTriggerNode(node.type) || shouldShowFlowInputPort(node) || shouldShowFlowOutputPort(node)"
          class="flex items-center justify-between mb-3"
        )
          div(class="flex items-center")
            // Enhanced flow input styling
            button(
              v-if="shouldShowFlowInputPort(node)"
              class="flow-input-port relative"
              @mousedown.stop="startConnection(node, $event, 'flow-input')"
              @mouseenter="setHoveredFlowPort({nodeId: node.id, type: 'flow-input'})"
              @mouseleave="setHoveredFlowPort(null)"
            )
              SolarPlayBold(
                v-if="isPortConnected(node.id, 'flow-input')"
                class="relative w-5 h-5 text-themePrimary"
              )
              SolarPlayBroken(
                v-else-if="hoveredFlowPort && hoveredFlowPort.nodeId === node.id && hoveredFlowPort.type === 'flow-input'"
                class="relative w-5 h-5 text-themePrimary"
              )
              SolarPlayBroken(v-else class="relative w-5 h-5 text-themeText3")

            // Enhanced trigger change button
            button(
              v-else-if="isTriggerNode(node.type)"
              class="text-themeText text-xs px-3 py-1.5 bg-themeBackground hover:bg-themeBackground3 rounded-xl border border-themeBorder transition-all duration-200 flex items-center gap-2 shadow-2xs cursor-pointer"
              @click.stop="showTriggerChangeMenu(node)"
            )
              MaterialSymbolsDataObject(class="w-3.5 h-3.5 text-themePrimary")

              span Change
          div(class="flex items-center justify-end")
            // Enhanced flow output styling
            button(
              v-if="shouldShowFlowOutputPort(node) && hasDefaultFlowOutput(node)"
              class="flow-output-port relative"
              @mousedown.stop="startFlowOutputConnection(node, $event)"
              @mouseenter="setHoveredFlowPort({nodeId: node.id, type: 'flow-output'})"
              @mouseleave="isConnecting ? '' : setHoveredFlowPort(null)"
            )
              SolarPlayBold(
                v-if="isPortConnected(node.id, 'flow-output')"
                class="relative w-5 h-5 text-themePrimary"
              )
              SolarPlayBroken(
                v-else-if="hoveredFlowPort && hoveredFlowPort.nodeId === node.id && hoveredFlowPort.type === 'flow-output'"
                class="relative w-5 h-5 text-themePrimary"
              )
              SolarPlayBroken(v-else class="relative w-5 h-5 text-themeText3")


        // Enhanced inputs and outputs section
        div(class="flex gap-6 w-full overflow-visible")
          // Enhanced inputs section (min-w-0 prevents pushing, flex-1 grows to fill)
          div(class="min-w-0 flex-1 space-y-3" v-if="!isTriggerNode(node.type)")
            div(
              v-for="input in (node.inputs || []).filter(input => isInputVisible(input, node))"
              :key="input.id"
              class="flow-value-row flow-value-row--input flex items-center text-sm gap-x-3 group max-w-full"
              :data-input-id="input.id"
            )
              div(
                :style="{ borderColor: getTypeColor(input.type), backgroundColor: isPortConnected(node.id, 'value-input-' + input.id) ? getTypeColor(input.type) : 'transparent', boxShadow: isPortConnected(node.id, 'value-input-' + input.id) ? `0 0 8px ${getTypeColor(input.type)}40` : 'none' }"
                class="value-input-port flow-value-port flow-value-port--input w-3.5 h-3.5 aspect-square rounded-full cursor-pointer z-5 hover:scale-125 transition-all duration-200 relative group-hover:shadow-lg flex-shrink-0 border-2"
                @mousedown.stop="startConnection(node, $event, 'value-input-' + input.id)"
              )
                div(v-if="isPortConnected(node.id, 'value-input-' + input.id)" class="absolute -inset-1 rounded-full opacity-20" :style="{ backgroundColor: getTypeColor(input.type) }")
                // Optional input indicator: hollow center when attached, inner ring when not attached
                svg(
                  v-if="input.required === false"
                  class="absolute inset-0 w-full h-full pointer-events-none"
                  viewBox="0 0 10 10"
                )
                  circle(
                    v-if="isPortConnected(node.id, 'value-input-' + input.id)"
                    cx="5"
                    cy="5"
                    r="1.8"
                    class="fill-themeBackground"
                  )
                  circle(
                    v-else
                    cx="5"
                    cy="5"
                    r="2"
                    fill="none"
                    :stroke="getTypeColor(input.type)"
                    stroke-width="1.2"
                  )
              div(class="flex flex-col gap-y-1 min-w-0 flex-1 max-w-[17rem]")
                span(class="text-themeText font-medium truncate text-xs" :title="input.name") {{ input.name }}

                div(
                  v-if="!input.forceInput"
                  class="flow-node-input-preview-row"
                  :title="isPortConnected(node.id, 'value-input-' + input.id) ? 'Connected input' : formatInputPreview(input.value)"
                )
                  span(
                    v-if="!isPortConnected(node.id, 'value-input-' + input.id) && getInputColorPreview(input)"
                    class="flow-node-color-swatch"
                    :style="{ backgroundColor: getInputColorPreview(input) || 'transparent' }"
                  )
                  span(
                    class="flow-node-input-preview text-xs truncate"
                    :class="isPortConnected(node.id, 'value-input-' + input.id) ? 'flow-node-input-preview--attached' : ''"
                  ) {{ isPortConnected(node.id, 'value-input-' + input.id) ? "Connected" : formatInputPreview(input.value) }}
                span(
                  v-else
                  class="flow-node-input-preview text-xs"
                  :class="isPortConnected(node.id, 'value-input-' + input.id) ? 'flow-node-input-preview--attached' : ''"
                ) {{ isPortConnected(node.id, 'value-input-' + input.id) ? "Connected" : "Connection only" }}

          // Enhanced outputs section (fixed-width right column to preserve input field sizing)
          div(class="flow-output-column flex-shrink-0 space-y-3 ml-auto")
            div(
              v-for="output in flowRouteOutputs(node)"
              :key="'route-' + output.id"
              class="flow-value-row flow-value-row--output flex items-center text-sm justify-end gap-x-3 group max-w-full"
              :data-flow-output-id="output.id"
            )
              span(class="text-themeText font-medium truncate" :title="output.name") {{ output.name }}
              button(
                class="flow-output-port relative bg-transparent border-0 outline-none p-0"
                @mousedown.stop="startNamedFlowOutputConnection(node, output.id, $event)"
                @mouseenter="setHoveredFlowPort({nodeId: node.id, type: 'flow-output-' + output.id})"
                @mouseleave="isConnecting ? '' : setHoveredFlowPort(null)"
              )
                SolarPlayBold(
                  v-if="isPortConnected(node.id, 'flow-output-' + output.id)"
                  class="relative w-5 h-5 text-themePrimary"
                )
                SolarPlayBroken(v-else class="relative w-5 h-5 text-themeText3")
            div(
              v-for="output in (node.outputs || []).filter(output => output.type !== 'flow')"
              :key="output.id"
              class="flow-value-row flow-value-row--output flex items-center text-sm justify-end gap-x-3 group max-w-full"
              :data-output-id="output.id"
            )
              div(class="flex items-center gap-x-2 min-w-0 flex-1 justify-end")
                span(class="text-themeText font-medium truncate" :title="output.name") {{ output.name }}
              div(
                :style="{ borderColor: getTypeColor(output.type), backgroundColor: isPortConnected(node.id, 'value-output-' + output.id) ? getTypeColor(output.type) : 'transparent', boxShadow: isPortConnected(node.id, 'value-output-' + output.id) ? `0 0 8px ${getTypeColor(output.type)}40` : 'none' }"
                class="value-output-port flow-value-port flow-value-port--output w-3.5 h-3.5 aspect-square rounded-full cursor-pointer z-5 hover:scale-125 transition-all duration-200 group-hover:shadow-xl flex-shrink-0 border-2"
                @mousedown.stop="startConnection(node, $event, 'value-output-' + output.id)"
              )

        // Preview Image Section (for image-output / preview nodes) - visible in compact mode too
        div(
          v-if="node.type === 'image-output' || node.type === 'preview-image' || node.type === 'webcam-capture'"
          class="flow-preview-media-section w-full mt-2 pt-2 pb-2 border-t border-themeBorder min-h-0"
        )
          div(
            class="flex-1 min-h-0 rounded-xl border border-themeBorder bg-themeBackground p-2 overflow-hidden"
          )
            img(
              v-if="getPreviewImageSrc(node) && !isPreviewImageErrored(node)"
              :src="getPreviewImageSrc(node)"
              class="w-full h-full min-h-0 rounded-lg object-contain"
              alt=""
              @load="clearPreviewImageError(node.id)"
              @error="handlePreviewImageError(node)"
            )
            div(
              v-else-if="isPreviewImageErrored(node)"
              class="rounded-lg border border-dashed border-red-500/45 bg-red-500/10 text-red-400 text-xs text-center px-2 py-3"
            )
              div(class="font-semibold") Image preview failed
              div(class="mt-1 text-[10px] text-red-400/80 break-all max-h-9 overflow-hidden") {{ getPreviewImageErrorMessage(node) }}
            div(
              v-else
              class="rounded-lg border border-dashed border-themeBorder text-themeText3 text-xs text-center py-3 px-2"
            )
              span(v-if="isPortConnected(node.id, 'value-input-imageData')") Connected image input. Run flow to render preview.
              span(v-else) Connect image data to preview.

        // Display Text Section (for display-text nodes) - visible in compact mode
        div(
          v-if="node.type === 'display-text'"
          class="flow-preview-media-section w-full mt-2 pt-2 pb-2 border-t border-themeBorder min-h-0"
        )
          div(
            class="flex-1 min-h-0 rounded-xl border border-themeBorder bg-themeBackground p-2 overflow-auto"
          )
            div(
              v-if="getDisplayTextValue(node, 'text')"
              class="w-full h-full min-h-0 rounded-lg px-3 py-2 text-center break-words overflow-auto"
              style="word-wrap: break-word; hyphens: auto;"
              :style="getDisplayTextStyle(node)"
            ) {{ getDisplayTextValue(node, 'text') }}
            div(
              v-else
              class="rounded-lg border border-dashed border-themeBorder text-themeText3 text-xs text-center py-3 px-2"
            )
              span(v-if="isPortConnected(node.id, 'value-input-text')") Connected text input. Run flow to render preview.
              span(v-else) Enter or connect text to preview.

        // Preview File Section (for preview-file nodes) - visible in compact mode too
        div(
          v-if="node.type === 'preview-file'"
          class="flow-preview-media-section w-full mt-2 pt-2 pb-2 border-t border-themeBorder min-h-0"
        )
          div(
            class="flex-1 min-h-0 rounded-xl border border-themeBorder bg-themeBackground p-2 overflow-auto"
          )
            component(
              v-if="getPreviewFileData(node)"
              :is="getWidgetComponent('preview-file')"
              :filePath="getPreviewFileData(node).filePath"
              :fileName="getPreviewFileData(node).fileName"
              :fileType="getPreviewFileData(node).fileType"
            )
            div(
              v-else-if="isPortConnected(node.id, 'value-input-fileData')"
              class="rounded-lg border border-dashed border-themePrimary/40 bg-themePrimary/10 text-themePrimary text-xs text-center py-3 px-2"
            )
              div(class="font-semibold") File input connected
              div(class="mt-1 text-[10px] text-themeText2") Run flow to generate preview metadata.
            div(
              v-else
              class="rounded-lg border border-dashed border-themeBorder bg-themeBackground text-themeText3 text-xs text-center py-3 px-2"
            )
              div(class="font-semibold") No file preview
              div(class="mt-1 text-[10px] text-themeText3") Connect a file path to `fileData`.

      // Minimized mode: only border flow ports (assignment-card behavior)
      div(
        v-if="node.minimized && shouldShowFlowInputPort(node)"
        class="absolute left-0 top-1/2 -translate-x-1/2 -translate-y-1/2 z-20"
      )
        button(
          class="flow-input-port block bg-transparent border-0 outline-none p-0"
          @mousedown.stop="startConnection(node, $event, 'flow-input')"
          @mouseenter="setHoveredFlowPort({nodeId: node.id, type: 'flow-input'})"
          @mouseleave="setHoveredFlowPort(null)"
        )
          SolarPlayBold(
            v-if="isPortConnected(node.id, 'flow-input')"
            class="relative w-5 h-5 text-white"
          )
          SolarPlayBroken(
            v-else-if="hoveredFlowPort && hoveredFlowPort.nodeId === node.id && hoveredFlowPort.type === 'flow-input'"
            class="relative w-5 h-5 text-white"
          )
          SolarPlayBroken(v-else class="relative w-5 h-5 text-white/70")

      div(
        v-if="node.minimized && shouldShowFlowOutputPort(node) && hasDefaultFlowOutput(node)"
        class="absolute right-0 top-1/2 translate-x-1/2 -translate-y-1/2 z-20"
      )
        button(
          class="flow-output-port block bg-transparent border-0 outline-none p-0"
          @mousedown.stop="startFlowOutputConnection(node, $event)"
          @mouseenter="setHoveredFlowPort({nodeId: node.id, type: 'flow-output'})"
          @mouseleave="isConnecting ? '' : setHoveredFlowPort(null)"
        )
          SolarPlayBold(
            v-if="isPortConnected(node.id, 'flow-output')"
            class="relative w-5 h-5 text-white"
          )
          SolarPlayBroken(
            v-else-if="hoveredFlowPort && hoveredFlowPort.nodeId === node.id && hoveredFlowPort.type === 'flow-output'"
            class="relative w-5 h-5 text-white"
          )
          SolarPlayBroken(v-else class="relative w-5 h-5 text-white/70")

      div(
        v-for="(output, routeIndex) in (node.minimized ? flowRouteOutputs(node) : [])"
        :key="'min-route-' + output.id"
        class="absolute right-0 translate-x-1/2 -translate-y-1/2 z-20 flex items-center gap-1"
        :style="{ top: `${((routeIndex + 1) / (flowRouteOutputs(node).length + 1)) * 100}%` }"
        :data-flow-output-id="output.id"
      )
        button(
          class="flow-output-port block bg-transparent border-0 outline-none p-0"
          :title="output.name"
          @mousedown.stop="startNamedFlowOutputConnection(node, output.id, $event)"
        )
          SolarPlayBold(v-if="isPortConnected(node.id, 'flow-output-' + output.id)" class="relative w-5 h-5 text-white")
          SolarPlayBroken(v-else class="relative w-5 h-5 text-white/70")

      // Resize handle (bottom-right corner)
      div(
        class="absolute bottom-0 right-0 w-4 h-4 cursor-nwse-resize opacity-0 hover:opacity-100 transition-opacity z-30"
        @mousedown.stop="startResize(node, $event)"
      )
        div(class="absolute bottom-1 right-1 w-3 h-3 border-r-2 border-b-2 border-white/50 rounded-br")

  // Permanent connections (fixed positioned, outside canvas)
  svg(
    class="fixed top-0 left-0 w-screen h-screen z-10"
    :style="svgConnectionStyles"
  )
    // Gradient definitions for colored connections
    defs
      // Flow secondary branch gradient (dashed neon)
      linearGradient(id="flowGradientBranch" x1="0%" y1="0%" x2="100%" y2="0%")
        stop(offset="0%" style="stop-color:#d946ef;stop-opacity:1")
        stop(offset="100%" style="stop-color:#c026d3;stop-opacity:1")

      // Flow connection gradient (solid fallback)
      linearGradient(id="flowGradient" x1="0%" y1="0%" x2="100%" y2="0%")
        stop(offset="0%" style="stop-color:#8b5cf6;stop-opacity:1")
        stop(offset="100%" style="stop-color:#a855f7;stop-opacity:1")

      // String connection gradient (blue)
      linearGradient(id="stringGradient" x1="0%" y1="0%" x2="100%" y2="0%")
        stop(offset="0%" style="stop-color:#3b82f6;stop-opacity:1")
        stop(offset="100%" style="stop-color:#1d4ed8;stop-opacity:1")

      // Number connection gradient (red)
      linearGradient(id="numberGradient" x1="0%" y1="0%" x2="100%" y2="0%")
        stop(offset="0%" style="stop-color:#ef4444;stop-opacity:1")
        stop(offset="100%" style="stop-color:#dc2626;stop-opacity:1")

      // Boolean connection gradient (green)
      linearGradient(id="booleanGradient" x1="0%" y1="0%" x2="100%" y2="0%")
        stop(offset="0%" style="stop-color:#10b981;stop-opacity:1")
        stop(offset="100%" style="stop-color:#059669;stop-opacity:1")

      // Object connection gradient (purple)
      linearGradient(id="objectGradient" x1="0%" y1="0%" x2="100%" y2="0%")
        stop(offset="0%" style="stop-color:#8b5cf6;stop-opacity:1")
        stop(offset="100%" style="stop-color:#7c3aed;stop-opacity:1")

      // Array connection gradient (orange)
      linearGradient(id="arrayGradient" x1="0%" y1="0%" x2="100%" y2="0%")
        stop(offset="0%" style="stop-color:#f97316;stop-opacity:1")
        stop(offset="100%" style="stop-color:#ea580c;stop-opacity:1")

      // Orchestrator connection gradient (bright teal AI management)
      linearGradient(id="orchestratorGradient" x1="0%" y1="0%" x2="100%" y2="0%")
        stop(offset="0%" style="stop-color:#14b8a6;stop-opacity:1")
        stop(offset="50%" style="stop-color:#5eead4;stop-opacity:1")
        stop(offset="100%" style="stop-color:#0d9488;stop-opacity:1")

      // Glow filter
      filter(id="glow")
        feGaussianBlur(:stdDeviation="Math.max(1, 3 * zoomRatio)" result="coloredBlur")
        feMerge
          feMergeNode(in="coloredBlur")
          feMergeNode(in="SourceGraphic")

      // Drop shadow filter
      filter(id="dropshadow")
        feGaussianBlur(in="SourceAlpha" stdDeviation="2")
        feOffset(dx="1" dy="2" result="offset")
        feFlood(flood-color="#000000" flood-opacity="0.3")
        feComposite(in2="offset" operator="in")
        feMerge
          feMergeNode
          feMergeNode(in="SourceGraphic")

    // Connection paths with enhanced visuals
    g(v-for="conn in validConnections" :key="conn.id" style="pointer-events: auto;")
      // Invisible clickable area for connection
      path(
        :d="connectionPaths[conn.id]"
        fill="none"
        stroke="rgba(255,255,255,0.01)"
        stroke-width="20"
        style="cursor: pointer; pointer-events: auto;"
        @click.stop="deleteConnection(conn.id)"
        @mouseenter="emit('updateHoveredConnection', conn.id)"
        @mouseleave="emit('updateHoveredConnection', null)"
      )

      // Background stroke for better visibility (dark outline)
      path(
        :class="getConnectionVisual(conn).outlineClass"
        :d="connectionPaths[conn.id]"
        fill="none"
        :stroke="getConnectionVisual(conn).outlineColor"
        :stroke-width="getConnectionVisual(conn).outlineWidth"
        stroke-linecap="round"
        stroke-linejoin="round"
        style="pointer-events: none;"
      )

      // Main connection with glow effect
      path(
        :class="getConnectionVisual(conn).pathClass"
        :id="'path-' + conn.id"
        :d="connectionPaths[conn.id]"
        fill="none"
        :stroke="getConnectionVisual(conn).stroke"
        :stroke-width="getConnectionVisual(conn).strokeWidth"
        :stroke-dasharray="getConnectionVisual(conn).dashArray"
        stroke-linecap="round"
        stroke-linejoin="round"
        filter="url(#glow)"
        :style="{ strokeDasharray: getConnectionVisual(conn).dashArray, pointerEvents: 'none', transition: 'stroke-width 0.2s ease, stroke 0.2s ease' }"
      )

      // Hover indicator text
      text(
        v-if="props.hoveredConnection === conn.id"
        :x="getConnectionMidpoint(conn).x"
        :y="getConnectionMidpoint(conn).y - 10"
        text-anchor="middle"
        fill="#ef4444"
        font-size="12"
        font-weight="bold"
        style="pointer-events: none;"
      ) Click to delete

      // Enhanced animated flow indicator for flow connections
      circle(
        v-if="conn.kind === 'control'"
        :r="((isOrchestratorConnection(conn) ? 3 : 2.5) * zoomRatio).toFixed(1)"
        :fill="getConnectionColor(conn)"
        class="flow-indicator"
        :opacity="isOrchestratorConnection(conn) ? '0.9' : '0.8'"
      )
        animateMotion(
          :dur="isOrchestratorConnection(conn) ? '1.5s' : (getAnimationDuration(conn) + 's')"
          repeatCount="indefinite"
        )
          mpath(:href="'#path-' + conn.id")
      
      // Additional AI management indicator for Orchestrator connections
      circle(
        v-if="isOrchestratorConnection(conn)"
        :r="(1.5 * zoomRatio).toFixed(1)"
        fill="#5eead4"
        class="ai-management-indicator"
        opacity="0.8"
      )
        animateMotion(
          dur="1s"
          repeatCount="indefinite"
          begin="0.5s"
        )
          mpath(:href="'#path-' + conn.id")
</template>

<script setup lang="ts">
import MaterialSymbolsDataObject from "~icons/material-symbols/data-object";
import SolarPlayBroken from "~icons/solar/play-broken";
import SolarPlayBold from "~icons/solar/play-bold";
import { convertFileSrc } from "@tauri-apps/api/core";

// Import widget registry for dynamic widget resolution
import { getWidgetComponent } from "../../composables/useWidgetRegistry";

import { computed, ref, toRef } from "vue";
import { AiNodeBehavior, type FlowCanvasConnection } from "../../types/flow";
import { getTypeColor } from "../../utils/typeColors";
import { getNamedFlowRouteOutputs } from "../../utils/portSemantics";
import { useVariableNodeSync } from "../../composables/useVariableNodeSync";

// Define interfaces for the props
// Type definitions
interface Node {
  persistedDefinition?: { id: string };
  id: number;
  type: string;
  text: string;
  x: number;
  y: number;
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
  invocationMode?: boolean; // Compact output-only view for referenced actions
  minimized?: boolean; // Header-only display mode
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
  required?: boolean; // Whether this input is required
  forceInput?: boolean; // If true, this input MUST be a connection (no widget shown)
  displayOptions?: {
    show?: Record<string, any[]>; // Show this field when other fields match these values
    hide?: Record<string, any[]>; // Hide this field when other fields match these values
  };
  // Widget metadata for UI controls
  widget?: string; // 'slider', 'color', 'toggle', 'file', 'textarea', 'select', 'multiselect', 'text', 'number'
  widgetOptions?: {
    min?: number;
    max?: number;
    step?: number;
    accept?: string[]; // File types for file picker
    multiline?: boolean; // For text areas
    rows?: number; // Number of rows for textarea
    options?: string[]; // Options for multi-select dropdown
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

interface Camera {
  x: number;
  y: number;
  scale: number;
}

interface HoveredFlowPort {
  nodeId: number;
  type: string;
}

// Props
interface Props {
  nodes: Node[];
  connections: Connection[];
  nodeTypeColors: Record<string, string>;
  camera: Camera;
  selectedNodes: Set<number>;
  selectedNode: Node | null;
  isDragging: boolean;
  isResizing: boolean;
  resizingNodeId: number | null;
  isSelecting: boolean;
  isConnecting: boolean;
  connectionStart: { x: number; y: number } | null;
  connectionEnd: { x: number; y: number } | null;
  hoveredFlowPort: HoveredFlowPort | null;
  hoveredConnection: number | null;
  selectionBoxStyle: Record<string, string>;
  gridStyle: Record<string, string>;
  connectionPaths: Record<string, string>;
  validConnections: Connection[];
  // Node execution state methods
  getNodeStatusClasses: (node: Node) => string;
  getNodeStatusTitle: (node: Node) => string;
  getNodeExecutionTime: (node: Node) => number | null;
  getNodeRetryCount: (node: Node) => number;
  isNodeRunningConcurrently: (node: Node) => boolean;
  getAiModelName: (aiModel: string) => string;
  getAiModelShortName: (aiModel: string) => string;
  getTemplateName: (templateId: string) => string;
  getTemplateShortName: (templateId: string) => string;
  isTextOnlyNode: (node: Node) => boolean;
  formatExecutionTime: (timeMs: number) => string;
  // Node progress tracking
  nodeProgress: Map<string, { progress: number; message: string }>;
  // Connection-related functions
  getConnectionColor: (connection: Connection) => string;
  getConnectionStrokeWidth: (connection: Connection) => number;
  getAnimationDuration: (connection: Connection) => number;
  getConnectionMidpoint: (connection: Connection) => { x: number; y: number };
}

const props = defineProps<Props>();

const openNodeActionsMenuId = ref<number | null>(null);

const formatInputPreview = (value: unknown): string => {
  if (value === null || value === undefined || value === "") {
    return "Set in inspector";
  }
  if (typeof value === "boolean") {
    return value ? "On" : "Off";
  }
  if (Array.isArray(value)) {
    return value.length ? `${value.length} items` : "No items";
  }
  if (typeof value === "object") {
    try {
      const text = JSON.stringify(value);
      return text.length > 44 ? `${text.slice(0, 44)}...` : text;
    } catch {
      return "Object value";
    }
  }
  const text = String(value);
  return text.length > 44 ? `${text.slice(0, 44)}...` : text;
};

const isCssColorLike = (value: string): boolean => {
  const trimmed = value.trim();
  if (!trimmed) return false;

  const isHex = /^#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/.test(trimmed);
  const isRgb = /^rgba?\(([^)]+)\)$/.test(trimmed);
  const isHsl = /^hsla?\(([^)]+)\)$/.test(trimmed);

  return isHex || isRgb || isHsl;
};

const getInputColorPreview = (input: NodeIO): string | null => {
  if (typeof input.value !== "string") return null;

  const isColorField =
    input.widget === "color" ||
    /color/i.test(input.id || "") ||
    /color/i.test(input.name || "");

  if (!isColorField) return null;
  return isCssColorLike(input.value) ? input.value.trim() : null;
};

// Sync variable node names/types
const variableNodeSync = useVariableNodeSync(
  toRef(() => props.nodes),
  toRef(() => props.connections)
);

// Expose sync methods for parent components to call
defineExpose({
  onConnectionChange: variableNodeSync.onConnectionChange,
  onVariableNameChange: variableNodeSync.onVariableNameChange,
});

// Emits
const emit = defineEmits<{
  drop: [event: DragEvent];
  mouseMove: [event: MouseEvent];
  mouseUp: [event: MouseEvent];
  startPan: [event: MouseEvent];
  canvasContextMenu: [event: MouseEvent];
  clickOutside: [event: MouseEvent];
  zoom: [event: WheelEvent];
  cancelConnection: [];
  unselectNodes: [];
  startDrag: [node: Node, event: MouseEvent];
  startResize: [node: Node, event: MouseEvent];
  startConnection: [node: Node, event: MouseEvent, portType: string];
  deleteNode: [node: Node];
  toggleMinimized: [nodeId: number];
  aiModelClick: [node: Node];
  showTriggerChangeMenu: [node: Node];
  deleteConnection: [connectionId: number];
  updateHoveredFlowPort: [port: HoveredFlowPort | null];
  updateHoveredConnection: [connectionId: number | null];
  openInspector: [node: Node];
}>();

// Helper functions
const isTriggerNode = (nodeType: string): boolean => {
  return (
    nodeType?.includes("trigger") ||
    nodeType?.includes("Trigger") ||
    nodeType === "schedule"
  );
};

const isAiNode = (node: Node): boolean => {
  return (
    node.type?.toLowerCase().includes("ai") ||
    node.type?.toLowerCase().includes("gpt") ||
    node.type?.toLowerCase().includes("chat") ||
    node.type?.toLowerCase().includes("hybrid-llm") // Include hybrid LLM nodes
  );
};

// Nodes that should behave as value-only graph nodes (no main flow play ports).
const VALUE_ONLY_NODE_TYPES = new Set(["get-variable", "set-variable"]);

const isValueOnlyNode = (node: Node): boolean => VALUE_ONLY_NODE_TYPES.has(node.type);

const hasControlFlowConnection = (node: Node): boolean =>
  props.connections.some(
    (conn) =>
      conn.kind === "control"
      && (conn.from === node.id || conn.to === node.id)
  );

const shouldShowFlowInputPort = (node: Node): boolean =>
  !isTriggerNode(node.type) && (!isValueOnlyNode(node) || hasControlFlowConnection(node));

const shouldShowFlowOutputPort = (node: Node): boolean =>
  !isValueOnlyNode(node) || hasControlFlowConnection(node);

const flowRouteOutputs = (node: Node): NodeIO[] =>
  getNamedFlowRouteOutputs(node.outputs);

const hasDefaultFlowOutput = (node: Node): boolean =>
  (node.outputs || []).some((output) => output.type === "flow" && output.id === "output");

const getVisibleInputCount = (node: Node): number =>
  (node.inputs || []).filter((input) => isInputVisible(input, node)).length;

const getVisibleOutputCount = (node: Node): number =>
  (node.outputs || []).filter((output) => output.type !== "flow").length;

const getNodeKindLabel = (node: Node): string => {
  const type = (node.type || "").toLowerCase();

  if (isTriggerNode(node.type)) return "TRIGGER";
  if (type.includes("condition") || type.includes("branch") || type.includes("if"))
    return "CONDITION";
  if (isAiNode(node)) return "AI";
  if (type.includes("delay") || type.includes("wait") || type.includes("timer"))
    return "TIMING";

  return "ACTION";
};

const isPortConnected = (nodeId: number, portType: string): boolean => {
  return props.connections.some((conn) => {
    if (portType.includes("input")) {
      return conn.to === nodeId && conn.toType === portType;
    } else {
      return conn.from === nodeId && conn.fromType === portType;
    }
  });
};

// Use prop functions directly
const getNodeStatusClasses = (node: Node) => props.getNodeStatusClasses(node);
const getNodeStatusTitle = (node: Node) => props.getNodeStatusTitle(node);

const getActionPreflightLabel = (status: string): string => ({
  ready: "Ready",
  syncing: "Syncing",
  disconnected: "Offline",
  missing_integration: "Missing",
  missing_target: "Target missing",
  permission_denied: "Permission",
  invalid_action: "Repair",
  contract_drift: "Update",
  receipt_degraded: "Limited receipt",
}[status] || "Check");

const getActionPreflightClasses = (status: string): string => {
  if (status === "ready") return "bg-emerald-500/15 text-emerald-300";
  if (["syncing", "disconnected", "receipt_degraded"].includes(status)) {
    return "bg-amber-500/15 text-amber-300";
  }
  return "bg-red-500/15 text-red-300";
};

const getActionPreflightTitle = (
  preflight: import("@base/types/actionReference").ActionPreflight,
): string => preflight.issues?.map((issue) => issue.message).join("; ") || "Action is ready";
const getNodeExecutionTime = (node: Node) => props.getNodeExecutionTime(node);
const getNodeRetryCount = (node: Node) => props.getNodeRetryCount(node);
const isNodeRunningConcurrently = (node: Node) => props.isNodeRunningConcurrently(node);
const getAiModelName = (aiModel: string) => props.getAiModelName(aiModel);
const getAiModelShortName = (aiModel: string) => props.getAiModelShortName(aiModel);
const getTemplateName = (templateId: string) => props.getTemplateName(templateId);
const getTemplateShortName = (templateId: string) =>
  props.getTemplateShortName(templateId);
const isTextOnlyNode = (node: Node) => props.isTextOnlyNode(node);
const formatExecutionTime = (timeMs: number) => props.formatExecutionTime(timeMs);

// Get node progress (for progress bars)
const getNodeProgress = (node: Node): { progress: number; message: string } | undefined => {
  const progress = props.nodeProgress.get(node.persistedDefinition?.id ?? String(node.id));
  if (progress) {
    console.log(`📊 Canvas: getNodeProgress for node ${node.id}:`, progress);
  }
  return progress;
};

// Helper functions for Orchestrator detection
const isNodeManagedByOrchestrator = (node: Node): boolean => {
  return props.connections.some((conn) => {
    if (conn.to === node.id) {
      const sourceNode = props.nodes.find((n) => n.id === conn.from);
      return sourceNode?.isAiNode === true && sourceNode?.aiBehavior === "Orchestrator";
    }
    return false;
  });
};

const getOrchestratorNodeName = (node: Node): string => {
  const conn = props.connections.find((c) => c.kind === "control" && c.to === node.id);
  if (conn) {
    const sourceNode = props.nodes.find((n) => n.id === conn.from);
    if (sourceNode?.isAiNode && sourceNode?.aiBehavior === "Orchestrator") {
      return sourceNode.text || "AI Orchestrator";
    }
  }
  return "";
};

const isOrchestratorConnection = (conn: Connection): boolean => {
  if (conn.kind !== "control") return false;
  const sourceNode = props.nodes.find((n) => n.id === conn.from);
  return sourceNode?.isAiNode === true && sourceNode?.aiBehavior === "Orchestrator";
};

// Optimized Display Text helpers - cache input lookups to avoid repeated .find() calls
const getDisplayTextValue = (node: Node, inputId: string): any => {
  // Priority: execution inputs (from connections) > widget inputs (manual entry)
  return node.executionState?.last_inputs?.[inputId]
    ?? node.inputs?.find(i => i.id === inputId)?.value;
};

const getDisplayTextStyle = (node: Node): Record<string, string> => {
  const backgroundColor = getDisplayTextValue(node, 'backgroundColor') || '#1f2937';
  const color = getDisplayTextValue(node, 'color') || '#ffffff';
  const fontSize = getDisplayTextValue(node, 'fontSize') || 14;

  return {
    backgroundColor,
    color,
    fontSize: `${fontSize}px`
  };
};

// Optimized Preview Image helper - same pattern as Display Text
const getPreviewImageValue = (node: Node): string | null => {
  const rawValue =
    node.executionState?.last_inputs?.imageData
    ?? node.executionState?.last_output?.display?.imageData
    ?? node.inputs?.find((i) => i.id === "imageData")?.value
    ?? null;

  if (typeof rawValue === "string") {
    return rawValue;
  }

  if (rawValue && typeof rawValue === "object") {
    const objectValue = rawValue as Record<string, unknown>;
    const nested =
      objectValue.imageData
      ?? objectValue.filePath
      ?? (
        objectValue.display && typeof objectValue.display === "object"
          ? (objectValue.display as Record<string, unknown>).imageData
          : null
      )
      ?? null;

    return typeof nested === "string" ? nested : null;
  }

  return null;
};

const normalizePreviewImageSrc = (value: string): string => {
  let source = value.trim();
  if (!source) return "";

  // Sometimes payload arrives as JSON-serialized string/object.
  // Example: "{\"filePath\":\"\\\\?\\C:\\\\...\\\\x.png\"}"
  if (source.startsWith("{") && source.endsWith("}")) {
    try {
      const parsed = JSON.parse(source) as Record<string, unknown>;
      const nested =
        parsed.imageData
        ?? parsed.filePath
        ?? parsed.path
        ?? parsed.url
        ?? (
          parsed.display && typeof parsed.display === "object"
            ? (parsed.display as Record<string, unknown>).imageData
            : null
        )
        ?? null;
      if (typeof nested === "string" && nested.trim()) {
        source = nested.trim();
      }
    } catch {
      // Keep original string if parse fails.
    }
  } else if (
    (source.startsWith('"') && source.endsWith('"')) ||
    (source.startsWith("'") && source.endsWith("'"))
  ) {
    // Quoted path/string payload
    source = source.slice(1, -1).trim();
  }

  // Already a browser-safe source
  if (
    source.startsWith("data:image/") ||
    source.startsWith("http://") ||
    source.startsWith("https://") ||
    source.startsWith("blob:") ||
    source.startsWith("tauri://") ||
    source.startsWith("asset://")
  ) {
    return source;
  }

  // Handle file:// URLs
  if (source.startsWith("file://")) {
    try {
      const rawPath = decodeURIComponent(source.replace(/^file:\/\//, ""));
      // file:///C:/... -> C:/...
      source = rawPath.replace(/^\/([a-zA-Z]:[\\/])/, "$1");
    } catch {
      // Keep source if decode fails
    }
  }

  // Local filesystem path (Windows or Unix) -> Tauri-safe URL.
  // Strip Windows long-path prefix first: \\?\C:\...
  let filePathSource = source;
  if (filePathSource.startsWith("\\\\?\\")) {
    filePathSource = filePathSource.slice(4);
  }

  const isWindowsPath = /^[a-zA-Z]:[\\/]/.test(filePathSource);
  const isUnixPath = filePathSource.startsWith("/");
  if (isWindowsPath || isUnixPath) {
    try {
      return convertFileSrc(filePathSource.replace(/\\/g, "/"));
    } catch {
      return filePathSource;
    }
  }

  // Raw base64 payload (without data URI prefix)
  const compact = source.replace(/\s+/g, "");
  const looksLikeBase64 =
    compact.length > 40 &&
    /^[A-Za-z0-9+/=]+$/.test(compact);
  if (looksLikeBase64) {
    return `data:image/png;base64,${compact}`;
  }

  return source;
};

const getPreviewImageSrc = (node: Node): string | null => {
  const raw = getPreviewImageValue(node);
  if (!raw) return null;
  const normalized = normalizePreviewImageSrc(raw);
  return normalized || null;
};

const isPreviewNode = (nodeType: string): boolean =>
  nodeType === "preview-image" ||
  nodeType === "preview-file" ||
  nodeType === "display-text";

const previewImageErrors = ref<Record<number, { source: string; message: string }>>({});

const clearPreviewImageError = (nodeId: number): void => {
  if (!(nodeId in previewImageErrors.value)) return;
  const next = { ...previewImageErrors.value };
  delete next[nodeId];
  previewImageErrors.value = next;
};

const isPreviewImageErrored = (node: Node): boolean => {
  const entry = previewImageErrors.value[node.id];
  const currentSource = getPreviewImageSrc(node) || "";
  return !!entry && entry.source === currentSource;
};

const getPreviewImageErrorMessage = (node: Node): string => {
  const entry = previewImageErrors.value[node.id];
  if (!entry) return "Unknown image source error";
  return entry.message;
};

const handlePreviewImageError = (node: Node): void => {
  const source = getPreviewImageSrc(node) || "";
  const compactSource = source.length > 72 ? `${source.slice(0, 72)}...` : source;
  previewImageErrors.value = {
    ...previewImageErrors.value,
    [node.id]: {
      source,
      message: compactSource || "Missing image source",
    },
  };
};

// Preview File helper - returns file path, name, and type from OUTPUT
const getPreviewFileData = (node: Node): { filePath: string; fileName: string; fileType: string } | null => {
  // Get from execution output (after node has run)
  const result = node.executionState?.last_output?.result;

  if (result?.filePath) {
    return {
      filePath: result.filePath,
      fileName: result.fileName || 'file',
      fileType: result.fileType || 'application/octet-stream'
    };
  }

  return null;
};

// Use parent's connection functions
const getConnectionColor = (connection: Connection) =>
  props.getConnectionColor(connection);
const getConnectionStrokeWidth = (connection: Connection) =>
  props.getConnectionStrokeWidth(connection);
const getAnimationDuration = (connection: Connection) =>
  props.getAnimationDuration(connection);
const getConnectionMidpoint = (connection: Connection) =>
  props.getConnectionMidpoint(connection);

interface ConnectionVisual {
  outlineClass: string[];
  pathClass: string[];
  outlineColor: string;
  outlineWidth: number;
  stroke: string;
  strokeWidth: number;
  dashArray: string;
}

const FLOW_DASHED_STROKE = "url(#flowGradientBranch)";
const HOVER_STROKE = "#ef4444";
const OUTLINE_STROKE = "rgba(9,14,28,0.7)";

const isFlowConnection = (conn: Connection): boolean =>
  conn.kind === "control";

const isValueConnection = (conn: Connection): boolean =>
  conn.kind === "data";

const isDashedConnection = (conn: Connection): boolean =>
  isFlowConnection(conn) || isValueConnection(conn);

// Smooth zoom-damped ratio for connection stroke, outline, glow, and dash adaptation
const zoomRatio = computed(() => {
  const scale = props.camera?.scale ?? 1;
  return Math.max(0.45, Math.min(1.2, 0.35 + 0.65 * scale));
});

// Dynamic dash array that preserves segment count as the canvas zooms out
const currentDashArray = computed(() => {
  const ratio = zoomRatio.value;
  const dash = Math.max(3.5, Number((10 * ratio).toFixed(1)));
  const gap = Math.max(2.5, Number((7 * ratio).toFixed(1)));
  return `${dash} ${gap}`;
});

// Dynamic styles and CSS variables applied to the root connection SVG
const svgConnectionStyles = computed(() => {
  const ratio = zoomRatio.value;
  const dashLength = Math.max(3.5, Number((10 * ratio).toFixed(1)));
  const dashGap = Math.max(2.5, Number((7 * ratio).toFixed(1)));
  const cycle = (dashLength + dashGap) * 2;
  const glow1 = Math.max(1, Number((4 * ratio).toFixed(1)));
  const glow2 = Math.max(2, Number((8 * ratio).toFixed(1)));
  const opacityRatio = Math.max(0.3, ratio);

  return {
    overflow: "visible",
    pointerEvents: "none" as const,
    "--flow-dash-array": `${dashLength}px ${dashGap}px`,
    "--flow-dash-cycle": `-${cycle.toFixed(1)}px`,
    "--flow-glow-1": `${glow1}px`,
    "--flow-glow-2": `${glow2}px`,
    "--flow-glow-opacity-1": (0.38 * opacityRatio).toFixed(2),
    "--flow-glow-opacity-2": (0.22 * opacityRatio).toFixed(2),
  };
});

const connectionVisuals = computed<Record<number, ConnectionVisual>>(() => {
  const visuals: Record<number, ConnectionVisual> = {};
  const ratio = zoomRatio.value;

  for (const conn of props.validConnections) {
    const dashed = isDashedConnection(conn);
    const hovered = props.hoveredConnection === conn.id;
    const baseStroke = isFlowConnection(conn)
      ? FLOW_DASHED_STROKE
      : getConnectionColor(conn);
    const baseWidth = getConnectionStrokeWidth(conn) * ratio;

    visuals[conn.id] = {
      outlineClass: [
        "connection-outline",
        ...(dashed ? ["connection-outline--dashed"] : []),
      ],
      pathClass: [
        "connection-path",
        ...(dashed ? ["connection-path--dashed"] : []),
      ],
      outlineColor: dashed ? "transparent" : OUTLINE_STROKE,
      outlineWidth: dashed
        ? (hovered ? 3.5 * ratio : baseWidth + 0.35 * ratio)
        : (hovered ? 4.5 * ratio : baseWidth + 1.0 * ratio),
      stroke: hovered ? HOVER_STROKE : baseStroke,
      strokeWidth: hovered ? 4.5 * ratio : baseWidth,
      dashArray: dashed ? currentDashArray.value : "none",
    };
  }

  return visuals;
});

const getConnectionVisual = (conn: Connection): ConnectionVisual => {
  const ratio = zoomRatio.value;
  const baseWidth = getConnectionStrokeWidth(conn) * ratio;
  return (
    connectionVisuals.value[conn.id] ?? {
      outlineClass: ["connection-outline"],
      pathClass: ["connection-path"],
      outlineColor: OUTLINE_STROKE,
      outlineWidth: baseWidth + 1.0 * ratio,
      stroke: getConnectionColor(conn),
      strokeWidth: baseWidth,
      dashArray: "none",
    }
  );
};

// Helper function to check if an input should be visible based on displayOptions
const isInputVisible = (input: NodeIO, node: Node): boolean => {
  if (!input.displayOptions) return true;

  const { show, hide } = input.displayOptions;

  // Check "show" conditions
  if (show) {
    for (const [fieldName, allowedValues] of Object.entries(show)) {
      const fieldInput = node.inputs?.find(i => i.id === fieldName);
      if (!fieldInput || !allowedValues.includes(fieldInput.value)) {
        return false;
      }
    }
  }

  // Check "hide" conditions
  if (hide) {
    for (const [fieldName, disallowedValues] of Object.entries(hide)) {
      const fieldInput = node.inputs?.find(i => i.id === fieldName);
      if (fieldInput && disallowedValues.includes(fieldInput.value)) {
        return false;
      }
    }
  }

  return true;
};

// Event handlers
const onDrop = (event: DragEvent) => {
  emit("drop", event);
};

const handleMouseMove = (event: MouseEvent) => {
  emit("mouseMove", event);
};

const handleMouseUp = (event: MouseEvent) => {
  emit("mouseUp", event);
};

const startPan = (event: MouseEvent) => {
  emit("startPan", event);
};

const handleClickOutside = (event: MouseEvent) => {
  emit("clickOutside", event);
};

const handleCanvasMouseDown = (event: MouseEvent) => {
  openNodeActionsMenuId.value = null;
  startPan(event);
  handleClickOutside(event);
};

const handleCanvasContextMenu = (event: MouseEvent) => {
  openNodeActionsMenuId.value = null;
  emit("canvasContextMenu", event);
};

const handleZoom = (event: WheelEvent) => {
  emit("zoom", event);
};

const cancelConnection = () => {
  emit("cancelConnection");
};

const unselectNodes = () => {
  emit("unselectNodes");
};

const startDrag = (node: Node, event: MouseEvent) => {
  openNodeActionsMenuId.value = null;
  emit("startDrag", node, event);
};

const startResize = (node: Node, event: MouseEvent) => {
  emit("startResize", node, event);
};

const startConnection = (node: Node, event: MouseEvent, portType: string) => {
  emit("startConnection", node, event, portType);
};

const setHoveredFlowPort = (port: HoveredFlowPort | null) => {
  emit("updateHoveredFlowPort", port);
};

const startFlowOutputConnection = (node: Node, event: MouseEvent) => {
  emit("startConnection", node, event, "flow-output");
  emit("updateHoveredFlowPort", { nodeId: node.id, type: "flow-output" });
};

const startNamedFlowOutputConnection = (node: Node, routeId: string, event: MouseEvent) => {
  const type = `flow-output-${routeId}`;
  emit("startConnection", node, event, type);
  emit("updateHoveredFlowPort", { nodeId: node.id, type });
};

const openNodeInspector = (node: Node) => {
  openNodeActionsMenuId.value = null;
  emit("openInspector", node);
};

const handleDeleteClick = (node: Node) => {
  emit("deleteNode", node);
};

const toggleNodeActionsMenu = (nodeId: number) => {
  openNodeActionsMenuId.value =
    openNodeActionsMenuId.value === nodeId ? null : nodeId;
};

const handleNodeMenuDelete = (node: Node) => {
  openNodeActionsMenuId.value = null;
  handleDeleteClick(node);
};

const handleToggleMinimized = (node: Node) => {
  openNodeActionsMenuId.value = null;
  emit("toggleMinimized", node.id);
};

const handleAiModelClick = (node: Node, closeMenu: boolean = false) => {
  if (closeMenu) {
    openNodeActionsMenuId.value = null;
  }
  emit("aiModelClick", node);
};

const showTriggerChangeMenu = (node: Node) => {
  emit("showTriggerChangeMenu", node);
};

// Custom directive for node dimension initialization
// Each node independently initializes its own dimensions when mounted
const vNodeInit = {
  mounted(el: HTMLElement, binding: { value: any }) {
    const node = binding.value;

    // Only initialize if dimensions aren't set
    if (!node.width || !node.height) {
      // Use double rAF to ensure DOM is painted before measuring
      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          const rect = el.getBoundingClientRect();

          // Get camera scale from parent node element's transform
          const nodeElement = el.closest('.node') as HTMLElement;
          let cameraScale = 1;
          if (nodeElement) {
            const transform = window.getComputedStyle(nodeElement).transform;
            if (transform && transform !== 'none') {
              const matrix = new DOMMatrix(transform);
              cameraScale = matrix.a;
            }
          }

          const actualWidth = rect.width / cameraScale;
          const actualHeight = rect.height / cameraScale;

          // Initialize node dimensions (mutating the node object triggers Vue reactivity)
          node.width = actualWidth;
          node.height = actualHeight;
          node.minWidth = actualWidth;
          node.minHeight = actualHeight;
        });
      });
    }
  },
};

const deleteConnection = (connectionId: number) => {
  emit("deleteConnection", connectionId);
};

// Connection and path methods
const createTemporaryPath = (
  start: { x: number; y: number } | null,
  end: { x: number; y: number } | null
): string => {
  if (!start || !end) return "";
  return `M ${start.x} ${start.y} L ${end.x} ${end.y}`;
};

const createSmoothBezierPath = (
  start: { x: number; y: number },
  end: { x: number; y: number }
): string => {
  const dx = end.x - start.x;
  const dy = end.y - start.y;
  const curveStrength = Math.min(Math.abs(dx) * 0.5, 100);

  return `M ${start.x} ${start.y} C ${start.x + curveStrength} ${start.y}, ${
    end.x - curveStrength
  } ${end.y}, ${end.x} ${end.y}`;
};
</script>

<style lang="scss" scoped></style>
