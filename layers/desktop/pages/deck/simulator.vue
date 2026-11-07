<template lang="pug">
  div(class="simulator-editor-shell flex h-full flex-col justify-between")
    div(class="simulator-editor-header flex w-full flex-wrap items-center justify-center gap-2 pb-2")

      div(class="relative inline-flex items-center cursor-pointer")
        label(class="relative inline-flex items-center cursor-pointer")
          input(
            type="checkbox"
            :value="isInteractiveMode"
            @change="toggleInteractiveMode"
            class="sr-only peer"
          )
          div(
            class="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-gradient-to-r peer-checked:from-blue-500 peer-checked:to-purple-500"
          )
        span(class="ml-3 text-sm font-medium text-themeText") Interactive mode

      button(
        @click="rotateComponent"
        class="simulator-rotate-btn flex items-center justify-center p-2 rounded-full text-white shadow-md hover:shadow-lg transition-all duration-300"
        title="Rotate Screen"
      )
        SolarSmartphoneRotate2Linear(class="w-5 h-5")

    Teleport(to="body")
      Transition(name="btn-ctx-fade")
        div(
          v-if="hasPersistedWidgetDocument && !isInteractiveMode && selectedPersistedInstance && isWidgetInspectorOpen"
          data-menu="widget-settings"
          class="simulator-widget-inspector btn-ctx-popover"
          :style="widgetInspectorStyle"
          @mousedown.stop
        )
          WidgetInspector(
            :instance="selectedPersistedInstance"
            :definition="definitionForInstance(selectedPersistedInstance)"
            :removable="!isLegacyWidgetInstance(selectedPersistedInstance)"
            :submenu-left="isWidgetInspectorSubmenuLeft"
            @configure="configureSelectedWidget"
            @bind="bindSelectedWidgetPort"
            @unbind="unbindSelectedWidgetPort"
            @remove="removeSelectedWidget"
            @close="isWidgetInspectorOpen = false"
          )

    Teleport(to="body")
      Transition(name="widget-palette")
        div(
          v-if="isWidgetPaletteOpen"
          class="simulator-widget-palette-backdrop"
          @click.self="closeWidgetPalette"
        )
          section(
            class="simulator-widget-palette btn-ctx-popover"
            :style="widgetPaletteStyle"
            role="dialog"
            aria-modal="false"
            aria-labelledby="widget-palette-title"
          )
            header(class="btn-ctx-header")
              span(class="btn-ctx-pill") {{ widgetPaletteTargetCell ? `G${widgetPaletteTargetCell.position}` : '+' }}
              div(class="btn-ctx-title-col")
                span#widget-palette-title(class="btn-ctx-header-title") {{ widgetPaletteTargetCell ? 'Add Widget' : 'Widget Library' }}
                span(class="btn-ctx-header-sub") {{ widgetPaletteTargetCell ? `Row ${widgetPaletteTargetCell.y + 1}, column ${widgetPaletteTargetCell.x + 1}` : 'Choose a widget' }}
              button(type="button" class="simulator-widget-palette__close" aria-label="Close widget palette" @click="closeWidgetPalette") ×
            div(class="simulator-widget-palette__content btn-ctx-body")
              ContextSubmenu(:width="176" menu-class="simulator-widget-palette-floating-menu")
                template(#trigger="{ open, show }")
                  button(type="button" class="btn-ctx-row justify-between w-full group" :class="{ 'btn-ctx-row--open': open }" @click="show")
                    div(class="flex items-center gap-2.5 min-w-0")
                      SolarCursorLinear(class="btn-ctx-icon text-blue-400 group-hover:scale-110 transition-transform")
                      span(class="btn-ctx-label") New Widget
                    span(class="simulator-widget-palette__meta") 2
                    SolarAltArrowRightLinear(class="w-3.5 h-3.5 text-slate-400 shrink-0")
                button(type="button" class="btn-ctx-subrow group" :disabled="!availableLegacySlots.length" @click="createLegacyWidget('click/press')")
                  SolarCursorLinear(class="w-4 h-4 text-blue-400 shrink-0")
                  span(class="btn-ctx-sublabel") Action Button
                  span(class="simulator-widget-palette__meta") 1×1
                button(type="button" class="btn-ctx-subrow group" :disabled="!availableLegacySlots.length" @click="createLegacyWidget('rotary')")
                  MdiTuner(class="w-4 h-4 text-emerald-400 shrink-0")
                  span(class="btn-ctx-sublabel") Rotary Control
                  span(class="simulator-widget-palette__meta") 1×1

              div(v-if="restorableRecentWidgets.length" class="btn-ctx-divider")
              ContextSubmenu(v-if="restorableRecentWidgets.length" :width="176" menu-class="simulator-widget-palette-floating-menu")
                template(#trigger="{ open, show }")
                  button(type="button" class="btn-ctx-row justify-between w-full group" :class="{ 'btn-ctx-row--open': open }" @click="show")
                    div(class="flex items-center gap-2.5 min-w-0")
                      SolarRestartLinear(class="btn-ctx-icon text-cyan-400 group-hover:scale-110 transition-transform")
                      span(class="btn-ctx-label") Recently Removed
                    span(class="simulator-widget-palette__meta") {{ restorableRecentWidgets.length }}
                    SolarAltArrowRightLinear(class="w-3.5 h-3.5 text-slate-400 shrink-0")
                button(
                  v-for="instance in restorableRecentWidgets"
                  :key="`recent:${instance.id}`"
                  type="button"
                  class="btn-ctx-subrow group"
                  @click="restoreRecentWidget(instance)"
                )
                  SolarRestartLinear(class="w-4 h-4 text-cyan-400 shrink-0")
                  span(class="btn-ctx-sublabel simulator-widget-palette__label") {{ String(instance.config.label || definitionForInstance(instance).name) }}
                  span(class="simulator-widget-palette__meta") {{ definitionForInstance(instance).category }}

              div(v-if="removedLegacyButtons.length" class="btn-ctx-divider")
              ContextSubmenu(v-if="removedLegacyButtons.length" :width="176" menu-class="simulator-widget-palette-floating-menu")
                template(#trigger="{ open, show }")
                  button(type="button" class="btn-ctx-row justify-between w-full group" :class="{ 'btn-ctx-row--open': open }" @click="show")
                    div(class="flex items-center gap-2.5 min-w-0")
                      SolarRestartLinear(class="btn-ctx-icon text-amber-400 group-hover:scale-110 transition-transform")
                      span(class="btn-ctx-label") Restore Button
                    span(class="simulator-widget-palette__meta") {{ removedLegacyButtons.length }}
                    SolarAltArrowRightLinear(class="w-3.5 h-3.5 text-slate-400 shrink-0")
                button(
                  v-for="button in removedLegacyButtons"
                  :key="`restore:${button.id}`"
                  type="button"
                  class="btn-ctx-subrow group"
                  @click="restoreLegacyButton(button)"
                )
                  SolarRestartLinear(class="w-4 h-4 text-amber-400 shrink-0")
                  span(class="btn-ctx-sublabel simulator-widget-palette__label") {{ button.label || (button.mode === 'rotary' ? 'Rotary Control' : 'Action Button') }}
                  span(class="simulator-widget-palette__meta") {{ button.mode === 'rotary' ? 'Rotary' : 'Button' }}

              div(v-if="availableWidgetDefinitions.length" class="btn-ctx-divider")
              ContextSubmenu(v-if="availableWidgetDefinitions.length" :width="176" menu-class="simulator-widget-palette-floating-menu")
                template(#trigger="{ open, show }")
                  button(type="button" class="btn-ctx-row justify-between w-full group" :class="{ 'btn-ctx-row--open': open }" @click="show")
                    div(class="flex items-center gap-2.5 min-w-0")
                      SolarSettingsLinear(class="btn-ctx-icon text-purple-400 group-hover:scale-110 transition-transform")
                      span(class="btn-ctx-label") Plugin Widgets
                    span(class="simulator-widget-palette__meta") {{ availableWidgetDefinitions.length }}
                    SolarAltArrowRightLinear(class="w-3.5 h-3.5 text-slate-400 shrink-0")
                button(
                  v-for="entry in availableWidgetDefinitions"
                  :key="entry.definition.id"
                  type="button"
                  class="btn-ctx-subrow group"
                  @click="addWidgetDefinition(entry.definition.id)"
                )
                  SolarSettingsLinear(class="w-4 h-4 text-purple-400 shrink-0")
                  span(class="btn-ctx-sublabel simulator-widget-palette__label") {{ entry.definition.name }}
                  span(class="simulator-widget-palette__meta") {{ entry.definition.defaultSpan.cols }}×{{ entry.definition.defaultSpan.rows }}
              p(
                v-if="!availableLegacySlots.length && !restorableRecentWidgets.length && !removedLegacyButtons.length && !availableWidgetDefinitions.length"
                class="simulator-widget-palette__empty"
              ) No widgets are available.

    Teleport(to="body")
      Transition(name="widget-selection-toolbar")
        div(v-if="selectedWidgetIds.size > 1 && !isInteractiveMode" class="simulator-widget-selection-toolbar")
          strong {{ selectedWidgetIds.size }} selected
          span Drag any selected widget to move the group · right-click for actions

    Teleport(to="body")
      Transition(name="btn-ctx-fade")
        div(
          v-if="isMultiSelectionMenuOpen && selectedWidgetIds.size > 1"
          data-menu="multi-widget-menu"
          class="btn-ctx-popover"
          :style="multiSelectionMenuStyle"
          @mousedown.stop
        )
          div(class="btn-ctx-header")
            span {{ selectedWidgetIds.size }} widgets selected
          button(type="button" class="btn-ctx-row btn-ctx-row--danger group" @click="removeSelectedWidgets")
            SolarTrashBinTrashLinear(class="btn-ctx-icon text-red-400")
            span(class="btn-ctx-label") Remove selected widgets


    div(class="flex justify-center my-auto"  )

        div(class="flex justify-center rounded-2xl overflow-hidden ")
            div#viframe(@wheel="handleZoom" ref="contentDiv" :class="isLandscape? 'aspect-[9/19] portrait': 'aspect-[19/9] landscape'" class="rotatable-container  bg-white rounded-2xl flex "   )
                component(
                  :is="AsyncLayout"
                  :isLandscape="isLandscape"
                  :gridSettings="deckStore.getActiveDeck?.settings"
                  :show-widget-palette="!isInteractiveMode"
                  :square-grid-tracks="true"
                  @open-widget-palette="openWidgetPalette"
                  @grid-pointerdown="startGridMarqueeSelection"
                )
                    template(v-slot:page)
                        div(
                          v-if="marqueeRect"
                          class="simulator-widget-marquee"
                          :style="marqueeRectStyle"
                        )
                        div(
                          v-for="instance in displayedWidgetInstances"
                          :key="instance.id"
                          :data-editor-widget-id="instance.id"
                          class="simulator-widget-cell relative min-h-0 min-w-0 h-full w-full"
                          :class="instance.definitionId === legacyRotaryDefinition.id ? 'simulator-widget-cell--rotary' : ''"
                          :style="persistedWidgetPlacementStyle(instance)"
                          @dragover="allowPersistedWidgetDrop($event, instance)"
                          @drop="dropOnPersistedWidget($event, instance)"
                          @pointerdown="startWidgetPointerEdit($event, instance, 'move')"
                        )
                          WidgetHost(
                            :instance="instance"
                            :definition="definitionForInstance(instance)"
                            :mode="isInteractiveMode ? 'interactive' : 'edit'"
                            :selected="selectedWidgetIds.has(instance.id)"
                            :locked="isPersistedWidgetLocked(instance)"
                            :selection-css="theme.components.clickButton.selectedCss"
                            :runtime-state="runtimeForInstance(instance.id)"
                            :theme="theme"
                            @select="selectPersistedWidget"
                            @open-context-menu="openPersistedWidgetContextMenu($event, instance)"
                            @intent="handleWidgetIntent"
                          )
                            WidgetRenderer(
                              :instance="instance"
                              :definition="definitionForInstance(instance)"
                              :runtime-state="runtimeForInstance(instance.id)"
                              :mode="isInteractiveMode ? 'interactive' : 'edit'"
                              :interactive="isInteractiveMode && !isPersistedWidgetLocked(instance)"
                              :theme="theme"
                              @intent="handleWidgetIntent"
                            )
                            div(
                              v-if="isInteractiveMode && isInstanceRepeating(instance)"
                              class="absolute top-1 right-1 w-3 h-3 bg-green-400 rounded-full shadow-lg border-2 border-white z-10 repeat-indicator"
                              :style="getRepeatIndicatorStyleForInstance(instance)"
                            )
                            template(#overlay)
                              DeckLockOverlay(
                                :position="persistedWidgetButtonPosition(instance)"
                                :is-locked="isPersistedWidgetLocked(instance)"
                                :grid-size="deckStore.getActiveDeck?.settings?.grid || '3,4'"
                                :is-mobile="false"
                                :border-radius="overlayBorderRadius"
                                :is-interactive="isInteractiveMode"
                                @locked-click="buttonLimits.handleLockedButtonClick"
                              )
                          button(
                            v-if="!isInteractiveMode && selectedWidgetIds.size === 1 && selectedWidgetIds.has(instance.id)"
                            v-for="handle in widgetResizeHandles"
                            :key="handle.mode"
                            type="button"
                            :class="['widget-resize-handle', handle.className]"
                            aria-label="Resize widget"
                            @pointerdown.stop.prevent="startWidgetPointerEdit($event, instance, handle.mode)"
                          )

                        div(
                          v-for="cell in persistedVacantCells"
                          :key="`vacant-cell:${editorLayoutKey()}:${cell.position}`"
                          class="simulator-vacant-cell relative h-full w-full min-h-0 min-w-0"
                          :style="{ gridColumn: `${cell.x + 1}`, gridRow: `${cell.y + 1}` }"
                        )
                          DeckLockOverlay(
                            v-if="cell.locked"
                            :position="cell.position"
                            :is-locked="true"
                            :grid-size="deckStore.getActiveDeck?.settings?.grid || '3,4'"
                            :is-mobile="false"
                            :border-radius="overlayBorderRadius"
                            :is-interactive="isInteractiveMode"
                            @locked-click="buttonLimits.handleLockedButtonClick"
                          )
                          button(
                            v-else-if="!isInteractiveMode"
                            type="button"
                            class="simulator-vacant-cell__add"
                            title="Add widget here"
                            :aria-label="`Add widget at row ${cell.y + 1}, column ${cell.x + 1}`"
                            @pointerdown.stop
                            @click.stop="openWidgetPalette($event, cell)"
                            @contextmenu.stop.prevent="openWidgetPalette($event, cell)"
                          )
                            span(aria-hidden="true") +
                //- Floating Button Context Menu
                Transition(name="btn-ctx-fade")
                  div(
                    @mousedown.stop=""
                    v-show="isButtonMenuOpen"
                    :style="optionsMenuPositionStyle"
                    data-menu="button-menu"
                    class="btn-ctx-popover"
                  )
                    // Header
                    div(class="btn-ctx-header")
                      span(class="btn-ctx-pill") G{{editingButton?.position}}
                      div(class="btn-ctx-title-col")
                        span(class="btn-ctx-header-title") Button Options
                        span(class="btn-ctx-header-sub") {{$t(`deck_editor.simulator.executionModes.${editingButton?.execution_mode || 'run_once'}`)}}

                    div(class="btn-ctx-body")
                      // Choose Icon
                      button(
                        type="button"
                        class="btn-ctx-row group"
                        @click="()=>{open(), isButtonMenuOpen = false}"
                      )
                        SolarGalleryWideLinear(class="btn-ctx-icon text-blue-400 group-hover:scale-110 transition-transform")
                        span(class="btn-ctx-label") Choose Icon

                      div(class="relative w-full" @mouseenter="showImageFitDropdown = true" @mouseleave="showImageFitDropdown = false")
                        button(type="button" class="btn-ctx-row justify-between w-full group" :class="{ 'btn-ctx-row--open': showImageFitDropdown }")
                          div(class="flex items-center gap-2.5 min-w-0")
                            SolarGalleryWideLinear(class="btn-ctx-icon text-cyan-400 group-hover:scale-110 transition-transform")
                            span(class="btn-ctx-label") Image Fit
                          span(class="text-[10px] text-slate-400 capitalize") {{ editingButton?.icon_fit ?? 'cover' }}
                          SolarAltArrowRightLinear(class="w-3.5 h-3.5 text-slate-400 group-hover:text-slate-200 transition-colors shrink-0")
                        Transition(name="btn-ctx-sub")
                          div(v-if="showImageFitDropdown" class="btn-ctx-submenu" :class="{ 'btn-ctx-submenu--left': isSubmenuLeft }")
                            button(type="button" class="btn-ctx-subrow group" :class="{ 'btn-ctx-subrow--active': (editingButton?.icon_fit ?? 'cover') === 'cover' }" @click="handleImageFitSet('cover', editingButton?.id)")
                              span(class="btn-ctx-sublabel") Cover
                              svg(v-if="(editingButton?.icon_fit ?? 'cover') === 'cover'" class="w-3.5 h-3.5 text-blue-400 ml-auto shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5")
                                polyline(points="20 6 9 17 4 12")
                            button(type="button" class="btn-ctx-subrow group" :class="{ 'btn-ctx-subrow--active': editingButton?.icon_fit === 'contain' }" @click="handleImageFitSet('contain', editingButton?.id)")
                              span(class="btn-ctx-sublabel") Contain
                              svg(v-if="editingButton?.icon_fit === 'contain'" class="w-3.5 h-3.5 text-blue-400 ml-auto shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5")
                                polyline(points="20 6 9 17 4 12")

                      // Reset Button (danger)
                      button(
                        type="button"
                        class="btn-ctx-row btn-ctx-row--danger group"
                        @click="resetButton"
                      )
                        SolarRestartLinear(class="btn-ctx-icon text-red-400 group-hover:scale-110 transition-transform")
                        span(class="btn-ctx-label") Reset Button

                      button(
                        type="button"
                        class="btn-ctx-row btn-ctx-row--danger group"
                        @click="removeEditingButtonFromLayout"
                      )
                        SolarTrashBinTrashLinear(class="btn-ctx-icon text-red-400 group-hover:scale-110 transition-transform")
                        span(class="btn-ctx-label") Remove widget

                      div(class="btn-ctx-divider")

                      // Change Mode Dropdown
                      div(
                        class="relative w-full"
                        @mouseenter="showModeDropdown = true"
                        @mouseleave="showModeDropdown = false"
                      )
                        button(type="button" class="btn-ctx-row justify-between w-full group" :class="{ 'btn-ctx-row--open': showModeDropdown }")
                          div(class="flex items-center gap-2.5 min-w-0")
                            SolarSettingsLinear(class="btn-ctx-icon text-purple-400 group-hover:scale-110 transition-transform")
                            span(class="btn-ctx-label") Change Mode
                          SolarAltArrowRightLinear(class="w-3.5 h-3.5 text-slate-400 group-hover:text-slate-200 transition-colors shrink-0")

                        // Mode Submenu
                        Transition(name="btn-ctx-sub")
                          div(
                            v-if="showModeDropdown"
                            class="btn-ctx-submenu"
                            :class="{ 'btn-ctx-submenu--left': isSubmenuLeft }"
                          )
                            button(
                              type="button"
                              class="btn-ctx-subrow group"
                              :class="{ 'btn-ctx-subrow--active': editingButton?.mode === null || editingButton?.mode === 'click/press' }"
                              @click="handleModeSet('click/press', editingButton?.id)"
                            )
                              SolarCursorLinear(class="w-4 h-4 text-blue-400 shrink-0")
                              span(class="btn-ctx-sublabel") Click / Press
                              svg(v-if="editingButton?.mode === null || editingButton?.mode === 'click/press'" class="w-3.5 h-3.5 text-blue-400 ml-auto shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5")
                                polyline(points="20 6 9 17 4 12")

                            button(
                              type="button"
                              class="btn-ctx-subrow group"
                              :class="{ 'btn-ctx-subrow--active': editingButton?.mode === 'rotary' }"
                              @click="handleModeSet('rotary', editingButton?.id)"
                            )
                              MdiTuner(class="w-4 h-4 text-emerald-400 shrink-0")
                              span(class="btn-ctx-sublabel") Rotary
                              svg(v-if="editingButton?.mode === 'rotary'" class="w-3.5 h-3.5 text-emerald-400 ml-auto shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5")
                                polyline(points="20 6 9 17 4 12")

                      // Execution Mode selector (only for click/press buttons)
                      div(
                        class="relative w-full"
                        v-if="editingButton?.mode === null || editingButton?.mode === 'click/press'"
                        @mouseenter="showExecutionDropdown = true"
                        @mouseleave="showExecutionDropdown = false"
                      )
                        button(type="button" class="btn-ctx-row justify-between w-full group" :class="{ 'btn-ctx-row--open': showExecutionDropdown }")
                          div(class="flex items-center gap-2.5 min-w-0")
                            SolarBoltLinear(class="btn-ctx-icon text-amber-400 group-hover:scale-110 transition-transform")
                            span(class="btn-ctx-label") Behavior
                          SolarAltArrowRightLinear(class="w-3.5 h-3.5 text-slate-400 group-hover:text-slate-200 transition-colors shrink-0")

                        // Behavior Submenu
                        Transition(name="btn-ctx-sub")
                          div(
                            v-if="showExecutionDropdown"
                            class="btn-ctx-submenu"
                            :class="{ 'btn-ctx-submenu--left': isSubmenuLeft }"
                          )
                            button(
                              type="button"
                              class="btn-ctx-subrow group"
                              :class="{ 'btn-ctx-subrow--active': (editingButton?.execution_mode || 'run_once') === 'run_once' }"
                              @click="handleExecutionModeSet('run_once', editingButton?.id)"
                            )
                              SolarPlayLinear(class="w-4 h-4 text-blue-400 shrink-0")
                              span(class="btn-ctx-sublabel") Run Once
                              svg(v-if="(editingButton?.execution_mode || 'run_once') === 'run_once'" class="w-3.5 h-3.5 text-blue-400 ml-auto shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5")
                                polyline(points="20 6 9 17 4 12")

                            button(
                              type="button"
                              class="btn-ctx-subrow group"
                              :class="{ 'btn-ctx-subrow--active': editingButton?.execution_mode === 'repeat_while_pressing' }"
                              @click="handleExecutionModeSet('repeat_while_pressing', editingButton?.id)"
                            )
                              MdiRepeat(class="w-4 h-4 text-emerald-400 shrink-0")
                              span(class="btn-ctx-sublabel") Hold to Repeat
                              svg(v-if="editingButton?.execution_mode === 'repeat_while_pressing'" class="w-3.5 h-3.5 text-emerald-400 ml-auto shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5")
                                polyline(points="20 6 9 17 4 12")

                            button(
                              type="button"
                              class="btn-ctx-subrow group"
                              :class="{ 'btn-ctx-subrow--active': editingButton?.execution_mode === 'switch' }"
                              @click="handleExecutionModeSet('switch', editingButton?.id)"
                            )
                              MdiToggleSwitch(class="w-4 h-4 text-purple-400 shrink-0")
                              span(class="btn-ctx-sublabel") Toggle
                              svg(v-if="editingButton?.execution_mode === 'switch'" class="w-3.5 h-3.5 text-purple-400 ml-auto shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5")
                                polyline(points="20 6 9 17 4 12")

                            button(
                              type="button"
                              class="btn-ctx-subrow group"
                              :class="{ 'btn-ctx-subrow--active': editingButton?.execution_mode === 'queue' }"
                              @click="handleExecutionModeSet('queue', editingButton?.id)"
                            )
                              MdiPlaylistPlay(class="w-4 h-4 text-amber-400 shrink-0")
                              span(class="btn-ctx-sublabel") Queue
                              svg(v-if="editingButton?.execution_mode === 'queue'" class="w-3.5 h-3.5 text-amber-400 ml-auto shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5")
                                polyline(points="20 6 9 17 4 12")

                      // Repeat Configuration Button
                      button(
                        v-if="editingButton?.execution_mode === 'repeat_while_pressing' && (editingButton?.mode === null || editingButton?.mode === 'click/press')"
                        type="button"
                        class="btn-ctx-row group"
                        @click="openRepeatConfigModal"
                      )
                        SolarClockCircleLinear(class="btn-ctx-icon text-emerald-400 group-hover:scale-110 transition-transform")
                        span(class="btn-ctx-label") Repeat Settings

                      // Rotary Set Value Dropdown
                      div(
                        class="relative w-full"
                        v-if="editingButton?.mode === 'rotary'"
                        @mouseenter="showValueDropdown = true"
                        @mouseleave="showValueDropdown = false"
                      )
                        button(type="button" class="btn-ctx-row justify-between w-full group" :class="{ 'btn-ctx-row--open': showValueDropdown }")
                          div(class="flex items-center gap-2.5 min-w-0")
                            MdiTuner(class="btn-ctx-icon text-purple-400 group-hover:scale-110 transition-transform")
                            span(class="btn-ctx-label") Set Value
                          SolarAltArrowRightLinear(class="w-3.5 h-3.5 text-slate-400 group-hover:text-slate-200 transition-colors shrink-0")

                        Transition(name="btn-ctx-sub")
                          div(
                            v-if="showValueDropdown"
                            class="btn-ctx-submenu p-3 text-center"
                            :class="{ 'btn-ctx-submenu--left': isSubmenuLeft }"
                          )
                            div(class="flex flex-col items-center justify-center p-2")
                              MdiTuner(class="w-7 h-7 mb-1.5 text-slate-400")
                              p(class="text-xs font-bold text-white") Coming soon
                              p(class="text-[11px] text-slate-400 mt-1 leading-tight") Use the left panel to drag and drop a value

                // Drop Macro / Action Trigger Menu
                Transition(name="btn-ctx-fade")
                  div(
                    @mousedown.stop=""
                    v-show="isDropMenuOpen"
                    :style="dropMenuPositionStyle"
                    data-menu="drop-menu"
                    class="btn-ctx-dropmenu"
                  )
                    div(class="btn-ctx-dropmenu-header")
                      span(class="text-[11px] font-bold text-slate-300 uppercase tracking-wider") Trigger Mode
                    button(
                      type="button"
                      class="btn-ctx-row w-full group"
                      @click="() => handleMacroTrigger('click')"
                    )
                      SolarCursorLinear(class="w-4 h-4 text-blue-400 shrink-0 group-hover:scale-110 transition-transform")
                      span(class="btn-ctx-label") On Click
                  //- button(
                  //-     disabled
                  //-     class="w-full py-2 px-2 line-through "
                  //-     @click="() => handleMacroTrigger('press')"
                  //- ) press

        div(class="absolute flex flex-col items-center gap-x-6 bottom-0 left-0 w-full   text-white")
          div(class="flex flex-wrap gap-x-2 items-center justify-start w-full px-2 ")
              button(@click="menu = 'deck-settings'" :class="menu === 'deck-settings'? 'bg-[#1d1d1d] text-white': 'bg-[#2d2d2d] hover:bg-[#2d2d2d] hover:text-white '" class="flex items-center  px-4 py-1 text-sm font-medium text-gray-300 bg-[#1d1d1d] rounded-t-lg  hover:text-white transition-colors ")
                  span Deck Settings
              button(@click="menu = 'buttons-settings'" :class="menu === 'buttons-settings'? 'bg-[#1d1d1d] text-white': 'bg-[#2d2d2d] hover:text-white  '" class="flex items-center  px-4 py-1 text-sm font-medium text-gray-300 bg-[#1d1d1d] rounded-t-lg  transition-colors ")
                  span Buttons Settings
              // save
              button(@click="saveDeckSettings" class="ml-auto flex items-center  px-4 py-1 text-sm font-medium text-gray-300 bg-blue-600 hover:bg-blue-500 rounded-t-lg  transition-colors ")
                  span Save


          div(class="flex items-center w-full px-3 h-[4.3rem] overflow-hidden bg-[#1d1d1d] text-white" )
            Transition(name="menu" mode="out-in")
              DeckEditorSimulatorSettingsMenu(v-if="menu === 'deck-settings'")
                template(v-slot:content)
                  UtilsSlider(label="Buttons Spacing" :modelValue="deckStore.getActiveDeck.settings.button_spacing"  @update="deckStore.getActiveDeck.settings.button_spacing = $event")
                  UtilsTwoValues(:modelValue="deckStore.getActiveDeck.settings.grid" @update="updateGridSize" label="Grid Size")
                  div(class="flex flex-nowrap items-end gap-x-2 shrink-0")
                    UtilsSlideBox(label="Adaptive" @update="toggleOrientationSync" :modelValue="orientationSyncEnabled" title="Mirror edits into the other screen orientation")
                    UtilsSlideBox(label="Maximize" @update="deckStore.toggleMaximizeButtonSpace" :modelValue="deckStore.getActiveDeck.settings.maximize_button_space" title="Grow the deck grid to the maximum available space")
                    button(
                      type="button"
                      class="h-7 px-2 rounded-md border border-blue-500/60 bg-blue-500/10 hover:bg-blue-500/20 text-[10px] font-medium whitespace-nowrap disabled:opacity-40 disabled:cursor-not-allowed"
                      :disabled="!hasPersistedWidgetDocument"
                      title="Copy this orientation's layout into the other orientation once"
                      @click="syncCurrentLayoutToOther"
                    ) Sync → other

            Transition(name="menu" mode="out-in")
              DeckEditorSimulatorSettingsMenu(v-if="menu === 'buttons-settings'")
                template(v-slot:content)
                  UtilsSlider(label="Padding" :modelValue="deckStore.getActiveDeck.settings.buttons_padding" @update="deckStore.getActiveDeck.settings.buttons_padding = $event")

            //- div(:class="isLandscape? 'rotate-90': '	 '" class="aspect-[9/16] h-[calc(100vh-12rem)] rotatable-container  bg-white rounded-2xl")
            //-     component(:is="AsyncComp")

    // Value Selection Modal
    BaseModal(:is-open="isChooseValueModalOpen" :z-index="12070" @close="isChooseValueModalOpen = false")
      div(class="vd-modal-card p-6 w-80")
        p(class="text-lg font-bold mb-4 text-center") Choose a value for action {{editingButton?.action}}
        div(class="mb-4")
          input(
            type="range"
            v-model.number="selectedValue"
            min="0"
            max="100"
            step="0.1"
            class="w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer"
          )
          div(class="flex justify-between mt-2")
            span(class="text-xs text-gray-500") 0
            span(class="text-lg font-semibold") {{ selectedValue.toFixed(1) }}
            span(class="text-xs text-gray-500") 100
        div(class="flex justify-end space-x-2")
          button(
            class="px-4 py-2 rounded-lg text-gray-600 hover:bg-gray-200 transition-colors"
            @click="isChooseValueModalOpen = false"
          ) Cancel
          button(
            class="bg-blue-500 text-white px-4 py-2 rounded-lg hover:bg-blue-600 transition-colors"
            @click="confirmValueSelection"
          ) Confirm

    // Repeat Configuration Modal
    RepeatConfigModal(
      :is-open="isRepeatConfigModalOpen"
      :button-position="editingButton?.position"
      :config="repeatConfig"
      @close="isRepeatConfigModalOpen = false"
      @apply="handleRepeatConfigApply"
    )
</template>

<style scoped>
/* Repeat indicator pulse animation */
@keyframes pulse {
  0% {
    transform: scale(1);
    opacity: 1;
  }
  50% {
    transform: scale(1.2);
    opacity: 0.8;
  }
  100% {
    transform: scale(1);
    opacity: 1;
  }
}

.repeat-indicator {
  animation: pulse var(--animation-duration, 100ms) infinite;
}

/* Button blocked flash animation */
@keyframes button-blocked-flash {
  0% {
    filter: brightness(1);
    transform: scale(1);
  }
  25% {
    filter: brightness(1.5) saturate(1.5);
    transform: scale(1.05);
  }
  50% {
    filter: brightness(0.5) saturate(0.5);
    transform: scale(0.95);
  }
  75% {
    filter: brightness(1.5) saturate(1.5);
    transform: scale(1.05);
  }
  100% {
    filter: brightness(1);
    transform: scale(1);
  }
}

.button-blocked-flash {
  animation: button-blocked-flash 0.5s ease-in-out;
}

.simulator-rotate-btn {
  background: linear-gradient(
    100deg,
    hsl(var(--editor-accent, 20 96% 55%) / 1),
    hsl(var(--editor-accent-2, 20 95% 62%) / 1)
  );
}
</style>

<script setup lang="ts">
definePageMeta({
  layout: "simulator",
});

import MdiTuner from "~icons/mdi/tuner";
import MdiRepeat from "~icons/mdi/repeat?width=16px&height=16px";
import MdiToggleSwitch from "~icons/mdi/toggle-switch?width=16px&height=16px";
import MdiPlaylistPlay from "~icons/mdi/playlist-play?width=16px&height=16px";
import SolarGalleryWideLinear from "~icons/solar/gallery-wide-linear?width=20px&height=20px";
import SolarRestartLinear from "~icons/solar/restart-linear?width=20px&height=20px";
import SolarTrashBinTrashLinear from "~icons/solar/trash-bin-trash-linear?width=20px&height=20px";
import SolarCursorLinear from "~icons/solar/cursor-linear?width=20px&height=20px";
import SolarBoltLinear from "~icons/solar/bolt-linear?width=20px&height=20px";
import SolarClockCircleLinear from "~icons/solar/clock-circle-linear?width=20px&height=20px";
import SolarAltArrowRightLinear from "~icons/solar/alt-arrow-right-linear?width=16px&height=16px";
import SolarPlayLinear from "~icons/solar/play-linear?width=16px&height=16px";
import SolarSettingsLinear from "~icons/solar/settings-linear?width=20px&height=20px";
import RepeatConfigModal from "@/components/Modal/RepeatConfigModal.vue";
import BaseModal from "../../components/Modal/BaseModal.vue";

import SolarSmartphoneRotate2Linear from "~icons/solar/smartphone-rotate-2-linear?width=24px&height=24px";
import {
  getAllWebviewWindows,
  getCurrentWebviewWindow,
} from "@tauri-apps/api/webviewWindow";
import { getCurrentWindow, PhysicalSize } from "@tauri-apps/api/window";
import {
  emitTo,
  listen,
  type Event as TauriEvent,
} from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { useDeckStore } from "@/stores/Deck";
import { useFileDialog } from "@vueuse/core";
import { useDesktopUtils } from "@desktop/stores/desktopUtils";
import { useMainStore } from "@/stores/Main";
import { createSwapy, type Swapy } from "swapy";
import type { ActionReference } from "@base/types/actionReference";
import type { Button, ButtonIconFit, Deck } from "@base/types/deck";
import type { DeckTheme } from "@base/stores/Deck";
import WidgetHost from "@base/components/Deck/Widget/Host.vue";
import WidgetRenderer from "@base/components/Deck/Widget/Renderer.vue";
import WidgetInspector from "@desktop/components/DeckEditor/WidgetInspector.vue";
import ContextSubmenu from "@desktop/components/ContextSubmenu.vue";
import {
  useDeckWidgetDocument,
  type DeckWidgetDocumentSnapshot,
} from "@base/composables/useDeckWidgetDocument";
import {
  useWidgetInteractionRouter,
  type WidgetDispatchOperation,
} from "@base/composables/useWidgetInteractionRouter";
import { useWidgetRegistry } from "@base/composables/useWidgetRegistry";
import { useInstalledDeckWidgets } from "@base/composables/useInstalledDeckWidgets";
import { useWidgetRuntimeState } from "@base/composables/useWidgetRuntimeState";
import { createDeckWidgetEditor } from "@base/utils/deckWidgetEditor";
import {
  canPlaceWidget,
  firstRestrictedCellPosition,
  moveWidgetsWithReflow,
  placeWidgetWithReflow,
} from "@base/utils/widgetLayout";
import { deriveResponsiveWidgetLayout } from "@base/utils/responsiveWidgetLayout";
import type {
  DeckWidgetInstance,
  WidgetBinding,
  WidgetDefinition,
  WidgetIntent,
  WidgetPlacement,
  WidgetRuntimeState,
} from "@base/types/deckWidget";
import type {
  DeckWidgetIntentAckMessage,
  DeckWidgetStateMessage,
} from "@base/types/deckWidgetProtocol";
import {
  legacyActionButtonDefinition,
  legacyButtonToWidget,
  reconcileLegacyWidgetInstance,
  legacyWidgetPlacementForLayout,
  legacyRotaryDefinition,
  isVacantLegacyButton,
} from "@base/utils/legacyDeckWidget";

import {
  onMounted,
  onUnmounted,
  computed,
  watch,
  ref,
  nextTick,
  type Ref,
} from "vue";

interface ServerInfo {
  ip: string;
  port: number;
  iconsServerPort: number;
}

interface SystemVolumePayload {
  volume: number;
  is_button_change: boolean;
  button_id: string | null;
}

interface DeckEventPayload {
  deck: Deck;
}

interface ThemeEventPayload {
  theme: DeckTheme;
}

interface SocketInjection {
  $socket: Ref<WebSocket | null>;
}

type SimulatorButton = Omit<Button, "id"> & { id: number | string };

const parseGrid = (grid?: string): [number, number] => {
  const [parsedCols, parsedRows] = (grid ?? "3,4").split(",").map(Number);
  const cols = Number.isFinite(parsedCols) && parsedCols! > 0 ? parsedCols! : 3;
  const rows = Number.isFinite(parsedRows) && parsedRows! > 0 ? parsedRows! : 4;
  return [cols, rows];
};

const isInteractiveMode = ref(false);
const selectedValue = ref(0);

// Button limits for lock overlays
const buttonLimits = useButtonLimits(false); // false = desktop, not mobile

const toggleInteractiveMode = () => {
  isInteractiveMode.value = !isInteractiveMode.value;
  swapy?.enable(false);
  swapy?.update();
};

// const handleValueChange = async (value: number) => {
//   alert("test")
//   await invoke("set_volume", { volume: value })
// }

const theme = computed(() => {
  return deckStore.getActiveTheme;
});

// Extract border-radius from theme buttonCss to match overlay shape
const overlayBorderRadius = computed(() => {
  try {
    const css = theme.value?.components?.clickButton?.buttonCss || "";

    // Extract border-radius from inline CSS if present
    const match = css.match(/border-radius:\s*([^;]+);?/);
    if (match) {
      return match[1].trim();
    }

    // Default to 0.75rem (rounded-xl) which matches the Dynamic Button's Tailwind class
    // Note: Dynamic Button uses "rounded-xl" Tailwind class when theme.buttonCss is empty
    return "0.75rem"; // rounded-xl = 0.75rem (12px)
  } catch (e) {
    return "0.75rem";
  }
});

// Removed old hardcoded limits - now using useButtonLimits composable

const menu: Ref<"deck-settings" | "buttons-settings"> = ref("deck-settings");
const desktopUtils = useDesktopUtils();
const deckStore = useDeckStore();
const mainStore = useMainStore();
const widgetRegistry = useWidgetRegistry();
const installedDeckWidgets = useInstalledDeckWidgets();
const widgetDocuments = useDeckWidgetDocument();
const widgetRuntime = useWidgetRuntimeState();
const latestSystemVolume = ref<number | null>(null);
const persistedWidgetSnapshot = ref<DeckWidgetDocumentSnapshot>({
  deckId: 0,
  revision: 0,
  updatedAt: "",
  instances: [],
  legacyBacked: true,
});
const selectedWidgetId = ref<string | null>(null);
const selectedWidgetIds = ref<Set<string>>(new Set());
const isWidgetInspectorOpen = ref(false);
const widgetInspectorPosition = ref({ top: 0, left: 0, width: 0, height: 0 });
const isWidgetPaletteOpen = ref(false);
const recentlyRemovedWidgets = ref<DeckWidgetInstance[]>([]);
type WidgetPaletteTargetCell = { x: number; y: number; position: number };
const widgetPaletteTargetCell = ref<WidgetPaletteTargetCell | null>(null);
const widgetPaletteAnchor = ref<{
  top: number;
  left: number;
  right: number;
  bottom: number;
} | null>(null);
const widgetPaletteStyle = computed(() => {
  const anchor = widgetPaletteAnchor.value;
  if (!anchor || typeof window === "undefined") {
    return { left: "50%", top: "50%", transform: "translate(-50%, -50%)" };
  }
  const width = Math.min(216, window.innerWidth - 16);
  const estimatedHeight = Math.min(240, window.innerHeight - 16);
  const left = Math.max(
    8,
    Math.min(anchor.left, window.innerWidth - width - 8),
  );
  const below = anchor.bottom + 8;
  const top =
    below + estimatedHeight <= window.innerHeight
      ? below
      : Math.max(8, anchor.top - estimatedHeight - 8);
  return { left: `${left}px`, top: `${top}px`, width: `${width}px` };
});
const closeWidgetPalette = () => {
  isWidgetPaletteOpen.value = false;
  widgetPaletteTargetCell.value = null;
  widgetPaletteAnchor.value = null;
};
const openWidgetPalette = (
  event?: MouseEvent,
  cell?: WidgetPaletteTargetCell,
) => {
  if (isInteractiveMode.value) return;
  const target = event?.currentTarget;
  const rect =
    target instanceof HTMLElement ? target.getBoundingClientRect() : null;
  widgetPaletteTargetCell.value = cell ? { ...cell } : null;
  widgetPaletteAnchor.value = rect
    ? { top: rect.top, left: rect.left, right: rect.right, bottom: rect.bottom }
    : null;
  isWidgetPaletteOpen.value = true;
};
const forceWidgetLayoutEditor = ref(false);
const widgetEditor = ref(createDeckWidgetEditor([], 0));
const { $socket } = useNuxtApp() as unknown as SocketInjection;
const groupButtons = ref(false);
const orientationSyncEnabled = computed(() =>
  Boolean(deckStore.getActiveDeck?.settings.orientation_sync_enabled),
);
const toggleOrientationSync = () => {
  const settings = deckStore.getActiveDeck?.settings;
  if (settings)
    settings.orientation_sync_enabled = !settings.orientation_sync_enabled;
};

const { files, open, reset, onCancel, onChange } = useFileDialog({
  accept: "image/*", // Set to accept only image files
  // Select directories instead of files if set true
});
const imageData = ref<string | null>(null); // Store Base64 image data
let swapy: Swapy | null = null;

const updateGridSize = (value: string) => {
  // deckStore.getActiveDeck.settings.grid = value;
  console.log(value);
};

const saveDeckSettings = async () => {
  const activeDeck = deckStore.getActiveDeck;
  if (!activeDeck) return;

  const status = await invoke<[string, number, Deck | null]>(
    "update_deck_settings",
    {
      deckId: activeDeck.id,
      deckSettings: activeDeck.settings,
    },
  );
  if (status[1] === 200) {
    console.log("Deck settings updated.");
    if (status[2]) {
      //buttons generated, updating getck
      console.log(status[2]);
      deckStore.updateActiveDeck(status[2]);
      swapy?.update();
    }
  } else {
    console.log("Failed to update deck settings.");
  }
};

const handleZoom = (event: WheelEvent) => {
  if (event.ctrlKey) {
    const activeDeck = deckStore.getActiveDeck;
    if (!activeDeck) return;

    // Get current grid dimensions
    const [cols, rows] = parseGrid(activeDeck.settings.grid);
    const delta = event.deltaY > 0 ? 1 : -1; // Convert scroll direction to +/-1 (reversed)

    // Determine if landscape or portrait based on current ratio
    const isLandscape = cols > rows;

    // Set max/min dimensions based on orientation
    const maxDims: [number, number] = isLandscape ? [11, 9] : [9, 11];
    const minDims: [number, number] = [1, 3];

    // Calculate new dimensions while respecting limits
    const newGrid: [number, number] = [
      Math.min(Math.max(cols + delta, minDims[0]), maxDims[0]),
      Math.min(Math.max(rows + delta, minDims[1]), maxDims[1]),
    ];

    // Only update if within bounds
    if (
      newGrid[0] >= minDims[0] &&
      newGrid[0] <= maxDims[0] &&
      newGrid[1] >= minDims[1] &&
      newGrid[1] <= maxDims[1]
    ) {
      activeDeck.settings.grid = newGrid.join(",");
    }
    swapy?.update();
  }
};

interface MenuPosition {
  top: number;
  left: number;
  width: number;
  height: number;
}

// Menu state
const isButtonMenuOpen = ref(false);
const isDropMenuOpen = ref(false);
const isMultiSelectionMenuOpen = ref(false);
const showModeDropdown = ref(false);
const showExecutionDropdown = ref(false);
const showValueDropdown = ref(false);
const showImageFitDropdown = ref(false);
const lastToggledButton = ref<number | null>(null);
const buttonMenuPosition = ref<MenuPosition>({
  top: 0,
  left: 0,
  width: 0,
  height: 0,
});
const dropMenuPosition = ref<MenuPosition>({
  top: 0,
  left: 0,
  width: 0,
  height: 0,
});
const multiSelectionMenuPosition = ref<MenuPosition>({
  top: 0,
  left: 0,
  width: 0,
  height: 0,
});

const multiSelectionMenuStyle = computed(() => {
  const viewportWidth =
    typeof window === "undefined" ? 1200 : window.innerWidth;
  const viewportHeight =
    typeof window === "undefined" ? 800 : window.innerHeight;
  const menuWidth = 216;
  const menuHeight = 84;
  const anchor = multiSelectionMenuPosition.value;
  const left = Math.max(
    8,
    Math.min(anchor.left, viewportWidth - menuWidth - 8),
  );
  const below = anchor.top + anchor.height + 6;
  const top =
    below + menuHeight <= viewportHeight - 8
      ? below
      : Math.max(8, anchor.top - menuHeight - 6);
  return {
    position: "fixed" as const,
    top: `${top}px`,
    left: `${left}px`,
    width: `${menuWidth}px`,
    zIndex: 2000,
  };
});

const widgetInspectorStyle = computed(() => {
  const viewportWidth =
    typeof window === "undefined" ? 1200 : window.innerWidth;
  const viewportHeight =
    typeof window === "undefined" ? 800 : window.innerHeight;
  const menuWidth = Math.min(216, viewportWidth - 16);
  const estimatedHeight = 176;
  const anchor = widgetInspectorPosition.value;
  const left = Math.max(
    8,
    Math.min(anchor.left, viewportWidth - menuWidth - 8),
  );
  const below = anchor.top + anchor.height + 6;
  const top =
    below + estimatedHeight <= viewportHeight - 8
      ? below
      : Math.max(8, anchor.top - estimatedHeight - 6);
  return {
    position: "fixed" as const,
    top: `${top}px`,
    left: `${left}px`,
    width: `${menuWidth}px`,
    zIndex: 2000,
  };
});

const isWidgetInspectorSubmenuLeft = computed(() => {
  if (typeof window === "undefined") return false;
  const menuWidth = Math.min(216, window.innerWidth - 16);
  const submenuWidth = 248;
  const anchor = widgetInspectorPosition.value;
  const left = Math.max(
    8,
    Math.min(anchor.left, window.innerWidth - menuWidth - 8),
  );
  return left + menuWidth + submenuWidth + 12 > window.innerWidth;
});

// Smart submenu direction calculation
const isSubmenuLeft = computed(() => {
  const windowWidth = typeof window !== "undefined" ? window.innerWidth : 1200;
  const menuWidth = 216; // 13.5rem
  const submenuWidth = 180; // 11.25rem
  const { left, width } = buttonMenuPosition.value;
  const buttonCenter = left + width / 2;

  // If button is on the right half of the screen or close to right edge, open submenu to the left
  if (
    buttonCenter > windowWidth / 2 ||
    left + menuWidth + submenuWidth + 24 > windowWidth
  ) {
    return true;
  }
  return false;
});

// Computed menu positions with viewport boundary clamping
const optionsMenuPositionStyle = computed(() => {
  const windowWidth = typeof window !== "undefined" ? window.innerWidth : 1200;
  const windowHeight = typeof window !== "undefined" ? window.innerHeight : 800;
  const menuWidth = 216; // 13.5rem
  const submenuWidth = 180; // 11.25rem
  const menuHeight = 308; // Includes the image-fit row

  const { top, left, width, height } = buttonMenuPosition.value;

  // 1. Calculate horizontal position ensuring both main menu & submenu fit on screen
  let calculatedLeft = left;

  if (isSubmenuLeft.value) {
    // Submenu opens to the LEFT of the main menu.
    // Ensure main menu is far enough right that: calculatedLeft - submenuWidth >= 16
    const minLeft = submenuWidth + 16;
    const maxLeft = Math.max(minLeft, windowWidth - menuWidth - 16);
    calculatedLeft = Math.max(
      minLeft,
      Math.min(left + width - menuWidth, maxLeft),
    );
  } else {
    // Submenu opens to the RIGHT of the main menu.
    // Ensure main menu is far enough left that: calculatedLeft + menuWidth + submenuWidth <= windowWidth - 16
    const minLeft = 16;
    const maxLeft = Math.max(
      minLeft,
      windowWidth - menuWidth - submenuWidth - 16,
    );
    calculatedLeft = Math.max(minLeft, Math.min(left, maxLeft));
  }

  // 2. Calculate vertical position
  let calculatedTop = top + height + 6;
  if (calculatedTop + menuHeight > windowHeight - 16) {
    // Position menu above button if overflowing bottom
    calculatedTop = Math.max(16, top - menuHeight - 6);
  }

  return {
    position: "fixed" as const,
    width: `${menuWidth}px`,
    top: `${calculatedTop}px`,
    left: `${calculatedLeft}px`,
    zIndex: 1000,
  };
});

const dropMenuPositionStyle = computed(() => ({
  position: "fixed",
  top: `${dropMenuPosition.value.top + dropMenuPosition.value.height}px`,
  left: `${dropMenuPosition.value.left + 50}px`,
  width: `${dropMenuPosition.value.width}px`,
}));

// Menu handlers
const resetMenus = () => {
  isButtonMenuOpen.value = false;
  isDropMenuOpen.value = false;
  isMultiSelectionMenuOpen.value = false;
  isWidgetInspectorOpen.value = false;
  // Don't reset lastToggledButton here to preserve editingButton reference
  // lastToggledButton.value = null;
};

const toggleButtonMenu = (bounds: MenuPosition, buttonId: number) => {
  buttonMenuPosition.value = bounds;

  if (lastToggledButton.value === buttonId) {
    isButtonMenuOpen.value = !isButtonMenuOpen.value;
  } else {
    isButtonMenuOpen.value = true;
  }

  lastToggledButton.value = buttonId;
  isDropMenuOpen.value = false; // Close drop menu when opening button menu
};

const selectButton = async (button: SimulatorButton) => {
  selectedButton.value = button;
  await emitTo("main", "ms_select_button", button.id);
  if (isInteractiveMode.value || typeof button.id !== "number") return;
  if (!hasPersistedWidgetDocument.value) await upgradeActiveDeckToWidgets();
  forceWidgetLayoutEditor.value = true;
  const instanceId = `button:${button.id}`;
  await pruneVacantLegacyWidgets(new Set([instanceId]));
  if (widgetEditor.value.instances().some(({ id }) => id === instanceId)) {
    setWidgetSelection([instanceId], instanceId);
  }
};

type ComponentMode = "rotary" | "click/press";

interface ActionData {
  buttonId: number;
  reference: import("@base/types/actionReference").ActionReference;
  label?: string;
}

interface ModeData {
  buttonId: number;
  mode: ComponentMode;
}

type DropEvent =
  | { event: "action-reference"; data: ActionData }
  | { event: "mode"; data: ModeData };

const isActionReferenceKind = (
  value: unknown,
): value is ActionData["reference"]["kind"] =>
  typeof value === "string" &&
  [
    "macro",
    "system_control",
    "program",
    "saved_action",
    "flow",
    "http_request",
    "focus_window",
  ].includes(value);

// Store drop data temporarily desktop/pages/deck/index.vue macros or layers/base/components/Submenu.vue
const pendingDropData = ref<DropEvent | null>(null);

// dropped from the element in:
const onDrop = async (
  event: DragEvent,
  buttonId: number,
  buttonMode: ComponentMode | null,
) => {
  if (!event.dataTransfer) return;

  try {
    const rawData =
      event.dataTransfer.getData("application/json") ||
      event.dataTransfer.getData("text/plain");
    if (!rawData) return;
    const data: unknown = JSON.parse(rawData);
    if (
      !data ||
      typeof data !== "object" ||
      !("event" in data) ||
      !("data" in data)
    )
      return;
    const dropData = data.data;
    if (!dropData || typeof dropData !== "object") return;

    // Get button element and its position
    const currentTarget =
      event.currentTarget instanceof HTMLElement ? event.currentTarget : null;
    const eventTarget =
      event.target instanceof HTMLElement ? event.target : null;
    const buttonElement =
      currentTarget ??
      eventTarget?.closest<HTMLElement>(
        "[data-swapy-item], .simulator-widget-cell",
      );
    if (!buttonElement) return;

    const bounds = buttonElement.getBoundingClientRect();
    dropMenuPosition.value = {
      top: bounds.top,
      left: bounds.left,
      width: bounds.width,
      height: bounds.height,
    };

    isButtonMenuOpen.value = false;

    // Handle different event types
    const reference = "reference" in dropData ? dropData.reference : null;
    if (
      data.event === "action-reference" &&
      reference &&
      typeof reference === "object" &&
      "kind" in reference &&
      isActionReferenceKind(reference.kind) &&
      "id" in reference &&
      typeof reference.id === "string"
    ) {
      pendingDropData.value = {
        event: "action-reference",
        data: {
          buttonId,
          reference: { kind: reference.kind, id: reference.id },
          label:
            "label" in dropData && typeof dropData.label === "string"
              ? dropData.label
              : undefined,
        },
      };

      handleMacroTrigger(buttonMode ?? "click/press");
    } else if (
      data.event === "mode" &&
      "mode" in dropData &&
      (dropData.mode === "rotary" || dropData.mode === "click/press")
    ) {
      pendingDropData.value = {
        event: "mode",
        data: {
          buttonId,
          mode: dropData.mode,
        },
      };
      handleModeSet(dropData.mode, buttonId);
    }
  } catch (error) {
    console.error("Error parsing drop data:", error);
  }
};

const persistReconciledLegacyWidget = async (button: Button) => {
  if (!hasPersistedWidgetDocument.value) return;
  const deckId = deckStore.getActiveDeck?.id;
  if (deckId == null) return;

  const saveAgainst = async (snapshot: DeckWidgetDocumentSnapshot) => {
    if (
      !snapshot.instances.some(
        ({ legacyButtonId }) => legacyButtonId === button.id,
      )
    )
      return null;
    return widgetDocuments.save(
      deckId,
      snapshot.revision,
      snapshot.instances.map((instance) =>
        reconcileLegacyWidgetInstance(instance, button),
      ),
    );
  };

  let saved: DeckWidgetDocumentSnapshot | null;
  try {
    saved = await saveAgainst(persistedWidgetSnapshot.value);
  } catch {
    // Another client may have advanced the document revision. Reconcile the
    // canonical snapshot once so a mode change never overwrites newer layouts.
    saved = await saveAgainst(await widgetDocuments.load(deckId));
  }
  if (!saved) return;
  persistedWidgetSnapshot.value = saved;
  widgetEditor.value.reset(saved.instances, saved.revision);
};

const refreshDeckAfterLegacyMutation = async (activeDeckId?: number) => {
  const decks = await invoke<Deck[]>("get_all_decks");
  deckStore.setDecks(decks);
  if (activeDeckId != null) await deckStore.setActiveDeck(activeDeckId);
};

const handleModeSet = async (newMode: ComponentMode, buttonId: number) => {
  // if (!pendingDropData.value) return;

  const activeDeckId = deckStore.getActiveDeck?.id;
  const previousButton = deckStore.getButtonById(buttonId);
  const previousMode: ComponentMode =
    previousButton?.mode === "rotary" ? "rotary" : "click/press";
  let legacyModeChanged = false;
  try {
    // Invoke the backend with the selected action
    await invoke("set_button_mode", {
      buttonId: buttonId, // The ID of the button you want to update
      mode: newMode,
    });
    legacyModeChanged = true;

    await refreshDeckAfterLegacyMutation(activeDeckId);
    const updatedButton = deckStore.getButtonById(buttonId);
    if (updatedButton) await persistReconciledLegacyWidget(updatedButton);

    await emitTo("main", "set_active_deck", { deck: deckStore.getActiveDeck });
    refreshIcons();
    isButtonMenuOpen.value = false;

    // Clear the pending data
    pendingDropData.value = null;

    // Close the drop menu
    isDropMenuOpen.value = false;

    // Show success notification
    const buttonPosition =
      deckStore.getButtonById(buttonId)?.position || buttonId;
    const modeDisplayName =
      newMode === "click/press" ? "Click/Press" : "Rotary";

    mainStore.addNotification(
      {
        title: "Button Mode Changed",
        message: `Button G${buttonPosition} is now in ${modeDisplayName} mode`,
        type: "success",
        autoClose: true,
        duration: 3000,
        buttonText: "",
        buttonFunction: () => {},
        color: "bg-blue-500",
      },
      ["main", "simulator"],
    );
  } catch (error) {
    console.error("Error setting macro trigger:", error);

    if (legacyModeChanged && previousMode !== newMode) {
      try {
        await invoke("set_button_mode", { buttonId, mode: previousMode });
        await refreshDeckAfterLegacyMutation(activeDeckId);
        await emitTo("main", "set_active_deck", {
          deck: deckStore.getActiveDeck,
        });
      } catch (rollbackError) {
        console.error(
          "Failed to roll back incomplete button mode conversion",
          rollbackError,
        );
      }
    }

    // Show error notification
    mainStore.addNotification(
      {
        title: "Failed to Change Mode",
        message: `Could not change button mode. Please try again.`,
        type: "error",
        autoClose: true,
        duration: 5000,
        buttonText: "",
        buttonFunction: () => {},
        color: "bg-red-500",
      },
      ["main", "simulator"],
    );
  }
};

const handleImageFitSet = async (
  imageFit: ButtonIconFit,
  buttonId?: number,
) => {
  if (buttonId == null) return;
  try {
    await invoke<Button>("set_button_icon_fit", {
      buttonId,
      iconFit: imageFit,
    });
    const activeDeckId = deckStore.getActiveDeck?.id;
    const decks = await invoke<Deck[]>("get_all_decks");
    deckStore.setDecks(decks);
    if (activeDeckId != null) deckStore.setActiveDeck(activeDeckId);
    await emitTo("main", "update_active_deck", deckStore.getActiveDeck);
    isButtonMenuOpen.value = false;
  } catch (error) {
    console.error("Failed to update button image fit", error);
  }
};

const persistedDropButton = (instance: DeckWidgetInstance) => {
  if (!isLegacyWidgetInstance(instance)) return undefined;
  const button = legacyButtonForInstance(instance);
  return button && typeof button.id === "number" ? button : undefined;
};

const allowPersistedWidgetDrop = (
  event: DragEvent,
  instance: DeckWidgetInstance,
) => {
  if (!persistedDropButton(instance) || !event.dataTransfer) return;
  event.preventDefault();
  event.dataTransfer.dropEffect = "copy";
};

const dropOnPersistedWidget = async (
  event: DragEvent,
  instance: DeckWidgetInstance,
) => {
  const button = persistedDropButton(instance);
  if (!button || typeof button.id !== "number") return;
  event.preventDefault();
  event.stopPropagation();
  const mode: ComponentMode =
    button.mode === "rotary" ? "rotary" : "click/press";
  await onDrop(event, button.id, mode);
};

const handleExecutionModeSet = async (
  executionMode: string,
  buttonId: number,
  executionConfig?: unknown,
  skipNotification: boolean = false,
) => {
  try {
    // Create default config for repeat_while_pressing mode
    let config = executionConfig;
    if (executionMode === "repeat_while_pressing" && !config) {
      config = {
        hold_threshold_ms: 300,
        repeat_interval_ms: 100,
      };
    }

    // Invoke the backend with the selected execution mode and config
    await invoke("set_button_execution_mode", {
      buttonId: buttonId,
      executionMode: executionMode,
      executionConfig: config,
    });

    // Refresh deck data
    const activeDeckId = deckStore.getActiveDeck?.id;
    const decks = await invoke<Deck[]>("get_all_decks");
    deckStore.setDecks(decks);
    if (activeDeckId != null) deckStore.setActiveDeck(activeDeckId);

    await emitTo("main", "set_active_deck", { deck: deckStore.getActiveDeck });
    refreshIcons();
    const updatedButton = deckStore.getButtonById(buttonId);
    if (updatedButton) await persistReconciledLegacyWidget(updatedButton);
    isButtonMenuOpen.value = false;

    // Clear the pending data
    pendingDropData.value = null;

    // Close the drop menu
    isDropMenuOpen.value = false;

    // Show success notification only if not skipped
    if (!skipNotification) {
      const buttonPosition =
        deckStore.getButtonById(buttonId)?.position || buttonId;
      const modeDisplayName =
        executionMode === "run_once"
          ? "Run Once"
          : executionMode === "repeat_while_pressing"
            ? "Hold to Repeat"
            : executionMode === "switch"
              ? "Toggle"
              : "Queue";

      mainStore.addNotification(
        {
          title: "Execution Mode Changed",
          message: `Button G${buttonPosition} execution mode set to ${modeDisplayName}`,
          type: "success",
          autoClose: true,
          duration: 3000,
          buttonText: "",
          buttonFunction: () => {},
          color: "bg-green-500",
        },
        ["main", "simulator"],
      );
    }
  } catch (error) {
    console.error("Error setting execution mode:", error);

    // Show error notification
    mainStore.addNotification(
      {
        title: "Failed to Change Execution Mode",
        message: `Could not change button execution mode. Please try again.`,
        type: "error",
        autoClose: true,
        duration: 5000,
        buttonText: "",
        buttonFunction: () => {},
        color: "bg-red-500",
      },
      ["main", "simulator"],
    );
  }
};

const isChooseValueModalOpen = ref(false);
const isRepeatConfigModalOpen = ref(false);
const editingButtonId = ref<number | null>(null);

// Button execution mode state (simplified for backend-driven approach)
const pressedButtons = ref(new Set<number>());
const repeatingButtons = ref(new Set<number>()); // Buttons currently in repeat mode (for visual indicator)

// Repeat configuration reactive data
const repeatConfig = ref({
  hold_threshold_ms: 300,
  repeat_interval_ms: 100,
});

// Function to get repeat indicator animation style based on button config
const getRepeatIndicatorStyle = (button: SimulatorButton) => {
  const config = button.execution_config
    ? JSON.parse(button.execution_config)
    : null;
  const repeatInterval = config?.repeat_interval_ms || 100;

  // Animation duration should match the repeat interval for synchronization
  const animationDuration = `${repeatInterval}ms`;

  return {
    "animation-duration": animationDuration,
  };
};

// Handle locked button click
const handleLockedButtonClick = (position: number) => {
  console.log(
    "🔒 Locked button clicked, position:",
    position,
    "limit:",
    buttonLimits.effectiveButtonLimit.value,
  );

  // Wait for DOM to be ready, then find the overlay and add flash animation
  setTimeout(() => {
    const overlay = document.querySelector(`.lock-overlay-${position}`);
    if (overlay) {
      overlay.classList.add("button-blocked-flash");
      setTimeout(() => {
        overlay.classList.remove("button-blocked-flash");
      }, 500);
    }
  }, 10); // Small delay to let DOM update

  // Always show toast notification when locked button is clicked
  mainStore.addNotification(
    {
      title: "Button Limit Reached",
      message: `Upgrade to unlock more buttons (current limit: ${buttonLimits.effectiveButtonLimit.value})`,
      type: "warning",
      autoClose: true,
      duration: 4000,
      buttonText: "Upgrade",
      buttonFunction: () => {
        // Navigate to upgrade page or show upgrade modal
        console.log("Navigate to upgrade page");
      },
      color: "bg-yellow-500",
    },
    ["main", "simulator"],
  );
};

// Responsive lock overlay sizing based on grid size
const gridSize = computed(() => {
  const settings = deckStore.getActiveDeck?.settings;
  const [cols, rows] = parseGrid(settings?.grid);
  return { rows, cols };
});

const totalButtons = computed(() => {
  return gridSize.value.rows * gridSize.value.cols;
});

const getOverlayClasses = () => {
  const total = totalButtons.value;
  if (total <= 9) return "gap-2"; // 3x3 or smaller: normal spacing
  if (total <= 16) return "gap-1.5"; // 4x4: slightly smaller
  if (total <= 25) return "gap-1"; // 5x5: smaller spacing
  return "gap-0.5"; // 6x6+: minimal spacing
};

const getIconClasses = () => {
  const total = totalButtons.value;
  if (total <= 9) return "h-6 w-6"; // 3x3 or smaller: normal size
  if (total <= 16) return "h-5 w-5"; // 4x4: slightly smaller
  if (total <= 25) return "h-4 w-4"; // 5x5: smaller
  return "h-3 w-3"; // 6x6+: very small
};

const getTextClasses = () => {
  const total = totalButtons.value;
  if (total <= 9) return "text-[10px]"; // 3x3 or smaller: normal text
  if (total <= 16) return "text-[9px]"; // 4x4: slightly smaller
  if (total <= 25) return "text-[8px]"; // 5x5: smaller text
  return "text-[7px]"; // 6x6+: very small text
};

const showUpgradeText = () => {
  const total = totalButtons.value;
  return total <= 36; // Hide text when 6x6+ (36+ buttons)
};

// Button execution handlers
const handleButtonClick = (button: SimulatorButton, _event?: Event) => {
  const executionMode = button.execution_mode || "run_once";
  console.log(
    "👆 Button clicked:",
    button.id,
    "execution_mode:",
    executionMode,
  );
  if (!button.action || typeof button.id !== "number") return;

  if (executionMode === "switch") {
    // For switch mode, we just send the command once to toggle
    console.log("🔄 Switch mode - executing once:", button.action);
    useSendCommand(button.action, undefined, button.id);
  } else if (executionMode === "run_once") {
    // For run_once mode, execute immediately
    console.log("▶️ Run once mode - executing:", button.action);
    useSendCommand(button.action, undefined, button.id);
  } else if (executionMode === "repeat_while_pressing") {
    // For repeat mode, execution is handled by press/release logic
    // This click event may fire, but the actual execution is controlled by the press/release handlers
    console.log("🟡 Repeat mode - execution handled by press/release logic");
  }
};

const handleButtonPress = (button: SimulatorButton, _event?: Event) => {
  const executionMode = button.execution_mode || "run_once";
  console.log(
    "🔽 Button pressed:",
    button.id,
    "execution_mode:",
    executionMode,
    "action:",
    button.action,
  );

  if (executionMode === "repeat_while_pressing") {
    if (typeof button.id !== "number") return;
    // Prevent multiple presses on the same button
    if (pressedButtons.value.has(button.id)) {
      console.log("⚠️ Button already pressed, ignoring:", button.id);
      return;
    }

    // Check if button has an action
    if (!button.action) {
      console.log("⚠️ No action for button:", button.id);
      return;
    }

    pressedButtons.value.add(button.id);
    console.log(
      "✅ Button press detected, sending start_button_repeat to backend:",
      button.id,
    );

    // Send WebSocket message to backend to start repeat
    if ($socket.value?.readyState === WebSocket.OPEN) {
      const message = JSON.stringify({
        action: "start_button_repeat",
        buttonId: button.id,
      });
      $socket.value.send(message);
      console.log("📤 Sent start_button_repeat message to backend");
    } else {
      console.log("❌ WebSocket not connected");
    }
  }
};

const handleButtonRelease = (button: SimulatorButton, _event?: Event) => {
  const executionMode = button.execution_mode || "run_once";
  console.log(
    "🔼 Button released:",
    button.id,
    "execution_mode:",
    executionMode,
    "action:",
    button.action,
  );

  if (executionMode === "repeat_while_pressing") {
    if (typeof button.id !== "number") return;
    // Clean up pressed state
    if (pressedButtons.value.has(button.id)) {
      pressedButtons.value.delete(button.id);
      console.log("✅ Button removed from pressed set:", button.id);
    }

    // Send WebSocket message to backend to stop repeat
    if ($socket.value?.readyState === WebSocket.OPEN) {
      const message = JSON.stringify({
        action: "stop_button_repeat",
        buttonId: button.id,
      });
      $socket.value.send(message);
      console.log("📤 Sent stop_button_repeat message to backend");
    } else {
      console.log("❌ WebSocket not connected");
    }

    // Remove from repeating set to hide visual indicator
    // This will be managed by WebSocket messages from backend
    repeatingButtons.value.delete(button.id);
  }
};

// Open repeat configuration modal
const openRepeatConfigModal = () => {
  if (!editingButton.value) return;

  // Load current configuration from the editing button
  const currentConfig = editingButton.value.execution_config
    ? JSON.parse(editingButton.value.execution_config)
    : { hold_threshold_ms: 300, repeat_interval_ms: 100 };

  // Update the reactive config with current values
  repeatConfig.value = { ...currentConfig };

  // Close the button menu and open the modal
  isButtonMenuOpen.value = false;
  isRepeatConfigModalOpen.value = true;
};

// Handle repeat configuration apply from modal
const handleRepeatConfigApply = async (config: {
  hold_threshold_ms: number;
  repeat_interval_ms: number;
}) => {
  if (!editingButton.value) return;

  try {
    // Update the reactive config with the new values
    repeatConfig.value = { ...config };

    // Call the execution mode setter with the custom configuration (skip notification since we show our own)
    await handleExecutionModeSet(
      "repeat_while_pressing",
      editingButton.value.id,
      config,
      true,
    );

    // Close the modal
    isRepeatConfigModalOpen.value = false;

    // Show success notification
    mainStore.addNotification(
      {
        title: "Repeat Configuration Applied",
        message: `Hold: ${config.hold_threshold_ms}ms, Repeat: ${config.repeat_interval_ms}ms`,
        type: "success",
        autoClose: true,
        duration: 3000,
        buttonText: "",
        buttonFunction: () => {},
        color: "bg-green-500",
      },
      ["main", "simulator"],
    );
  } catch (error) {
    console.error("Failed to apply repeat configuration:", error);

    // Show error notification
    mainStore.addNotification(
      {
        title: "Configuration Failed",
        message: `Could not apply repeat configuration. Please try again.`,
        type: "error",
        autoClose: true,
        duration: 5000,
        buttonText: "",
        buttonFunction: () => {},
        color: "bg-red-500",
      },
      ["main", "simulator"],
    );
  }
};

// Apply repeat configuration from modal (legacy - keeping for backward compatibility)
const applyRepeatConfigFromModal = async () => {
  if (!editingButton.value) return;

  try {
    // Call the execution mode setter with the custom configuration
    await handleExecutionModeSet(
      "repeat_while_pressing",
      editingButton.value.id,
      repeatConfig.value,
    );

    // Close the modal
    isRepeatConfigModalOpen.value = false;

    // Show success notification
    mainStore.addNotification(
      {
        title: "Repeat Configuration Applied",
        message: `Hold: ${repeatConfig.value.hold_threshold_ms}ms, Repeat: ${repeatConfig.value.repeat_interval_ms}ms`,
        type: "success",
        autoClose: true,
        duration: 3000,
        buttonText: "",
        buttonFunction: () => {},
        color: "bg-green-500",
      },
      ["main", "simulator"],
    );
  } catch (error) {
    console.error("Failed to apply repeat configuration:", error);

    // Show error notification
    mainStore.addNotification(
      {
        title: "Configuration Failed",
        message: `Could not apply repeat configuration. Please try again.`,
        type: "error",
        autoClose: true,
        duration: 5000,
        buttonText: "",
        buttonFunction: () => {},
        color: "bg-red-500",
      },
      ["main", "simulator"],
    );
  }
};

// Apply repeat configuration (legacy function - keeping for compatibility)
const applyRepeatConfig = async () => {
  if (!editingButton.value) return;

  try {
    // Call the execution mode setter with the custom configuration
    await handleExecutionModeSet(
      "repeat_while_pressing",
      editingButton.value.id,
      repeatConfig.value,
    );

    // Close the menu after successful configuration
    isButtonMenuOpen.value = false;

    // Show success notification
    mainStore.addNotification(
      {
        title: "Repeat Configuration Applied",
        message: `Hold: ${repeatConfig.value.hold_threshold_ms}ms, Repeat: ${repeatConfig.value.repeat_interval_ms}ms`,
        type: "success",
        autoClose: true,
        duration: 3000,
        buttonText: "",
        buttonFunction: () => {},
        color: "bg-green-500",
      },
      ["main", "simulator"],
    );
  } catch (error) {
    console.error("Failed to apply repeat configuration:", error);

    // Show error notification
    mainStore.addNotification(
      {
        title: "Configuration Failed",
        message: `Could not apply repeat configuration. Please try again.`,
        type: "error",
        autoClose: true,
        duration: 5000,
        buttonText: "",
        buttonFunction: () => {},
        color: "bg-red-500",
      },
      ["main", "simulator"],
    );
  }
};

// Cleanup when component unmounts
onUnmounted(() => {
  // Clear pressed buttons set
  pressedButtons.value.clear();

  // Clear repeating buttons set
  repeatingButtons.value.clear();
});

const handleMacroTrigger = async (buttonMode: ComponentMode) => {
  if (
    !pendingDropData.value ||
    pendingDropData.value.event !== "action-reference"
  )
    return;

  try {
    const { buttonId, reference } = pendingDropData.value.data;
    const label = pendingDropData.value.data.label ?? reference.id;

    // For system control actions, we may need to get a value from user
    if (reference.kind === "system_control") {
      if (!buttonMode || buttonMode === "click/press") {
        // Save current editing state
        editingButtonId.value = buttonId;

        // Open modal and wait for user input
        isChooseValueModalOpen.value = true;

        // Wait for user to confirm selection using a Promise
        await new Promise<void>((resolve) => {
          // Store the resolve function to call it when user confirms
          confirmValueResolve.value = resolve;
        });

        // At this point, the modal has been confirmed and selectedValue contains the user's choice
        // Create updated action data with the selected value
        const baseId = reference.id.split(":")[0];
        const targetValue = Number.isFinite(selectedValue.value)
          ? selectedValue.value
          : 50;
        const updatedReference: ActionReference = {
          ...reference,
          id: `${baseId}:${targetValue}`,
        };

        // Set the button action with the updated data
        await setButtonAction(buttonId, updatedReference, label);
        // Clear the pending data
        pendingDropData.value = null;

        // Close the drop menu
        isDropMenuOpen.value = false;
        return; // Exit early as we've already handled this case
      } else if (buttonMode === "rotary") {
        console.log("Rotary system control - will use rotary values");
        // For rotary controls, we don't need a modal
      } else {
        console.log("System control value not available for this button mode");
        return; // Exit if incompatible mode
      }
    }

    // Continue with setting the button action for non-system control actions
    await setButtonAction(buttonId, reference, label);
    if (buttonMode === "click/press") {
      await handleExecutionModeSet("run_once", buttonId, undefined, true);
    }
    // Clear the pending data
    pendingDropData.value = null;

    // Close the drop menu
    isDropMenuOpen.value = false;
  } catch (error) {
    console.error("Error setting macro trigger:", error);
  }
};

// Store the resolve function for the confirmation promise
const confirmValueResolve = ref<(() => void) | null>(null);

const confirmValueSelection = () => {
  // Close the modal
  isChooseValueModalOpen.value = false;

  // If we have a resolve function, call it to continue execution
  if (confirmValueResolve.value) {
    confirmValueResolve.value();
    confirmValueResolve.value = null;
  }
};

const setButtonAction = async (
  buttonId: number,
  actionReference: ActionReference,
  label?: string,
) => {
  try {
    await useSetButtonAction(buttonId, actionReference, label);

    const deck = await invoke<Deck>("get_active_deck");
    await deckStore.updateActiveDeck(deck);
    await emitTo("main", "update_active_deck", deckStore.getActiveDeck);

    const updatedButton = deckStore.getButtonById(buttonId);
    if (updatedButton) await persistReconciledLegacyWidget(updatedButton);

    // Show success notification
    const actionName = label || actionReference.id || "Action";
    const buttonPosition =
      deckStore.getButtonById(buttonId)?.position || buttonId;

    mainStore.addNotification(
      {
        title: "Action Added Successfully",
        message: `"${actionName}" has been assigned to button G${buttonPosition}`,
        type: "success",
        autoClose: true,
        duration: 4000,
        buttonText: "",
        buttonFunction: () => {},
        color: "bg-green-500",
      },
      ["main", "simulator"],
    );
  } catch (error) {
    console.error("Error setting button action:", error);

    // Show error notification
    mainStore.addNotification(
      {
        title: "Failed to Add Action",
        message: `Could not assign action to button. Please try again.`,
        type: "error",
        autoClose: true,
        duration: 5000,
        buttonText: "",
        buttonFunction: () => {},
        color: "bg-red-500",
      },
      ["main", "simulator"],
    );
  }
};

const handleDocumentMouseDown = (event: MouseEvent) => {
  const target = event.target;
  if (
    !(target instanceof Element) ||
    !target.closest("[data-menu], .context-menu-floating")
  )
    resetMenus();
};

let modifierPollTimeout: ReturnType<typeof setTimeout> | null = null;
let modifierPollRequestId = 0;
let isCheckingModifiers = false;
let isPointerInsideDocument = false;

const pollModifiers = async () => {
  if (!isPointerInsideDocument) return;
  if (isCheckingModifiers) return;
  isCheckingModifiers = true;
  const currentRequestId = ++modifierPollRequestId;

  try {
    const mods = await invoke<{
      isAlt?: boolean;
      is_alt?: boolean;
      isShift?: boolean;
      is_shift?: boolean;
    }>("get_global_modifiers");

    // Strictly discard stale responses if polling stopped, pointer left, or newer event occurred
    if (currentRequestId !== modifierPollRequestId || !isPointerInsideDocument) {
      return;
    }

    if (mods) {
      const alt = Boolean(mods.isAlt ?? mods.is_alt);
      const shift = Boolean(mods.isShift ?? mods.is_shift);
      if (deckStore.getIsAlt !== alt) deckStore.setIsAlt(alt);
      if (deckStore.getIsShift !== shift) deckStore.setIsShift(shift);
    }
  } catch {
    // Unsupported or non-Tauri environment fallback
  } finally {
    isCheckingModifiers = false;
    if (isPointerInsideDocument) {
      modifierPollTimeout = setTimeout(pollModifiers, 40);
    }
  }
};

const startModifierPolling = () => {
  isPointerInsideDocument = true;
  if (modifierPollTimeout != null || isCheckingModifiers) return;
  void pollModifiers();
};

const stopModifierPolling = () => {
  isPointerInsideDocument = false;
  modifierPollRequestId++; // Invalidate all pending in-flight requests immediately
  if (modifierPollTimeout != null) {
    clearTimeout(modifierPollTimeout);
    modifierPollTimeout = null;
  }
  isCheckingModifiers = false;
  if (deckStore.getIsAlt) deckStore.setIsAlt(false);
  if (deckStore.getIsShift) deckStore.setIsShift(false);
};

const syncPointerModifiers = (event: PointerEvent | MouseEvent) => {
  isPointerInsideDocument = true;
  if (modifierPollTimeout == null && !isCheckingModifiers) {
    startModifierPolling();
  }
  const alt = Boolean(event.altKey);
  const shift = Boolean(event.shiftKey);
  // Fast path: direct pointer event modifier sync (0ms latency)
  if (deckStore.getIsAlt !== alt) {
    modifierPollRequestId++; // Invalidate any older in-flight IPC checks
    deckStore.setIsAlt(alt);
  }
  if (deckStore.getIsShift !== shift) {
    modifierPollRequestId++;
    deckStore.setIsShift(shift);
  }
};

const handleDocumentMouseEnter = (event: MouseEvent) => {
  isPointerInsideDocument = true;
  syncPointerModifiers(event);
  startModifierPolling();
};

const handleDocumentMouseLeave = (event: MouseEvent) => {
  // If moving out of the window document completely
  if (!event.relatedTarget) {
    stopModifierPolling();
  }
};

const handleWindowBlur = () => {
  // Only stop modifier polling if the cursor is also outside the simulator
  if (!isPointerInsideDocument) {
    stopModifierPolling();
  }
};

const handleModifierKeyDown = (event: KeyboardEvent) => {
  if (event.key === "Alt") {
    event.preventDefault();
    modifierPollRequestId++;
    deckStore.setIsAlt(true);
    deckStore.setIsShift(false);
  } else if (event.key === "Shift") {
    event.preventDefault();
    modifierPollRequestId++;
    deckStore.setIsShift(true);
    deckStore.setIsAlt(false);
  }
};

const handleModifierKeyUp = (event: KeyboardEvent) => {
  if (event.key === "Alt") {
    event.preventDefault();
    modifierPollRequestId++;
    deckStore.setIsAlt(false);
  } else if (event.key === "Shift") {
    event.preventDefault();
    modifierPollRequestId++;
    deckStore.setIsShift(false);
  }
};

// Close menus when clicking outside and track modifier keys while the page is active.
onMounted(() => {
  emitTo("main", "mobileDeckLoaded");
  document.addEventListener("mousedown", handleDocumentMouseDown);
  document.addEventListener("keydown", handleModifierKeyDown);
  document.addEventListener("keyup", handleModifierKeyUp);
  document.addEventListener("mouseenter", handleDocumentMouseEnter);
  document.addEventListener("mouseleave", handleDocumentMouseLeave);
  window.addEventListener("pointermove", syncPointerModifiers, { passive: true });
  window.addEventListener("pointerdown", syncPointerModifiers, { passive: true });
  window.addEventListener("pointerover", syncPointerModifiers, { passive: true });
  window.addEventListener("blur", handleWindowBlur);
  startModifierPolling();
});

onUnmounted(() => {
  stopModifierPolling();
  document.removeEventListener("mousedown", handleDocumentMouseDown);
  document.removeEventListener("keydown", handleModifierKeyDown);
  document.removeEventListener("keyup", handleModifierKeyUp);
  document.removeEventListener("mouseenter", handleDocumentMouseEnter);
  document.removeEventListener("mouseleave", handleDocumentMouseLeave);
  window.removeEventListener("pointermove", syncPointerModifiers);
  window.removeEventListener("pointerdown", syncPointerModifiers);
  window.removeEventListener("pointerover", syncPointerModifiers);
  window.removeEventListener("blur", handleWindowBlur);
});

// Handle file selection
onChange(async (selectedFiles) => {
  const file = selectedFiles?.[0];
  if (file) {
    // Create a FileReader to read the file as a Data URL (Base64)
    const reader = new FileReader();
    reader.onloadend = async () => {
      const base64Data = reader.result;
      const button = editingButton.value;
      if (typeof base64Data !== "string" || !button) return;

      imageData.value = base64Data; // Set the image preview
      // Remove the 'data:image/png;base64,' part from the Base64 string

      imageData.value = base64Data; // Set the image preview
      const base64SaveData = base64Data.split(",", 2)[1]; // Get only the Base64 data without the prefix
      if (!base64SaveData) return;
      // Now send this to the Tauri command for processing
      await invoke("save_icon", {
        buttonId: `${button.id}`,
        base64Image: base64SaveData, // Send the clean Base64 data
      });

      refreshIcons();
      resetButtonOptions();
    };
    reader.readAsDataURL(file); // This reads the file as a Base64 string
  }
});

const selectedButton = ref<SimulatorButton | null>(null);
const setDeck = async ({ payload }: TauriEvent<DeckEventPayload>) => {
  console.log(payload);
  deckStore.updateActiveDeck(payload.deck);
  emitTo("main", "update_active_deck", deckStore.getActiveDeck);
  swapy?.update();
};

const setTheme = async ({ payload }: TauriEvent<ThemeEventPayload>) => {
  deckStore.setActiveTheme(payload.theme);
};

const serverInfo = await invoke<ServerInfo | null>("get_server_info_new");
console.log(serverInfo);

// Handle null serverInfo with fallback and proper error handling
if (!serverInfo) {
  console.error("Server info is null - using fallback values");
  throw new Error("Failed to retrieve server information");
}

const iconUrlBase = ref(
  `http://${serverInfo.ip}:${serverInfo.iconsServerPort}/icons/`,
);
deckStore.setIconUrlBase(iconUrlBase.value);

const editingButton = computed(() => {
  const buttonId = lastToggledButton.value;
  return buttonId == null ? undefined : deckStore.getButtonById(buttonId);
});

// Watch for changes in editing button to update repeat configuration
watch(
  editingButton,
  (newButton) => {
    if (newButton && newButton.execution_config) {
      try {
        const config = JSON.parse(newButton.execution_config);
        repeatConfig.value = {
          hold_threshold_ms: config.hold_threshold_ms || 300,
          repeat_interval_ms: config.repeat_interval_ms || 100,
        };
      } catch (error) {
        console.error("Failed to parse execution config:", error);
        // Reset to defaults if parsing fails
        repeatConfig.value = {
          hold_threshold_ms: 300,
          repeat_interval_ms: 100,
        };
      }
    } else {
      // Reset to defaults if no config
      repeatConfig.value = {
        hold_threshold_ms: 300,
        repeat_interval_ms: 100,
      };
    }
  },
  { immediate: true },
);

const resetButtonOptions = () => {
  lastToggledButton.value = null;
  isButtonMenuOpen.value = false;
  showImageFitDropdown.value = false;
};

const iconRefreshKey = ref(Date.now()); // This can be any value, but using a timestamp ensures uniqueness.

const refreshIcons = () => {
  console.log("Refreshing icons...");
  // Change the key to force re-render

  iconRefreshKey.value = Date.now(); // Update the key with a new timestamp
  deckStore.refreshIcons();
  swapy?.update();
};
const unlisten = await listen("set_active_deck", setDeck);
// const unlistenRefresh = await listen("refresh", refreshIcons);
const unlisten2 = await listen("set_active_theme", setTheme);

const unlistenToSystemVolume = await listen<SystemVolumePayload>(
  "system_volume_changed",
  ({ payload }) => {
    // alert(JSON.stringify(data))
    console.log(payload);
    if (payload.is_button_change) {
      deckStore.setSystemVolume(
        payload.volume,
        payload.is_button_change,
        payload.button_id,
      );
    } else {
      deckStore.setSystemVolume(payload.volume, payload.is_button_change);
    }

    latestSystemVolume.value = payload.volume;
  },
);
const unlistenWidgetState = await listen<DeckWidgetStateMessage>(
  "deck_widget_state",
  ({ payload }) => {
    if (payload.deckId !== deckStore.getActiveDeck?.id) return;
    for (const patch of payload.patches) widgetRuntime.applyPatch(patch);
  },
);
const unlistenWidgetIntentAck = await listen<DeckWidgetIntentAckMessage>(
  "deck_widget_intent_ack",
  ({ payload }) => {
    if (
      payload.interactionSessionId &&
      payload.interactionSessionId !==
        widgetInteractionRouter.interactionSessionId
    )
      return;
    if (payload.status !== "rejected") return;
    mainStore.addNotification(
      {
        title: "Widget action rejected",
        message:
          payload.message ??
          payload.code ??
          "The host rejected this widget interaction.",
        type: "error",
        autoClose: true,
        duration: 5000,
        buttonText: "",
        buttonFunction: () => {},
        color: "bg-red-500",
      },
      "main",
    );
  },
);

onUnmounted(() => {
  unlisten();
  unlisten2();
  unlistenToSystemVolume();
  unlistenWidgetState();
  unlistenWidgetIntentAck();
});
const sortedButtons = computed(() => {
  // based on grid settings render buttons and empty slots
  const activeDeck = deckStore.getActiveDeck;
  if (!activeDeck) return [];

  const [cols, rows] = parseGrid(activeDeck.settings.grid);
  const totalSlots = cols * rows;

  // Get all buttons from the active deck
  const allButtons = activeDeck.buttons;

  // Create a result array with exactly totalSlots items
  const result: Array<SimulatorButton | null> = new Array(totalSlots).fill(
    null,
  );

  // First, place all buttons into their respective positions in the result array
  // If a position is out of bounds, we'll handle it later
  allButtons.forEach((button) => {
    // Convert position to zero-based index for array
    const index = button.position - 1;

    // Only place button if position is valid
    if (index >= 0 && index < totalSlots) {
      result[index] = {
        id: button.id,
        label: button.label,
        icon: button.icon || null,
        action: button.action || null,
        position: button.position,
        background: button.background || null,
        deck_id: activeDeck.id,
        mode: button.mode || null,
        execution_mode: button.execution_mode || "run_once",
        execution_config: button.execution_config || null,
        icon_updated_at: button.icon_updated_at,
        icon_fit: button.icon_fit ?? "cover",
      };
    }
  });

  // Now fill in any null slots with empty buttons
  for (let i = 0; i < result.length; i++) {
    if (result[i] === null) {
      result[i] = {
        id: `empty-${i + 1}`,
        label: "",
        icon: null,
        action: null,
        position: i + 1,
        background: null,
        deck_id: activeDeck.id,
        mode: null,
        execution_mode: "run_once",
        execution_config: null,
        icon_updated_at: null,
        icon_fit: "cover",
      };
    }
  }

  return result.filter((button): button is SimulatorButton => button !== null);
});

const widgetInstances = computed(() => {
  const instances = new Map<string, ReturnType<typeof legacyButtonToWidget>>();
  for (const button of sortedButtons.value) {
    const instance = legacyButtonToWidget(button, {
      portrait: gridSize.value.cols,
      landscape: gridSize.value.rows,
    });
    instances.set(instance.id, instance);
  }

  return instances;
});

const hasPersistedWidgetDocument = computed(
  () =>
    persistedWidgetSnapshot.value.deckId === deckStore.getActiveDeck?.id &&
    !persistedWidgetSnapshot.value.legacyBacked,
);

const availableWidgetDefinitions = computed(() =>
  widgetRegistry.list.value.filter(
    (entry) =>
      entry.definition.id !== "vortideck.action-button" &&
      entry.definition.id !== "vortideck.rotary",
  ),
);

const removedLegacyButtons = computed(() => {
  if (!hasPersistedWidgetDocument.value) return [];
  const activeLegacyIds = new Set(
    persistedWidgetSnapshot.value.instances
      .map(({ legacyButtonId }) => legacyButtonId)
      .filter((id): id is number => typeof id === "number"),
  );
  return sortedButtons.value.filter(
    (button): button is SimulatorButton & { id: number } =>
      typeof button.id === "number" &&
      !activeLegacyIds.has(button.id) &&
      !isVacantLegacyButton(button),
  );
});

const availableLegacySlots = computed(() => {
  if (!hasPersistedWidgetDocument.value) return [];
  const activeLegacyIds = new Set(
    persistedWidgetSnapshot.value.instances
      .map(({ legacyButtonId }) => legacyButtonId)
      .filter((id): id is number => typeof id === "number"),
  );
  return sortedButtons.value.filter(
    (button): button is SimulatorButton & { id: number } =>
      typeof button.id === "number" &&
      !activeLegacyIds.has(button.id) &&
      isVacantLegacyButton(button),
  );
});

const restorableRecentWidgets = computed(() => {
  const activeIds = new Set(
    persistedWidgetSnapshot.value.instances.map(({ id }) => id),
  );
  return recentlyRemovedWidgets.value.filter(
    (instance) =>
      !activeIds.has(instance.id) &&
      widgetRegistry.resolve(instance.definitionId),
  );
});

const selectedPersistedInstance = computed(() =>
  persistedWidgetSnapshot.value.instances.find(
    (instance) => instance.id === selectedWidgetId.value,
  ),
);

const setWidgetSelection = (
  ids: Iterable<string>,
  primaryId?: string | null,
) => {
  const next = new Set(ids);
  isMultiSelectionMenuOpen.value = false;
  selectedWidgetIds.value = next;
  selectedWidgetId.value =
    primaryId && next.has(primaryId)
      ? primaryId
      : (next.values().next().value ?? null);
  if (next.size !== 1) isWidgetInspectorOpen.value = false;
};

const toggleWidgetSelection = (instanceId: string) => {
  const next = new Set(selectedWidgetIds.value);
  if (next.has(instanceId)) next.delete(instanceId);
  else next.add(instanceId);
  setWidgetSelection(next, next.has(instanceId) ? instanceId : undefined);
};

const activeWidgetInstances = computed(() => {
  if (hasPersistedWidgetDocument.value) {
    return new Map(
      persistedWidgetSnapshot.value.instances.map((instance) => [
        instance.id,
        instance,
      ]),
    );
  }
  return widgetInstances.value;
});

const displayedWidgetInstances = computed<DeckWidgetInstance[]>(() =>
  Array.from(activeWidgetInstances.value.values()),
);

const isInstanceRepeating = (instance: DeckWidgetInstance) => {
  const button = legacyButtonForInstance(instance);
  return Boolean(
    button &&
      button.execution_mode === "repeat_while_pressing" &&
      typeof button.id === "number" &&
      repeatingButtons.value.has(button.id),
  );
};

const getRepeatIndicatorStyleForInstance = (instance: DeckWidgetInstance) => {
  const button = legacyButtonForInstance(instance);
  return button ? getRepeatIndicatorStyle(button) : {};
};

const unavailableDefinition = (definitionId: string): WidgetDefinition => ({
  id: definitionId,
  name: "Unavailable widget",
  category: "unavailable",
  rendererId: "unavailable",
  interactionType: "telemetry_display",
  defaultSpan: { cols: 1, rows: 1 },
  minSpan: { cols: 1, rows: 1 },
  maxSpan: { cols: 1, rows: 1 },
  configSchema: {},
  inputs: [],
  mobileSupport: "fallback_only",
});

const definitionForInstance = (instance: DeckWidgetInstance) =>
  widgetRegistry.resolve(instance.definitionId)?.definition ??
  unavailableDefinition(instance.definitionId);

const runtimeForInstance = (instanceId: string): WidgetRuntimeState =>
  widgetRuntime.get(instanceId);

const isLegacyWidgetInstance = (instance: DeckWidgetInstance) =>
  instance.legacyButtonId != null &&
  (instance.definitionId === legacyActionButtonDefinition.id ||
    instance.definitionId === legacyRotaryDefinition.id);

const legacyButtonForInstance = (instance: DeckWidgetInstance) =>
  instance.legacyButtonId == null
    ? undefined
    : sortedButtons.value.find(({ id }) => id === instance.legacyButtonId);

const persistedWidgetButtonPosition = (instance: DeckWidgetInstance) => {
  const placement = persistedWidgetPlacement(instance);
  if (!placement) return 1;
  return (
    firstRestrictedCellPosition(
      placement,
      editorGridBounds.value,
      buttonLimits.effectiveButtonLimit.value,
    ) ?? placement.y * editorGridBounds.value.cols + placement.x + 1
  );
};

const isPersistedWidgetLocked = (instance: DeckWidgetInstance) => {
  if (buttonLimits.isLoading.value) return false;
  const placement = persistedWidgetPlacement(instance);
  return Boolean(
    placement &&
    firstRestrictedCellPosition(
      placement,
      editorGridBounds.value,
      buttonLimits.effectiveButtonLimit.value,
    ),
  );
};

const isVacantLegacyWidget = (instance: DeckWidgetInstance) => {
  if (instance.config.explicitlyAdded === true) return false;
  const button = legacyButtonForInstance(instance);
  return Boolean(button && isVacantLegacyButton(button));
};

const openPersistedWidgetContextMenu = (
  payload: { bounds: MenuPosition },
  instance: DeckWidgetInstance,
) => {
  if (
    selectedWidgetIds.value.size > 1 &&
    selectedWidgetIds.value.has(instance.id)
  ) {
    resetMenus();
    multiSelectionMenuPosition.value = payload.bounds;
    isMultiSelectionMenuOpen.value = true;
    return;
  }
  const button = legacyButtonForInstance(instance);
  if (button && typeof button.id === "number") {
    isWidgetInspectorOpen.value = false;
    setWidgetSelection([instance.id], instance.id);
    selectedButton.value = button;
    toggleButtonMenu(payload.bounds, button.id);
    return;
  }
  resetMenus();
  setWidgetSelection([instance.id], instance.id);
  widgetInspectorPosition.value = payload.bounds;
  isWidgetInspectorOpen.value = true;
};

const normalizeLegacyWidgetLayouts = (
  instances: DeckWidgetInstance[],
): DeckWidgetInstance[] =>
  instances.map((instance) => {
    if (instance.legacyButtonId == null) return instance;
    const position = Number(instance.config.position);
    if (!Number.isInteger(position) || position < 1) return instance;
    if (instance.layouts.portrait && instance.layouts.landscape)
      return instance;
    const columns = {
      portrait: gridSize.value.cols,
      landscape: gridSize.value.rows,
    };
    const portrait =
      instance.layouts.portrait ??
      legacyWidgetPlacementForLayout(instance, "portrait", columns);
    const landscape =
      instance.layouts.landscape ??
      legacyWidgetPlacementForLayout(instance, "landscape", columns);
    if (!portrait || !landscape) return instance;
    return {
      ...instance,
      layouts: {
        ...instance.layouts,
        portrait,
        landscape,
      },
    };
  });

const normalizeLegacyWidgetSnapshot = (
  snapshot: DeckWidgetDocumentSnapshot,
): DeckWidgetDocumentSnapshot => ({
  ...snapshot,
  instances: normalizeLegacyWidgetLayouts(snapshot.instances),
});

type WidgetPointerMode =
  | "move"
  | "resize-nw"
  | "resize-ne"
  | "resize-sw"
  | "resize-se";
type WidgetPointerEdit = {
  instanceId: string;
  instanceIds: string[];
  mode: WidgetPointerMode;
  pointerId: number;
  startX: number;
  startY: number;
  cellWidth: number;
  cellHeight: number;
  original: { x: number; y: number; cols: number; rows: number };
};
const widgetPointerEdit = ref<WidgetPointerEdit | null>(null);
const widgetPointerPreview = ref<{
  instanceId: string;
  placement: {
    x: number;
    y: number;
    cols: number;
    rows: number;
  };
} | null>(null);
const widgetPointerDragOffset = ref<{
  instanceIds: string[];
  x: number;
  y: number;
} | null>(null);
let suppressWidgetClickSelectionUntil = 0;
const widgetResizeHandles: Array<{
  mode: Exclude<WidgetPointerMode, "move">;
  className: string;
}> = [
  { mode: "resize-nw", className: "widget-resize-handle--nw" },
  { mode: "resize-ne", className: "widget-resize-handle--ne" },
  { mode: "resize-sw", className: "widget-resize-handle--sw" },
  { mode: "resize-se", className: "widget-resize-handle--se" },
];

type MarqueeRect = { left: number; top: number; width: number; height: number };
type MarqueeSelection = {
  pointerId: number;
  surface: HTMLElement;
  bounds: DOMRect;
  overlayBounds: DOMRect;
  startX: number;
  startY: number;
  baseSelection: Set<string>;
};
const marqueeSelection = ref<MarqueeSelection | null>(null);
const marqueeRect = ref<MarqueeRect | null>(null);
const marqueeRectStyle = computed(() => {
  const rect = marqueeRect.value;
  if (!rect) return {};
  return {
    left: `${rect.left}px`,
    top: `${rect.top}px`,
    width: `${rect.width}px`,
    height: `${rect.height}px`,
  };
});

const rectanglesIntersect = (first: DOMRect, second: DOMRect) =>
  first.left < second.right &&
  first.right > second.left &&
  first.top < second.bottom &&
  first.bottom > second.top;

const handleGridMarqueeMove = (event: PointerEvent) => {
  const selection = marqueeSelection.value;
  if (!selection || event.pointerId !== selection.pointerId) return;
  const currentX = Math.min(
    selection.bounds.right,
    Math.max(selection.bounds.left, event.clientX),
  );
  const currentY = Math.min(
    selection.bounds.bottom,
    Math.max(selection.bounds.top, event.clientY),
  );
  const left = Math.min(selection.startX, currentX);
  const top = Math.min(selection.startY, currentY);
  const rect = new DOMRect(
    left,
    top,
    Math.abs(currentX - selection.startX),
    Math.abs(currentY - selection.startY),
  );
  marqueeRect.value = {
    left: rect.left - selection.overlayBounds.left,
    top: rect.top - selection.overlayBounds.top,
    width: rect.width,
    height: rect.height,
  };
  const ids = new Set(selection.baseSelection);
  for (const element of selection.surface.querySelectorAll<HTMLElement>(
    "[data-editor-widget-id]",
  )) {
    const instanceId = element.dataset.editorWidgetId;
    if (
      instanceId &&
      rectanglesIntersect(rect, element.getBoundingClientRect())
    )
      ids.add(instanceId);
  }
  setWidgetSelection(ids);
};

const finishGridMarqueeSelection = (event?: PointerEvent) => {
  const selection = marqueeSelection.value;
  if (!selection || (event && event.pointerId !== selection.pointerId)) return;
  marqueeSelection.value = null;
  marqueeRect.value = null;
  window.removeEventListener("pointermove", handleGridMarqueeMove);
  window.removeEventListener("pointerup", finishGridMarqueeSelection);
  window.removeEventListener("pointercancel", finishGridMarqueeSelection);
};

const startGridMarqueeSelection = (event: PointerEvent) => {
  if (isInteractiveMode.value || event.button !== 0) return;
  const target = event.target;
  if (!(target instanceof Element)) return;
  if (
    target.closest(
      "[data-editor-widget-id], button, input, select, textarea, [data-menu], [role='dialog']",
    )
  )
    return;
  const surface = event.currentTarget;
  if (!(surface instanceof HTMLElement)) return;
  const overlay = surface.querySelector<HTMLElement>("[data-deck-widget-grid]");
  if (!overlay) return;
  const additive = event.shiftKey || event.ctrlKey || event.metaKey;
  if (!additive) setWidgetSelection([]);
  marqueeSelection.value = {
    pointerId: event.pointerId,
    surface,
    bounds: surface.getBoundingClientRect(),
    overlayBounds: overlay.getBoundingClientRect(),
    startX: event.clientX,
    startY: event.clientY,
    baseSelection: additive ? new Set(selectedWidgetIds.value) : new Set(),
  };
  marqueeRect.value = {
    left: event.clientX - marqueeSelection.value.overlayBounds.left,
    top: event.clientY - marqueeSelection.value.overlayBounds.top,
    width: 0,
    height: 0,
  };
  window.addEventListener("pointermove", handleGridMarqueeMove);
  window.addEventListener("pointerup", finishGridMarqueeSelection);
  window.addEventListener("pointercancel", finishGridMarqueeSelection);
  event.preventDefault();
};

const persistedWidgetPlacement = (instance: DeckWidgetInstance) => {
  // `isLandscape` means the simulator has been rotated from its default
  // landscape shell, so the visible document layout is portrait when true.
  const layoutKey = isLandscape.value ? "portrait" : "landscape";
  return widgetPointerEdit.value?.mode !== "move" &&
    widgetPointerPreview.value?.instanceId === instance.id
    ? widgetPointerPreview.value.placement
    : (instance.layouts[layoutKey] ?? instance.layouts.default);
};

const persistedWidgetPlacementStyle = (instance: DeckWidgetInstance) => {
  const placement = persistedWidgetPlacement(instance);
  if (!placement) return { display: "none" };
  const gap = Number(deckStore.getActiveDeck?.settings.button_spacing ?? 8);
  const horizontalGap = gap * Math.max(0, placement.cols - 1);
  const verticalGap = gap * Math.max(0, placement.rows - 1);
  const widthFromHeightRatio = placement.cols / placement.rows;
  const heightFromWidthRatio = placement.rows / placement.cols;
  // Match the mature grid's orientation rule: landscape derives square units
  // from column width, portrait derives them from row height. Choosing the
  // smaller axis here made portrait widgets visibly shrink after migration.
  const renderWidth = isLandscape.value
    ? `calc(${100 * widthFromHeightRatio}cqh + ${horizontalGap - verticalGap * widthFromHeightRatio}px)`
    : "100cqw";
  const renderHeight = isLandscape.value
    ? "100cqh"
    : `calc(${100 * heightFromWidthRatio}cqw + ${verticalGap - horizontalGap * heightFromWidthRatio}px)`;
  const horizontalOffset = isLandscape.value
    ? `calc(50cqw - ${50 * widthFromHeightRatio}cqh - ${(horizontalGap - verticalGap * widthFromHeightRatio) / 2}px)`
    : "0px";
  const verticalOffset = isLandscape.value
    ? "0px"
    : `calc(50cqh - ${50 * heightFromWidthRatio}cqw - ${(verticalGap - horizontalGap * heightFromWidthRatio) / 2}px)`;
  const dragOffset = widgetPointerDragOffset.value?.instanceIds.includes(
    instance.id,
  )
    ? widgetPointerDragOffset.value
    : null;
  return {
    gridColumn: `${placement.x + 1} / span ${placement.cols}`,
    gridRow: `${placement.y + 1} / span ${placement.rows}`,
    aspectRatio: `${placement.cols} / ${placement.rows}`,
    "--widget-render-width": renderWidth,
    "--widget-render-height": renderHeight,
    "--widget-render-offset-x": horizontalOffset,
    "--widget-render-offset-y": verticalOffset,
    transform: dragOffset
      ? `translate3d(${dragOffset.x}px, ${dragOffset.y}px, 0)`
      : undefined,
    zIndex: dragOffset ? 55 : undefined,
  };
};

const handleWidgetPointerMove = (event: PointerEvent) => {
  const edit = widgetPointerEdit.value;
  if (!edit || event.pointerId !== edit.pointerId) return;
  const instance = persistedWidgetSnapshot.value.instances.find(
    ({ id }) => id === edit.instanceId,
  );
  if (!instance) return;
  const definition = definitionForInstance(instance);
  const dx = Math.round(
    (event.clientX - edit.startX) / Math.max(1, edit.cellWidth),
  );
  const dy = Math.round(
    (event.clientY - edit.startY) / Math.max(1, edit.cellHeight),
  );
  let candidate =
    edit.mode === "move"
      ? { ...edit.original, x: edit.original.x + dx, y: edit.original.y + dy }
      : { ...edit.original };
  if (edit.mode !== "move") {
    const west = edit.mode.endsWith("w");
    const north = edit.mode.includes("n");
    const cols = Math.min(
      definition.maxSpan.cols,
      Math.max(definition.minSpan.cols, edit.original.cols + (west ? -dx : dx)),
    );
    const rows = Math.min(
      definition.maxSpan.rows,
      Math.max(
        definition.minSpan.rows,
        edit.original.rows + (north ? -dy : dy),
      ),
    );
    candidate = {
      x: west ? edit.original.x + edit.original.cols - cols : edit.original.x,
      y: north ? edit.original.y + edit.original.rows - rows : edit.original.y,
      cols,
      rows,
    };
  }
  const currentInstances = widgetEditor.value.instances();
  const layoutKey = editorLayoutKey();
  const result =
    edit.mode === "move" && edit.instanceIds.length > 1
      ? moveWidgetsWithReflow(
          currentInstances,
          edit.instanceIds,
          layoutKey,
          {
            x: candidate.x - edit.original.x,
            y: candidate.y - edit.original.y,
          },
          editorGridBounds.value,
        )
      : edit.mode === "move"
        ? placeWidgetWithReflow(
            currentInstances,
            edit.instanceId,
            layoutKey,
            candidate,
            editorGridBounds.value,
          )
        : canPlaceWidget(
              currentInstances,
              edit.instanceId,
              layoutKey,
              candidate,
              editorGridBounds.value,
            )
          ? {
              ok: true as const,
              instances: currentInstances.map((candidateInstance) =>
                candidateInstance.id === edit.instanceId
                  ? {
                      ...candidateInstance,
                      layouts: {
                        ...candidateInstance.layouts,
                        [layoutKey]: candidate,
                      },
                    }
                  : candidateInstance,
              ),
            }
          : { ok: false as const };
  if (result.ok) {
    widgetPointerPreview.value = {
      instanceId: edit.instanceId,
      placement: candidate,
    };
  } else if (edit.mode === "move") {
    widgetPointerPreview.value = null;
  }
  if (edit.mode === "move") {
    if (
      Math.abs(event.clientX - edit.startX) > 4 ||
      Math.abs(event.clientY - edit.startY) > 4
    ) {
      suppressWidgetClickSelectionUntil = Date.now() + 150;
    }
    widgetPointerDragOffset.value = {
      instanceIds: edit.instanceIds,
      x: event.clientX - edit.startX,
      y: event.clientY - edit.startY,
    };
  }
};

const finishWidgetPointerEdit = async (event?: PointerEvent) => {
  const edit = widgetPointerEdit.value;
  if (!edit || (event && event.pointerId !== edit.pointerId)) return;
  const preview = widgetPointerPreview.value;
  widgetPointerEdit.value = null;
  widgetPointerPreview.value = null;
  widgetPointerDragOffset.value = null;
  window.removeEventListener("pointermove", handleWidgetPointerMove);
  window.removeEventListener("pointerup", finishWidgetPointerEdit);
  window.removeEventListener("pointercancel", finishWidgetPointerEdit);
  if (!preview) return;
  const instance = widgetEditor.value
    .instances()
    .find(({ id }) => id === preview.instanceId);
  if (!instance) return;
  if (edit.mode === "move" && edit.instanceIds.length > 1) {
    await applyWidgetEdit(
      () =>
        widgetEditor.value.moveMany(
          edit.instanceIds,
          editorLayoutKey(),
          {
            x: preview.placement.x - edit.original.x,
            y: preview.placement.y - edit.original.y,
          },
          editorGridBounds.value,
        ),
      true,
    );
    return;
  }
  await applyWidgetEdit(
    () =>
      widgetEditor.value.placeWithReflow(
        instance.id,
        definitionForInstance(instance),
        editorLayoutKey(),
        preview.placement,
        editorGridBounds.value,
      ),
    true,
  );
};

const startWidgetPointerEdit = (
  event: PointerEvent,
  instance: DeckWidgetInstance,
  mode: WidgetPointerMode,
) => {
  if (isInteractiveMode.value || event.button !== 0) return;
  if (event.shiftKey || event.ctrlKey || event.metaKey) {
    return;
  }
  const movingSelection =
    mode === "move" &&
    selectedWidgetIds.value.size > 1 &&
    selectedWidgetIds.value.has(instance.id);
  if (!selectedWidgetIds.value.has(instance.id)) {
    setWidgetSelection([instance.id], instance.id);
  }
  const placement =
    instance.layouts[editorLayoutKey()] ?? instance.layouts.default;
  const pointerTarget = event.currentTarget;
  if (!placement || !(pointerTarget instanceof HTMLElement)) return;
  const widgetCell = pointerTarget.matches(".simulator-widget-cell")
    ? pointerTarget
    : pointerTarget.closest<HTMLElement>(".simulator-widget-cell");
  if (!widgetCell) return;
  const bounds = widgetCell.getBoundingClientRect();
  const gridStyle = widgetCell.parentElement
    ? window.getComputedStyle(widgetCell.parentElement)
    : undefined;
  const columnGap = Number.parseFloat(gridStyle?.columnGap ?? "0") || 0;
  const rowGap = Number.parseFloat(gridStyle?.rowGap ?? "0") || 0;
  widgetPointerEdit.value = {
    instanceId: instance.id,
    instanceIds: movingSelection ? [...selectedWidgetIds.value] : [instance.id],
    mode,
    pointerId: event.pointerId,
    startX: event.clientX,
    startY: event.clientY,
    cellWidth: (bounds.width + columnGap) / placement.cols,
    cellHeight: (bounds.height + rowGap) / placement.rows,
    original: { ...placement },
  };
  widgetPointerDragOffset.value =
    mode === "move"
      ? {
          instanceIds: movingSelection
            ? [...selectedWidgetIds.value]
            : [instance.id],
          x: 0,
          y: 0,
        }
      : null;
  pointerTarget.setPointerCapture?.(event.pointerId);
  window.addEventListener("pointermove", handleWidgetPointerMove);
  window.addEventListener("pointerup", finishWidgetPointerEdit);
  window.addEventListener("pointercancel", finishWidgetPointerEdit);
  event.preventDefault();
};

const selectPersistedWidget = (payload: {
  instanceId: string;
  additive: boolean;
}) => {
  if (!payload.additive && Date.now() < suppressWidgetClickSelectionUntil)
    return;
  if (payload.additive) toggleWidgetSelection(payload.instanceId);
  else setWidgetSelection([payload.instanceId], payload.instanceId);
};

const editorLayoutKey = () => (isLandscape.value ? "portrait" : "landscape");
const gridBoundsForLayout = (layoutKey: "portrait" | "landscape") =>
  layoutKey === "portrait"
    ? { cols: gridSize.value.cols, rows: gridSize.value.rows }
    : { cols: gridSize.value.rows, rows: gridSize.value.cols };
const editorGridBounds = computed(() => gridBoundsForLayout(editorLayoutKey()));

const persistedVacantCells = computed(() => {
  if (buttonLimits.isLoading.value) return [];
  const bounds = editorGridBounds.value;
  const firstLockedIndex = Math.max(
    0,
    Math.floor(buttonLimits.effectiveButtonLimit.value),
  );
  const cells: Array<WidgetPaletteTargetCell & { locked: boolean }> = [];
  const instances = displayedWidgetInstances.value;
  for (let index = 0; index < bounds.cols * bounds.rows; index += 1) {
    const x = index % bounds.cols;
    const y = Math.floor(index / bounds.cols);
    const occupied = instances.some((instance) => {
      const placement = persistedWidgetPlacement(instance);
      return (
        placement &&
        x >= placement.x &&
        x < placement.x + placement.cols &&
        y >= placement.y &&
        y < placement.y + placement.rows
      );
    });
    if (!occupied)
      cells.push({
        x,
        y,
        position: index + 1,
        locked: index >= firstLockedIndex,
      });
  }
  return cells;
});

const validExistingWidgetLayout = (
  layoutKey: "portrait" | "landscape",
): Record<string, WidgetPlacement> | undefined => {
  const instances = widgetEditor.value.instances();
  const bounds = gridBoundsForLayout(layoutKey);
  const placed: DeckWidgetInstance[] = [];
  const placements: Record<string, WidgetPlacement> = {};
  for (const instance of instances) {
    const placement = instance.layouts[layoutKey];
    if (!placement) return undefined;
    const definition = definitionForInstance(instance);
    if (
      placement.cols < definition.minSpan.cols ||
      placement.rows < definition.minSpan.rows ||
      placement.cols > definition.maxSpan.cols ||
      placement.rows > definition.maxSpan.rows ||
      !canPlaceWidget(placed, instance.id, layoutKey, placement, bounds)
    )
      return undefined;
    placements[instance.id] = { ...placement };
    placed.push(instance);
  }
  return placements;
};

/**
 * Widget membership belongs to the deck, while placement belongs to an
 * orientation. Older/partially-written documents can therefore only be
 * considered usable once every widget has a valid placement in both layouts.
 * Preserve a valid independent target layout first; only repack the target as
 * a last-resort recovery so a widget can never silently disappear on rotate.
 */
const repairIncompleteWidgetOrientations = async () => {
  let repaired = false;
  const layoutKeys = ["portrait", "landscape"] as const;

  for (const targetLayoutKey of layoutKeys) {
    if (validExistingWidgetLayout(targetLayoutKey)) continue;
    const sourceLayoutKey =
      targetLayoutKey === "portrait" ? "landscape" : "portrait";
    const instances = widgetEditor.value.instances();
    const targetPlacements =
      deriveResponsiveWidgetLayout(
        instances,
        sourceLayoutKey,
        targetLayoutKey,
        gridBoundsForLayout(sourceLayoutKey),
        gridBoundsForLayout(targetLayoutKey),
        definitionForInstance,
        { preserveTargetPlacements: true },
      ) ??
      deriveResponsiveWidgetLayout(
        instances,
        sourceLayoutKey,
        targetLayoutKey,
        gridBoundsForLayout(sourceLayoutKey),
        gridBoundsForLayout(targetLayoutKey),
        definitionForInstance,
      );
    if (!targetPlacements) {
      console.error(
        `Could not recover the incomplete ${targetLayoutKey} widget layout`,
      );
      return false;
    }
    const result = widgetEditor.value.replaceLayout(
      targetLayoutKey,
      targetPlacements,
      gridBoundsForLayout(targetLayoutKey),
      false,
    );
    if (!result.ok) {
      console.error(
        `Could not recover the incomplete ${targetLayoutKey} widget layout`,
        result.reason,
      );
      return false;
    }
    repaired = true;
  }

  if (!repaired) return true;
  syncEditorInstances();
  return persistWidgetEditor();
};

const syncEditorInstances = () => {
  persistedWidgetSnapshot.value = {
    ...persistedWidgetSnapshot.value,
    instances: widgetEditor.value.instances(),
  };
};

const persistWidgetEditor = async () => {
  if (!widgetEditor.value.isDirty()) return true;
  const deckId = deckStore.getActiveDeck?.id;
  if (deckId == null) return false;
  try {
    const saved = await widgetDocuments.save(
      deckId,
      widgetEditor.value.revision(),
      widgetEditor.value.instances(),
    );
    persistedWidgetSnapshot.value = saved;
    widgetEditor.value.markSaved(saved.revision, saved.instances);
    return true;
  } catch (error) {
    console.error(
      "Failed to save widget edit; reloading canonical document",
      error,
    );
    const canonical = await widgetDocuments.load(deckId);
    persistedWidgetSnapshot.value = canonical;
    widgetEditor.value.reset(canonical.instances, canonical.revision);
    return false;
  }
};

const captureWidgetBounds = () => {
  const bounds = new Map<string, DOMRect>();
  for (const element of document.querySelectorAll<HTMLElement>(
    ".simulator-widget-cell[data-editor-widget-id]",
  )) {
    const instanceId = element.dataset.editorWidgetId;
    if (instanceId) bounds.set(instanceId, element.getBoundingClientRect());
  }
  return bounds;
};

const animateWidgetReflow = (before: Map<string, DOMRect>) => {
  for (const element of document.querySelectorAll<HTMLElement>(
    ".simulator-widget-cell[data-editor-widget-id]",
  )) {
    const instanceId = element.dataset.editorWidgetId;
    const previous = instanceId ? before.get(instanceId) : undefined;
    if (!previous) continue;
    const current = element.getBoundingClientRect();
    const deltaX = previous.left - current.left;
    const deltaY = previous.top - current.top;
    if (Math.abs(deltaX) < 0.5 && Math.abs(deltaY) < 0.5) continue;
    element.animate(
      [
        { transform: `translate3d(${deltaX}px, ${deltaY}px, 0)` },
        { transform: "translate3d(0, 0, 0)" },
      ],
      { duration: 250, easing: "cubic-bezier(0.2, 0.8, 0.2, 1)" },
    );
  }
};

const notifyOrientationLayout = (
  title: string,
  message: string,
  type: "success" | "warning" | "error",
) => {
  mainStore.addNotification(
    {
      title,
      message,
      type,
      autoClose: true,
      duration: 5000,
      buttonText: "",
      buttonFunction: () => {},
      color:
        type === "success"
          ? "bg-green-500"
          : type === "warning"
            ? "bg-amber-500"
            : "bg-red-500",
    },
    "main",
  );
};

const applyWidgetEdit = async (
  edit: () => { ok: boolean },
  animateLayout = false,
  seedOppositeLayout = false,
) => {
  const previousBounds = animateLayout ? captureWidgetBounds() : null;
  const result = edit();
  if (!result.ok) return false;
  const mirrorEdit = orientationSyncEnabled.value || seedOppositeLayout;
  if (mirrorEdit) {
    const sourceLayoutKey = editorLayoutKey();
    const targetLayoutKey =
      sourceLayoutKey === "portrait" ? "landscape" : "portrait";
    const targetPlacements = deriveResponsiveWidgetLayout(
      widgetEditor.value.instances(),
      sourceLayoutKey,
      targetLayoutKey,
      gridBoundsForLayout(sourceLayoutKey),
      gridBoundsForLayout(targetLayoutKey),
      definitionForInstance,
      orientationSyncEnabled.value ? {} : { preserveTargetPlacements: true },
    );
    const completed =
      targetPlacements &&
      widgetEditor.value.replaceLayout(
        targetLayoutKey,
        targetPlacements,
        gridBoundsForLayout(targetLayoutKey),
        false,
      );
    if (!completed || !completed.ok) {
      if (seedOppositeLayout) {
        widgetEditor.value.reset(
          persistedWidgetSnapshot.value.instances,
          persistedWidgetSnapshot.value.revision,
        );
        notifyOrientationLayout(
          "Widget was not added",
          "The other orientation has no collision-free space for this widget.",
          "warning",
        );
        return false;
      }
      notifyOrientationLayout(
        "Other orientation kept unchanged",
        "This edit was saved here, but Adaptive Layout could not mirror it without a collision.",
        "warning",
      );
    }
  }
  syncEditorInstances();
  if (previousBounds) {
    await nextTick();
    animateWidgetReflow(previousBounds);
  }
  return persistWidgetEditor();
};

const syncCurrentLayoutToOther = async () => {
  if (!hasPersistedWidgetDocument.value) return;
  const sourceLayoutKey = editorLayoutKey();
  const targetLayoutKey =
    sourceLayoutKey === "portrait" ? "landscape" : "portrait";
  const targetPlacements = deriveResponsiveWidgetLayout(
    widgetEditor.value.instances(),
    sourceLayoutKey,
    targetLayoutKey,
    gridBoundsForLayout(sourceLayoutKey),
    gridBoundsForLayout(targetLayoutKey),
    definitionForInstance,
  );
  if (!targetPlacements) {
    notifyOrientationLayout(
      "Layout could not be synchronized",
      "The current arrangement cannot fit into the other orientation without collisions.",
      "warning",
    );
    return;
  }
  const result = widgetEditor.value.replaceLayout(
    targetLayoutKey,
    targetPlacements,
    gridBoundsForLayout(targetLayoutKey),
    true,
  );
  if (!result.ok) {
    notifyOrientationLayout(
      "Layout could not be synchronized",
      "The generated layout was rejected, so the other orientation was left unchanged.",
      "error",
    );
    return;
  }
  syncEditorInstances();
  if (!(await persistWidgetEditor())) return;
  notifyOrientationLayout(
    "Other orientation synchronized",
    `The ${sourceLayoutKey} layout was adapted and copied to ${targetLayoutKey}.`,
    "success",
  );
};

let vacantLegacyPrunePromise: Promise<void> | null = null;
const pruneVacantLegacyWidgets = async (preserveIds = new Set<string>()) => {
  if (vacantLegacyPrunePromise) {
    await vacantLegacyPrunePromise;
    return;
  }
  const prune = async () => {
    const current = widgetEditor.value.instances();
    const retained = current.filter(
      (instance) =>
        preserveIds.has(instance.id) || !isVacantLegacyWidget(instance),
    );
    if (retained.length === current.length) return;
    const deckId = deckStore.getActiveDeck?.id;
    if (deckId == null) return;
    try {
      const saved = await widgetDocuments.save(
        deckId,
        widgetEditor.value.revision(),
        retained,
      );
      persistedWidgetSnapshot.value = saved;
      widgetEditor.value.reset(saved.instances, saved.revision);
    } catch (error) {
      console.error(
        "Failed to remove vacant legacy slots from widget layout",
        error,
      );
      const canonical = await widgetDocuments.load(deckId);
      persistedWidgetSnapshot.value = canonical;
      widgetEditor.value.reset(canonical.instances, canonical.revision);
    }
  };
  vacantLegacyPrunePromise = prune();
  try {
    await vacantLegacyPrunePromise;
  } finally {
    vacantLegacyPrunePromise = null;
  }
};

watch(
  hasPersistedWidgetDocument,
  (hasDocument) => {
    if (!hasDocument || forceWidgetLayoutEditor.value)
      return;
    void pruneVacantLegacyWidgets();
  },
);

const addWidgetDefinition = async (definitionId: string) => {
  if (!hasPersistedWidgetDocument.value) await upgradeActiveDeckToWidgets();
  await pruneVacantLegacyWidgets();
  const definition = widgetRegistry.resolve(definitionId)?.definition;
  if (!definition) return;
  const target = widgetPaletteTargetCell.value;
  const preferredPlacement = target
    ? {
        x: target.x,
        y: target.y,
        cols: definition.defaultSpan.cols,
        rows: definition.defaultSpan.rows,
      }
    : undefined;
  const previousIds = new Set(
    widgetEditor.value.instances().map(({ id }) => id),
  );
  const saved = await applyWidgetEdit(
    () =>
      widgetEditor.value.add(
        definition,
        editorLayoutKey(),
        editorGridBounds.value,
        {},
        undefined,
        preferredPlacement,
      ),
    false,
    true,
  );
  if (!saved) {
    notifyOrientationLayout(
      "Widget does not fit there",
      "Choose another empty cell or make enough contiguous space for this widget.",
      "warning",
    );
    return;
  }
  const added = widgetEditor.value
    .instances()
    .find(({ id }) => !previousIds.has(id));
  if (added) {
    setWidgetSelection([added.id], added.id);
    isWidgetInspectorOpen.value = false;
  }
  closeWidgetPalette();
};

const restoreLegacyButton = async (
  button: SimulatorButton & { id: number },
) => {
  if (!hasPersistedWidgetDocument.value) await upgradeActiveDeckToWidgets();
  const target =
    widgetPaletteTargetCell.value ??
    persistedVacantCells.value.find(({ locked }) => !locked) ??
    persistedVacantCells.value[0];
  if (!target) {
    notifyOrientationLayout(
      "Deck is full",
      "Remove or resize a widget before restoring this button.",
      "warning",
    );
    return;
  }
  const definition =
    button.mode === "rotary"
      ? legacyRotaryDefinition
      : legacyActionButtonDefinition;
  const restored = legacyButtonToWidget(button, {
    portrait: gridSize.value.cols,
    landscape: gridSize.value.rows,
  });
  const placement = { x: target.x, y: target.y, cols: 1, rows: 1 };
  const saved = await applyWidgetEdit(
    () =>
      widgetEditor.value.insert(
        restored,
        definition,
        editorLayoutKey(),
        placement,
        editorGridBounds.value,
      ),
    false,
    true,
  );
  if (!saved) {
    notifyOrientationLayout(
      "Button could not be restored",
      "The selected cell cannot produce a collision-free layout in both orientations.",
      "warning",
    );
    return;
  }
  setWidgetSelection([restored.id], restored.id);
  closeWidgetPalette();
};

const createLegacyWidget = async (mode: ComponentMode) => {
  if (!hasPersistedWidgetDocument.value) await upgradeActiveDeckToWidgets();
  const target =
    widgetPaletteTargetCell.value ??
    persistedVacantCells.value.find(({ locked }) => !locked) ??
    persistedVacantCells.value[0];
  const slot = availableLegacySlots.value[0];
  if (!target || !slot) {
    notifyOrientationLayout(
      "No empty button slot",
      "Remove a widget or increase the grid size first.",
      "warning",
    );
    return;
  }

  let button: SimulatorButton & { id: number } = slot;
  let changedLegacyMode = false;
  try {
    if (mode === "rotary") {
      await invoke("set_button_mode", { buttonId: slot.id, mode: "rotary" });
      changedLegacyMode = true;
      await refreshDeckAfterLegacyMutation(deckStore.getActiveDeck?.id);
      const updated = deckStore.getButtonById(slot.id);
      if (updated) button = updated;
    }

    const definition =
      mode === "rotary" ? legacyRotaryDefinition : legacyActionButtonDefinition;
    const created = legacyButtonToWidget(button, {
      portrait: gridSize.value.cols,
      landscape: gridSize.value.rows,
    });
    created.definitionId = definition.id;
    created.config = { ...created.config, explicitlyAdded: true };
    const saved = await applyWidgetEdit(
      () =>
        widgetEditor.value.insert(
          created,
          definition,
          editorLayoutKey(),
          { x: target.x, y: target.y, cols: 1, rows: 1 },
          editorGridBounds.value,
        ),
      false,
      true,
    );
    if (!saved) throw new Error("widget placement was rejected");
    setWidgetSelection([created.id], created.id);
    closeWidgetPalette();
  } catch (error) {
    if (changedLegacyMode) {
      try {
        await invoke("set_button_mode", {
          buttonId: slot.id,
          mode: "click/press",
        });
        await refreshDeckAfterLegacyMutation(deckStore.getActiveDeck?.id);
      } catch (rollbackError) {
        console.error(
          "Failed to roll back the unused rotary slot",
          rollbackError,
        );
      }
    }
    console.error("Failed to create widget", error);
    notifyOrientationLayout(
      "Widget could not be added",
      "The selected cell cannot produce a collision-free layout in both orientations.",
      "warning",
    );
  }
};

const restoreRecentWidget = async (instance: DeckWidgetInstance) => {
  const target =
    widgetPaletteTargetCell.value ??
    persistedVacantCells.value.find(({ locked }) => !locked) ??
    persistedVacantCells.value[0];
  if (!target) {
    notifyOrientationLayout(
      "Deck is full",
      "Remove or resize a widget before restoring this widget.",
      "warning",
    );
    return;
  }
  const definition = definitionForInstance(instance);
  const saved = await applyWidgetEdit(
    () =>
      widgetEditor.value.insert(
        instance,
        definition,
        editorLayoutKey(),
        {
          x: target.x,
          y: target.y,
          cols: definition.defaultSpan.cols,
          rows: definition.defaultSpan.rows,
        },
        editorGridBounds.value,
      ),
    false,
    true,
  );
  if (!saved) {
    notifyOrientationLayout(
      "Widget could not be restored",
      "Choose a cell with enough contiguous space for this widget.",
      "warning",
    );
    return;
  }
  recentlyRemovedWidgets.value = recentlyRemovedWidgets.value.filter(
    ({ id }) => id !== instance.id,
  );
  setWidgetSelection([instance.id], instance.id);
  closeWidgetPalette();
};

const rememberRecentlyRemoved = (instances: DeckWidgetInstance[]) => {
  const pluginInstances = instances.filter(
    (instance) => !isLegacyWidgetInstance(instance),
  );
  if (!pluginInstances.length) return;
  const removedIds = new Set(pluginInstances.map(({ id }) => id));
  recentlyRemovedWidgets.value = [
    ...pluginInstances,
    ...recentlyRemovedWidgets.value.filter(({ id }) => !removedIds.has(id)),
  ].slice(0, 12);
};

const notifyWidgetRemoval = (count: number) => {
  mainStore.addNotification(
    {
      key: "deck-widget-removal",
      title: count === 1 ? "Widget removed" : `${count} widgets removed`,
      message: "Removed from portrait and landscape.",
      type: "info",
      autoClose: true,
      duration: 7000,
      buttonText: "Undo",
      buttonFunction: () => {
        void undoWidgetEdit();
      },
      color: "bg-blue-500",
    },
    "main",
  );
};

const removeSelectedWidget = async () => {
  const instanceId = selectedWidgetId.value;
  if (!instanceId) return;
  const instance = widgetEditor.value
    .instances()
    .find(({ id }) => id === instanceId);
  if (!instance) return;
  if (!(await applyWidgetEdit(() => widgetEditor.value.remove(instanceId))))
    return;
  rememberRecentlyRemoved([instance]);
  setWidgetSelection([]);
  isWidgetInspectorOpen.value = false;
  notifyWidgetRemoval(1);
};

const removeSelectedWidgets = async () => {
  const removedInstances = widgetEditor.value
    .instances()
    .filter((instance) => selectedWidgetIds.value.has(instance.id));
  const removableIds = removedInstances.map(({ id }) => id);
  if (!removableIds.length) return;
  if (
    !(await applyWidgetEdit(() => widgetEditor.value.removeMany(removableIds)))
  )
    return;
  rememberRecentlyRemoved(removedInstances);
  resetMenus();
  setWidgetSelection([]);
  isWidgetInspectorOpen.value = false;
  notifyWidgetRemoval(removableIds.length);
};

const findEditingLegacyWidget = async () => {
  const buttonId = editingButton.value?.id;
  if (typeof buttonId !== "number") return undefined;
  if (!hasPersistedWidgetDocument.value) await upgradeActiveDeckToWidgets();
  return widgetEditor.value
    .instances()
    .find(({ legacyButtonId }) => legacyButtonId === buttonId);
};

const removeEditingButtonFromLayout = async () => {
  const instance = await findEditingLegacyWidget();
  isButtonMenuOpen.value = false;
  if (!instance) return;
  setWidgetSelection([instance.id], instance.id);
  forceWidgetLayoutEditor.value = true;
  await removeSelectedWidget();
};

const configureSelectedWidget = async (patch: Record<string, unknown>) => {
  const instance = selectedPersistedInstance.value;
  if (!instance) return;
  if (
    !widgetEditor.value.configure(
      instance.id,
      definitionForInstance(instance),
      patch,
    )
  )
    return;
  syncEditorInstances();
  await persistWidgetEditor();
};

const bindSelectedWidgetPort = async (binding: WidgetBinding) => {
  const instance = selectedPersistedInstance.value;
  if (!instance || !widgetEditor.value.bind(instance.id, binding)) return;
  syncEditorInstances();
  await persistWidgetEditor();
};

const unbindSelectedWidgetPort = async (portId: string) => {
  const instance = selectedPersistedInstance.value;
  if (!instance || !widgetEditor.value.unbind(instance.id, portId)) return;
  syncEditorInstances();
  await persistWidgetEditor();
};

const undoWidgetEdit = async () => {
  widgetEditor.value.undo();
  syncEditorInstances();
  await persistWidgetEditor();
};

const redoWidgetEdit = async () => {
  widgetEditor.value.redo();
  syncEditorInstances();
  await persistWidgetEditor();
};

const upgradeActiveDeckToWidgets = async () => {
  const deckId = deckStore.getActiveDeck?.id;
  if (deckId == null) return;
  const legacyInstances = Array.from(widgetInstances.value.values()).filter(
    (instance) =>
      instance.legacyButtonId != null && !isVacantLegacyWidget(instance),
  );
  persistedWidgetSnapshot.value = await widgetDocuments.save(
    deckId,
    persistedWidgetSnapshot.value.revision,
    legacyInstances,
  );
  widgetEditor.value.reset(
    persistedWidgetSnapshot.value.instances,
    persistedWidgetSnapshot.value.revision,
  );
  swapy?.enable(false);
  swapy?.update();
};

const revertActiveDeckToLegacy = async () => {
  const deckId = deckStore.getActiveDeck?.id;
  if (deckId == null) return;
  await invoke("revert_deck_widget_document", { deckId });
  persistedWidgetSnapshot.value = await widgetDocuments.load(deckId);
  widgetEditor.value.reset([], 0);
  swapy?.enable(false);
  swapy?.update();
};

watch(
  () => deckStore.getActiveDeck?.id,
  async (deckId) => {
    setWidgetSelection([]);
    forceWidgetLayoutEditor.value = false;
    if (deckId == null) return;
    try {
      persistedWidgetSnapshot.value = normalizeLegacyWidgetSnapshot(
        await widgetDocuments.load(deckId),
      );
      widgetEditor.value.reset(
        persistedWidgetSnapshot.value.instances,
        persistedWidgetSnapshot.value.revision,
      );
      if (!persistedWidgetSnapshot.value.legacyBacked) {
        await repairIncompleteWidgetOrientations();
      }
    } catch (error) {
      console.error(
        "Failed to load deck widget document; using legacy projection",
        error,
      );
      persistedWidgetSnapshot.value = {
        deckId,
        revision: 0,
        updatedAt: "",
        instances: [],
        legacyBacked: true,
      };
    }
  },
  { immediate: true },
);

const widgetForButton = (button: SimulatorButton) => {
  const instanceId = `button:${button.id}`;
  return (
    widgetInstances.value.get(instanceId) ??
    legacyButtonToWidget(button, {
      portrait: gridSize.value.cols,
      landscape: gridSize.value.rows,
    })
  );
};

const widgetDefinitionFor = (button: SimulatorButton): WidgetDefinition =>
  button.mode === "rotary"
    ? legacyRotaryDefinition
    : legacyActionButtonDefinition;

const isWidgetLocked = (index: number) =>
  !buttonLimits.isLoading.value && buttonLimits.isButtonLocked(index);

const openWidgetContextMenu = (
  payload: { bounds: MenuPosition },
  button: SimulatorButton,
) => {
  if (typeof button.id === "number")
    toggleButtonMenu(payload.bounds, button.id);
};

const dispatchWidgetOperation = async (
  operation: WidgetDispatchOperation,
  intent: WidgetIntent,
) => {
  if (hasPersistedWidgetDocument.value) {
    const deckId = deckStore.getActiveDeck?.id;
    if (deckId == null) return;
    const normalizedIntent =
      operation.type === "execute"
        ? { ...intent, value: operation.value }
        : intent;
    const currentLayoutKey = editorLayoutKey();

    const sendIntent = async (rev: number) => {
      return await invoke<{
        success: boolean;
        error?: string;
        documentRevision?: number;
      }>("execute_deck_widget_intent", {
        payload: {
          action: "deck_widget_intent",
          protocolVersion: 1,
          deckId,
          documentRevision: rev,
          layoutKey: currentLayoutKey,
          intent: normalizedIntent,
          correlationId:
            globalThis.crypto?.randomUUID?.() ??
            `${normalizedIntent.instanceId}:${normalizedIntent.sequence}:${Date.now()}`,
        },
      });
    };

    let result = await sendIntent(persistedWidgetSnapshot.value.revision);
    if (result && !result.success && result.error === "stale_revision") {
      try {
        const fresh = normalizeLegacyWidgetSnapshot(
          await widgetDocuments.load(deckId),
        );
        persistedWidgetSnapshot.value = fresh;
        widgetEditor.value.reset(fresh.instances, fresh.revision);
        result = await sendIntent(fresh.revision);
      } catch (err) {
        console.error("Failed to retry widget intent after stale revision", err);
      }
    }
    return;
  }

  const buttonId = operation.instance.legacyButtonId;

  if (operation.type === "repeat_start") {
    if (buttonId == null) return;
    const button = sortedButtons.value.find(
      (candidate) => candidate.id === buttonId,
    );
    if (!button) return;
    handleButtonPress(button);
    return;
  }
  if (operation.type === "repeat_stop") {
    if (buttonId == null) return;
    const button = sortedButtons.value.find(
      (candidate) => candidate.id === buttonId,
    );
    if (!button) return;
    handleButtonRelease(button);
    return;
  }

  if (operation.type !== "execute") return;
  if (typeof operation.binding.action !== "string") return;
  const value =
    typeof operation.value === "number" ? operation.value : undefined;
  await useSendCommand(operation.binding.action, value, buttonId);
};

const widgetInteractionRouter = useWidgetInteractionRouter({
  getInstance: (instanceId) => activeWidgetInstances.value.get(instanceId),
  canInteract: (instance) =>
    isInteractiveMode.value && !isPersistedWidgetLocked(instance),
  dispatch: dispatchWidgetOperation,
});

const handleWidgetIntent = (intent: WidgetIntent) => {
  void widgetInteractionRouter.route(intent);
};

// Removed old fetchFeatureStatus - now using useButtonLimits composable

let unlistenWidgetDelta: (() => void) | null = null;

onMounted(async () => {
  try {
    await installedDeckWidgets.refresh();
  } catch (error) {
    console.error("Failed to load installed deck widget definitions", error);
  }
  emitTo("main", "toggle_is_simulator", true);

  try {
    unlistenWidgetDelta = await listen<{ deckId?: number }>(
      "deck_widget_delta",
      async (event) => {
        const activeDeckId = deckStore.getActiveDeck?.id;
        if (
          activeDeckId != null &&
          (!event.payload?.deckId || event.payload.deckId === activeDeckId)
        ) {
          try {
            const fresh = normalizeLegacyWidgetSnapshot(
              await widgetDocuments.load(activeDeckId),
            );
            persistedWidgetSnapshot.value = fresh;
            widgetEditor.value.reset(fresh.instances, fresh.revision);
          } catch (err) {
            console.error("Failed to reload widget document on delta", err);
          }
        }
      },
    );
  } catch (err) {
    console.warn("Could not attach deck_widget_delta listener", err);
  }

  // Button limits are now handled by useButtonLimits composable
  const swapyContainer = document.querySelector("#swapy-container");
  if (!(swapyContainer instanceof HTMLElement)) {
    console.error(
      "Cannot initialize deck sorting: #swapy-container was not found",
    );
    return;
  }
  swapy = createSwapy(swapyContainer, {
    enabled: false,
    animation: "dynamic",
    swapMode: "drop",
    // Your config options
  });

  swapy.onSwap(async (event) => {
    // Swapy's dynamic animation must finish before the persisted deck refresh
    // rebuilds the slot DOM, otherwise the swap visibly snaps or flickers.
    await new Promise<void>((resolve) => window.setTimeout(resolve, 250));
    await saveChangePosition(
      event.draggingItem,
      event.swappedWithItem,
      event.fromSlot,
      event.toSlot,
    );
  });
});

onUnmounted(() => {
  unlistenWidgetDelta?.();
  unlistenWidgetDelta = null;
  swapy?.destroy();
  swapy = null;
  window.removeEventListener("pointermove", handleWidgetPointerMove);
  window.removeEventListener("pointerup", finishWidgetPointerEdit);
  window.removeEventListener("pointercancel", finishWidgetPointerEdit);
  window.removeEventListener("pointermove", handleGridMarqueeMove);
  window.removeEventListener("pointerup", finishGridMarqueeSelection);
  window.removeEventListener("pointercancel", finishGridMarqueeSelection);
});

const saveChangePosition = async (
  swappingId: string,
  swappedId: string,
  from: string,
  to: string,
) => {
  try {
    await invoke("change_button_position", {
      swappingButtonId: Number(swappingId),
      swappedButtonId: Number(swappedId),
      from: Number(from),
      to: Number(to),
    });
    const deck = await invoke<Deck>("get_active_deck");
    await deckStore.updateActiveDeck(deck);
    await emitTo("main", "update_active_deck", deckStore.getActiveDeck);
    swapy?.update();

    if (hasPersistedWidgetDocument.value) {
      const synchronizedInstances = persistedWidgetSnapshot.value.instances.map(
        (instance) => {
          if (!isLegacyWidgetInstance(instance)) return instance;
          const projection = widgetInstances.value.get(instance.id);
          if (!projection) return instance;
          return {
            ...instance,
            config: {
              ...instance.config,
              position: projection.config.position,
            },
            layouts: projection.layouts,
          };
        },
      );
      persistedWidgetSnapshot.value = await widgetDocuments.save(
        deck.id,
        persistedWidgetSnapshot.value.revision,
        synchronizedInstances,
      );
      widgetEditor.value.reset(
        persistedWidgetSnapshot.value.instances,
        persistedWidgetSnapshot.value.revision,
      );
    }
  } catch (error) {
    console.error("Error invoking command:", error);
  }
};

// const saveDeck = () => {

// }

// async function setupWindow() {
//   await getCurrentWebviewWindow().show();
// }

// WebSocket message handlers for backend repeat notifications
onMounted(() => {
  if ($socket.value) {
    const handleWebSocketMessage = (event: MessageEvent) => {
      try {
        const data = JSON.parse(event.data);

        if (data.action === "button_repeat_started") {
          const buttonId = data.buttonId;
          console.log(
            "🟢 Backend notified: repeat started for button",
            buttonId,
          );
          repeatingButtons.value.add(buttonId);
        }

        // Note: We don't handle 'button_repeat_stopped' because we remove the indicator
        // immediately on release for better UX responsiveness
      } catch (error) {
        // Ignore non-JSON messages
      }
    };

    $socket.value.addEventListener("message", handleWebSocketMessage);

    // Cleanup on unmount
    onUnmounted(() => {
      if ($socket.value) {
        $socket.value.removeEventListener("message", handleWebSocketMessage);
      }
    });
  }
});

const AsyncLayout = defineAsyncComponent(
  () => import("@desktop/layouts/deck.vue"),
);

const sendExampleComponent = () => {
  if ($socket.value?.readyState === WebSocket.OPEN) {
    $socket.value.send(
      JSON.stringify({
        action: "component",
        payload: "test component",
      }),
    );
  }
};

const isLandscape = ref(false);
const rotateComponent = async () => {
  if (hasPersistedWidgetDocument.value) {
    const sourceLayoutKey = editorLayoutKey();
    const targetLayoutKey =
      sourceLayoutKey === "portrait" ? "landscape" : "portrait";
    const existingTarget = validExistingWidgetLayout(targetLayoutKey);
    const placements =
      existingTarget ??
      deriveResponsiveWidgetLayout(
        widgetEditor.value.instances(),
        sourceLayoutKey,
        targetLayoutKey,
        gridBoundsForLayout(sourceLayoutKey),
        gridBoundsForLayout(targetLayoutKey),
        definitionForInstance,
        { preserveTargetPlacements: true },
      );
    if (!placements) {
      console.error(
        "Could not initialize the rotated deck without changing its saved layout",
      );
      return;
    }
    if (existingTarget) {
      isLandscape.value = !isLandscape.value;
      return;
    }
    const result = widgetEditor.value.replaceLayout(
      targetLayoutKey,
      placements,
      gridBoundsForLayout(targetLayoutKey),
      false,
    );
    if (!result.ok) {
      console.error(
        "Could not synchronize the rotated widget layout",
        result.reason,
      );
      return;
    }
    syncEditorInstances();
    if (!(await persistWidgetEditor())) return;
  }
  isLandscape.value = !isLandscape.value;
};

const resetButton = async () => {
  const button = editingButton.value;
  if (!button) return;
  console.log(button.position);
  const result = await invoke("reset_button", {
    buttonId: button.id,
  });
  resetButtonOptions();
  refreshIcons();
  console.log(result);
  const deck = await invoke<Deck>("get_active_deck");
  console.log(deck);
  await deckStore.updateActiveDeck(deck);
  await emitTo("main", "update_active_deck", deckStore.getActiveDeck);

  const updatedButton = deckStore.getButtonById(button.id);
  if (updatedButton) await persistReconciledLegacyWidget(updatedButton);

  swapy?.update();
};
</script>

<style lang="scss" scoped>
@use "../../components/Modal/modalTheme.scss" as *;

/* Rotation container simulating the phone screen */

$available-height: calc(100vh - 12rem);

.rotatable-container.landscape {
  width: $available-height; /* Use the calculated height */
}

.rotatable-container.portrait {
  height: $available-height; /* Use the same value for width */
}

.menu-move, /* apply transition to moving elements */
  .menu-enter-active,
  .menu-leave-active {
  transition: all 0.3s ease;
}

.menu-enter-from {
  opacity: 0;
  transform: translateY(-30px);
}

.menu-leave-to {
  opacity: 0;
  transform: translateY(60px);
}

/* ensure leaving items are taken out of layout flow so that moving
     animations can be calculated correctly. */
.menu-leave-active {
  position: absolute;
  width: 100%;
}

/* Custom pulse animation for repeat indicator */
@keyframes repeat-pulse {
  0% {
    transform: scale(1);
    opacity: 1;
  }
  50% {
    transform: scale(1.2);
    opacity: 0.8;
  }
  100% {
    transform: scale(1);
    opacity: 1;
  }
}

.repeat-indicator {
  animation: repeat-pulse 0.6s ease-in-out infinite;
}

.simulator-editor-shell {
  min-height: 0;
}

.simulator-editor-header {
  flex: 0 0 auto;
}

.simulator-widget-cell {
  display: grid;
  place-items: center;
  container-type: size;
  cursor: grab;
  touch-action: none;
  will-change: transform;
}

.simulator-widget-cell:active {
  cursor: grabbing;
}

.simulator-vacant-cell {
  z-index: 1;
  display: grid;
  place-items: stretch;
}

.simulator-vacant-cell__add {
  display: grid;
  width: 100%;
  height: 100%;
  place-items: center;
  border: 1px dashed rgb(56 189 248 / 0.28);
  border-radius: 0.7rem;
  background: rgb(14 165 233 / 0.025);
  color: rgb(125 211 252 / 0);
  font-size: 1.25rem;
  line-height: 1;
  transition:
    border-color 120ms ease,
    background 120ms ease,
    color 120ms ease;
}

.simulator-vacant-cell__add:hover,
.simulator-vacant-cell__add:focus-visible {
  border-color: rgb(56 189 248 / 0.8);
  background: rgb(14 165 233 / 0.12);
  color: rgb(186 230 253);
  outline: none;
}

.simulator-widget-cell :deep(.deck-widget-host) {
  width: var(--widget-render-width);
  height: var(--widget-render-height);
}

.simulator-widget-cell--rotary :deep(.deck-widget-host) {
  display: grid;
  place-items: center;
}

.simulator-widget-cell--rotary :deep(.rotary-container) {
  width: min(var(--widget-render-width), var(--widget-render-height));
  height: min(var(--widget-render-width), var(--widget-render-height));
}

.widget-resize-handle {
  position: absolute;
  z-index: 40;
  width: 0.7rem;
  height: 0.7rem;
  border: 1px solid white;
  border-radius: 999px;
  background: rgb(59 130 246);
  box-shadow: 0 1px 5px rgb(0 0 0 / 0.65);
  touch-action: none;
}

.widget-resize-handle--nw {
  top: -0.3rem;
  left: -0.3rem;
  cursor: nwse-resize;
}

.widget-resize-handle--ne {
  top: -0.3rem;
  right: -0.3rem;
  cursor: nesw-resize;
}

.widget-resize-handle--sw {
  bottom: -0.3rem;
  left: -0.3rem;
  cursor: nesw-resize;
}

.widget-resize-handle--se {
  right: -0.3rem;
  bottom: -0.3rem;
  cursor: nwse-resize;
}

.simulator-widget-cell .widget-resize-handle--nw,
.simulator-widget-cell .widget-resize-handle--ne {
  top: calc(var(--widget-render-offset-y) - 0.3rem);
}

.simulator-widget-cell .widget-resize-handle--sw,
.simulator-widget-cell .widget-resize-handle--se {
  bottom: calc(var(--widget-render-offset-y) - 0.3rem);
}

.simulator-widget-cell .widget-resize-handle--nw,
.simulator-widget-cell .widget-resize-handle--sw {
  left: calc(var(--widget-render-offset-x) - 0.3rem);
}

.simulator-widget-cell .widget-resize-handle--ne,
.simulator-widget-cell .widget-resize-handle--se {
  right: calc(var(--widget-render-offset-x) - 0.3rem);
}

.simulator-widget-inspector {
  min-width: 0;
  overflow: visible;
}

.simulator-widget-marquee {
  position: absolute;
  z-index: 60;
  border: 1px solid rgb(96 165 250);
  border-radius: 0.25rem;
  background: rgb(59 130 246 / 0.18);
  box-shadow: inset 0 0 0 1px rgb(255 255 255 / 0.12);
  pointer-events: none;
}

.simulator-widget-selection-toolbar {
  position: fixed;
  top: 3.75rem;
  left: 50%;
  z-index: 1900;
  display: flex;
  align-items: center;
  gap: 0.65rem;
  max-width: calc(100vw - 1rem);
  border: 1px solid hsl(var(--border2) / 0.9);
  border-radius: 999px;
  background: hsl(var(--background) / 0.98);
  padding: 0.35rem 0.4rem 0.35rem 0.8rem;
  box-shadow: 0 14px 38px rgb(0 0 0 / 0.42);
  color: hsl(var(--text));
  transform: translateX(-50%);
}

.simulator-widget-selection-toolbar strong {
  white-space: nowrap;
  font-size: 0.75rem;
}

.simulator-widget-selection-toolbar span {
  overflow: hidden;
  color: hsl(var(--text3));
  font-size: 0.65rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.widget-selection-toolbar-enter-active,
.widget-selection-toolbar-leave-active {
  transition:
    opacity 0.15s ease,
    transform 0.15s ease;
}

.widget-selection-toolbar-enter-from,
.widget-selection-toolbar-leave-to {
  opacity: 0;
  transform: translate(-50%, -0.4rem);
}

.simulator-widget-palette-backdrop {
  position: fixed;
  inset: 0;
  z-index: 2000;
  background: transparent;
}

.simulator-widget-palette {
  position: fixed;
  width: min(13.5rem, calc(100vw - 1rem));
  max-height: min(24rem, calc(100vh - 1rem));
  overflow: hidden;
}

.simulator-widget-palette__content {
  display: flex;
  min-height: 0;
  gap: 0.1rem;
  overflow: visible;
}

.simulator-widget-palette__close {
  display: grid;
  width: 1.5rem;
  height: 1.5rem;
  flex: 0 0 auto;
  margin-left: auto;
  place-items: center;
  border-radius: 999px;
  color: hsl(var(--text3));
  font-size: 0.9rem;
  line-height: 1;
}

.simulator-widget-palette__close:hover {
  background: hsl(var(--hover));
  color: hsl(var(--text));
}

.simulator-widget-palette__label {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}

.simulator-widget-palette .btn-ctx-row:disabled {
  cursor: not-allowed;
  opacity: 0.4;
}

.simulator-widget-palette .btn-ctx-row:disabled:hover {
  background: transparent;
}

.simulator-widget-palette__meta {
  max-width: 4rem;
  flex: 0 1 auto;
  overflow: hidden;
  color: hsl(var(--text3));
  font-size: 0.6rem;
  text-overflow: ellipsis;
  text-transform: capitalize;
  white-space: nowrap;
}

:global(.simulator-widget-palette-floating-menu .btn-ctx-subrow:disabled) {
  cursor: not-allowed;
  opacity: 0.4;
}

:global(
  .simulator-widget-palette-floating-menu .btn-ctx-subrow:disabled:hover
) {
  background: transparent;
}

.simulator-widget-palette__empty {
  margin: 0;
  padding: 0.55rem 0.6rem;
  color: hsl(var(--text3));
  font-size: 0.72rem;
}

.widget-palette-enter-active,
.widget-palette-leave-active {
  transition: opacity 0.16s ease;
}

.widget-palette-enter-active .simulator-widget-palette,
.widget-palette-leave-active .simulator-widget-palette {
  transition:
    opacity 0.16s ease,
    transform 0.16s ease;
}

.widget-palette-enter-from,
.widget-palette-leave-to,
.widget-palette-enter-from .simulator-widget-palette,
.widget-palette-leave-to .simulator-widget-palette {
  opacity: 0;
}

.widget-palette-enter-from .simulator-widget-palette,
.widget-palette-leave-to .simulator-widget-palette {
  transform: translateY(0.5rem) scale(0.98);
}

/* Button Context Menu - Theme Compliant Glassmorphic Styling */
.btn-ctx-popover {
  position: fixed;
  z-index: 1000;
  min-width: 13.5rem;
  border-radius: 0.85rem;
  border: 1px solid hsl(var(--border2) / 0.85);
  background: hsl(var(--background) / 0.95);
  box-shadow:
    0 20px 48px -8px hsl(var(--text) / 0.25),
    0 0 0 1px hsl(var(--border2) / 0.4),
    inset 0 1px 0 hsl(var(--text) / 0.08);
  backdrop-filter: blur(24px);
  -webkit-backdrop-filter: blur(24px);
  display: flex;
  flex-direction: column;
  overflow: visible;
  user-select: none;
  color: hsl(var(--text));
}

.btn-ctx-header {
  display: flex;
  align-items: center;
  gap: 0.55rem;
  padding: 0.55rem 0.75rem;
  border-bottom: 1px solid hsl(var(--border2) / 0.7);
  background: hsl(
    var(--editor-surface-soft, var(--background2, 210 33% 98%)) / 0.65
  );
  border-top-left-radius: 0.85rem;
  border-top-right-radius: 0.85rem;
}

.btn-ctx-pill {
  font-size: 0.65rem;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-weight: 800;
  padding: 0.12rem 0.42rem;
  border-radius: 0.35rem;
  background: hsl(var(--button) / 0.18);
  color: hsl(var(--button));
  border: 1px solid hsl(var(--button) / 0.35);
  letter-spacing: 0.04em;
}

.btn-ctx-title-col {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.btn-ctx-header-title {
  font-size: 0.78rem;
  font-weight: 700;
  color: hsl(var(--text));
  line-height: 1.2;
}

.btn-ctx-header-sub {
  font-size: 0.65rem;
  color: hsl(var(--text3));
  text-transform: capitalize;
}

.btn-ctx-body {
  padding: 0.32rem;
  display: flex;
  flex-direction: column;
  gap: 0.12rem;
}

.btn-ctx-row {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  padding: 0.45rem 0.6rem;
  border-radius: 0.55rem;
  font-size: 0.8rem;
  font-weight: 550;
  color: hsl(var(--text));
  border: none;
  background: transparent;
  cursor: pointer;
  text-align: left;
  transition:
    background 0.15s ease,
    color 0.15s ease;

  &:hover,
  &--open {
    background: hsl(var(--hover));
    color: hsl(var(--text));
  }

  &--danger {
    &:hover {
      background: hsl(var(--error) / 0.14);
      color: hsl(var(--error));

      .btn-ctx-icon {
        color: hsl(var(--error)) !important;
      }
    }
  }
}

.btn-ctx-icon {
  width: 1.15rem;
  height: 1.15rem;
  flex-shrink: 0;
}

.btn-ctx-label {
  flex: 1 1 auto;
  white-space: nowrap;
}

.btn-ctx-divider {
  height: 1px;
  background: hsl(var(--border2) / 0.6);
  margin: 0.18rem 0.2rem;
}

.btn-ctx-submenu {
  position: absolute;
  top: -0.25rem;
  left: calc(100% + 0.35rem);
  min-width: 11rem;
  padding: 0.32rem;
  border-radius: 0.8rem;
  border: 1px solid hsl(var(--border2) / 0.85);
  background: hsl(var(--background) / 0.96);
  box-shadow:
    0 20px 48px -8px hsl(var(--text) / 0.25),
    0 0 0 1px hsl(var(--border2) / 0.4),
    inset 0 1px 0 hsl(var(--text) / 0.08);
  backdrop-filter: blur(24px);
  -webkit-backdrop-filter: blur(24px);
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
  z-index: 1050;

  // The submenu is offset from its parent. Keep that visual spacing while
  // extending the hover hit-area across it so crossing the gap cannot close
  // and immediately reopen the menu.
  &::before {
    content: "";
    position: absolute;
    inset-block: 0;
    right: 100%;
    width: 0.45rem;
  }

  &--left {
    left: auto;
    right: calc(100% + 0.35rem);

    &::before {
      right: auto;
      left: 100%;
    }
  }
}

.btn-ctx-subrow {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.42rem 0.55rem;
  border-radius: 0.5rem;
  font-size: 0.78rem;
  font-weight: 550;
  color: hsl(var(--text));
  border: 1px solid transparent;
  background: transparent;
  cursor: pointer;
  text-align: left;
  transition:
    background 0.15s ease,
    color 0.15s ease,
    border-color 0.15s ease;

  &:hover {
    background: hsl(var(--hover));
    color: hsl(var(--text));
  }

  &--active {
    background: hsl(var(--button) / 0.16);
    color: hsl(var(--button));
    border-color: hsl(var(--button) / 0.32);
    font-weight: 600;
  }
}

.btn-ctx-sublabel {
  flex: 1 1 auto;
  white-space: nowrap;
}

.btn-ctx-dropmenu {
  position: fixed;
  z-index: 1000;
  min-width: 10rem;
  padding: 0.35rem;
  border-radius: 0.8rem;
  border: 1px solid hsl(var(--border2) / 0.85);
  background: hsl(var(--background) / 0.96);
  box-shadow:
    0 20px 48px -8px hsl(var(--text) / 0.25),
    inset 0 1px 0 hsl(var(--text) / 0.08);
  backdrop-filter: blur(24px);
  -webkit-backdrop-filter: blur(24px);
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  color: hsl(var(--text));
}

.btn-ctx-dropmenu-header {
  padding: 0.35rem 0.5rem 0.2rem;
  border-bottom: 1px solid hsl(var(--border2) / 0.6);
}

.btn-ctx-fade-enter-active,
.btn-ctx-fade-leave-active,
.btn-ctx-sub-enter-active,
.btn-ctx-sub-leave-active {
  transition:
    opacity 0.14s cubic-bezier(0.16, 1, 0.3, 1),
    transform 0.14s cubic-bezier(0.16, 1, 0.3, 1);
}

.btn-ctx-fade-enter-from,
.btn-ctx-fade-leave-to,
.btn-ctx-sub-enter-from,
.btn-ctx-sub-leave-to {
  opacity: 0;
  transform: scale(0.96) translateY(-4px);
}
</style>
