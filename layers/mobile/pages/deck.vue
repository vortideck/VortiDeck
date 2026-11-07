<!-- eslint-disable @typescript-eslint/no-unused-vars -->
<!-- eslint-disable @typescript-eslint/no-unused-vars -->
<!-- eslint-disable @typescript-eslint/no-unused-vars -->
<template lang="pug">

  // Connection status overlay
  div(v-if="connectionStore.getStatus !== 'paired'" class="fixed inset-0 bg-black/80 backdrop-blur z-50 flex items-center justify-center pointer-events-auto")
    div(class="text-center p-8 bg-gray-900/90 rounded-xl border border-red-500/30 pointer-events-auto" @click.stop)
      div(v-if="connectionStore.getStatus === 'connecting'" class="flex flex-col items-center gap-4")
        div(class="w-12 h-12 border-4 border-blue-500/30 border-t-blue-500 rounded-full animate-spin")
        h2(class="text-xl font-semibold text-white") {{ isRelayMode ? 'Connecting to Desktop...' : 'Connecting to Host...' }}
        p(class="text-gray-300") {{ isRelayMode ? 'Connecting via cloud relay' : 'Please wait while we establish connection' }}
        button(@click.stop="$router.push('/')" class="mt-4 px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition pointer-events-auto")
          | Go Back
  
     
        // Relay mode - show waiting message
        div(v-if="isRelayMode" class="flex gap-3 mt-4")
          button(@click.stop="$router.push('/')" class="px-6 py-2 bg-purple-600 text-white rounded-lg hover:bg-purple-700 transition pointer-events-auto")
            | Back to Home
  
        // Direct mode - show QR code option
        div(v-else class="flex gap-3 mt-4")
          button(@click.stop="$router.push('/qr')" class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition pointer-events-auto")
            | Scan QR Code
          button(@click.stop="$router.push('/')" class="px-6 py-2 bg-gray-600 text-white rounded-lg hover:bg-gray-700 transition pointer-events-auto")
            | Go Back
  
  // Main deck interface (only show when connected)
  component(v-if="connectionStore.getStatus === 'paired'" :is="AsyncLayout"  :gridSettings="deckStore.getActiveDeck?.settings"  :isNative="true"  
          @touchstart="handleTouchStart" 
          @touchmove="handleTouchMove" 
          @touchend="handleTouchEnd") 
    template(v-slot:page)

      div(
        v-if="hasSyncedWidgetDocument"
        v-for="instance in syncedWidgets.instances.value"
        :key="instance.id"
        class="min-h-0 min-w-0 h-full w-full"
        :style="syncedWidgetPlacementStyle(instance)"
      )
        WidgetHost(
          :instance="instance"
          :definition="definitionForInstance(instance)"
          mode="interactive"
          :runtime-state="syncedWidgets.runtime.get(instance.id)"
          :theme="deckStore.getActiveTheme"
          @intent="sendWidgetIntent"
        )
          WidgetRenderer(
            :instance="instance"
            :definition="definitionForInstance(instance)"
            :runtime-state="syncedWidgets.runtime.get(instance.id)"
            mode="interactive"
            :interactive="true"
            :theme="deckStore.getActiveTheme"
            @intent="sendWidgetIntent"
          )

      div(v-if="!hasSyncedWidgetDocument" v-for="(button, index) in sortedButtons"  :class='orientation === "landscape"? "h-full  " : "h-auto max-h-full  "' class="aspect-square items-center justify-center flex " :key="`${button.id}-${button.icon?.substring(0, 50) || 'no-icon'}`")
        div(class="aspect-square relative w-full h-full" :data-button-id="button.id")
          WidgetHost(
            :instance="widgetForButton(button)"
            :definition="widgetDefinitionFor(button)"
            mode="interactive"
            :locked="isWidgetLocked(index)"
            :theme="deckStore.getActiveTheme"
          )
            component(@touchstart="handleButtonPress(button, $event)" @touchend="handleButtonRelease(button)" @touchcancel="handleButtonRelease(button)" @touchmove="handleButtonMove(button, $event)" v-if="button.mode === null || button.mode === 'click/press'" :is="AsyncClickComponent" :isReactive="true" :data="button" :isAlt="deckStore.getIsAlt" :theme="deckStore.getActiveTheme.components.clickButton" :padding="deckStore.getActiveDeck.settings.buttons_padding" :image-fit="button.icon_fit ?? 'cover'" :iconUrl="getIconUrl(button)" )

            div(v-if="button.execution_mode === 'repeat_while_pressing' && repeatingButtons.has(button.id)"
                class="absolute top-1 right-1 w-3 h-3 bg-green-400 rounded-full shadow-lg border-2 border-white z-10 repeat-indicator"
                :style="getRepeatIndicatorStyle(button)")

            component(@valueChange="handleValueChange(button, $event)" v-else-if="button.mode === 'rotary'" :is="AsyncRotaryComponent" :isReactive="true" :displayValue="false" :data="button" :theme="deckStore.getActiveTheme.components.rotaryButton" behavior="free" class="h-full" :iconUrl="getIconUrl(button)" )

            template(#overlay)
              DeckLockOverlay(
                :position="button.position"
                :is-locked="isWidgetLocked(index)"
                :is-mobile="true"
                :border-radius="overlayBorderRadius"
                @locked-touch-start="handleLockedButtonPress(button, $event)"
                @locked-touch-end="handleLockedButtonRelease(button, $event)"
                @locked-touch-move="handleLockedButtonMove(button, $event)"
                @locked-click="handleLockedButtonClick(button)"
              )
  
  </template>

<script setup lang="ts">
import { keepScreenOn } from "tauri-plugin-keep-screen-on-api";
import FxemojiVideogame from "~icons/fxemoji/videogame?width=70px&height=70px";
import IconParkMessage from "~icons/icon-park/message?width=48px&height=48px";
import {
  vibrate,
  impactFeedback,
  notificationFeedback,
  selectionFeedback,
} from "@tauri-apps/plugin-haptics";
import { useDeckStore } from "@/stores/Deck";
import { useConnectionStore } from "@/stores/Connection";
import { useButtonLimits } from "@/composables/useButtonLimits";
import {
  onMounted,
  onUnmounted,
  ref,
  computed,
  watch,
  defineAsyncComponent,
} from "vue";
import { invoke } from "@tauri-apps/api/core";
import WidgetHost from "@base/components/Deck/Widget/Host.vue";
import WidgetRenderer from "@base/components/Deck/Widget/Renderer.vue";
import { useSyncedDeckWidgets } from "@base/composables/useSyncedDeckWidgets";
import { useWidgetRegistry } from "@base/composables/useWidgetRegistry";
import type {
  DeckWidgetInstance,
  WidgetDefinition,
  WidgetIntent,
} from "@base/types/deckWidget";
import {
  legacyActionButtonDefinition,
  legacyButtonToWidget,
  legacyWidgetPlacementForLayout,
  legacyRotaryDefinition,
} from "@base/utils/legacyDeckWidget";

const { $socket, $isWebSocketConnected, $sendSignedMessage } = useNuxtApp();
const connectionStore = useConnectionStore();
const isRelayMode = computed(
  () => connectionStore.getConnectionMode === "relay",
);

keepScreenOn(true);

const orientation = ref();

// Button execution state management
const buttonIntervals = ref(new Map());
const buttonTimeouts = ref(new Map());
const pressedButtons = ref(new Set());
const repeatingButtons = ref(new Set());
const vibrationIntervals = ref(new Map());

// Gesture detection state
const isGesturing = ref(false);
const activeTouches = ref(0);
const currentlyPressedButtons = ref(new Set());

// Button limits for lock overlays
const buttonLimits = useButtonLimits(true); // true = mobile

// ✅ IMPROVED: Safe button press handler with gesture detection
// NOTE: This handler is for UNLOCKED buttons only
// Locked buttons are handled by handleLockedButtonPress via the overlay
const handleButtonPress = async (button, event) => {
  const buttonId = button.id;

  // Add this button to currently pressed set
  currentlyPressedButtons.value.add(buttonId);

  // For repeat buttons, handle immediately without delay
  const executionMode = button.execution_mode || "run_once";
  if (executionMode === "repeat_while_pressing") {
    // Visual feedback for repeat buttons (immediate since no gesture detection)
    const buttonElement = document.querySelector(
      `[data-button-id="${buttonId}"]`,
    );
    if (buttonElement) {
      addVisualFeedback(buttonElement);
    }
    // Remove from currently pressed since repeat buttons handle their own state
    currentlyPressedButtons.value.delete(buttonId);
    // Execute repeat button immediately (old behavior)
    executeButtonAction(button);
    return;
  }

  // For non-repeat buttons, use delay to detect gestures
  // 50ms delay allows time to detect swipe gestures (which trigger at 50px movement)
  setTimeout(() => {
    // If multiple buttons are pressed simultaneously, it's a gesture
    if (currentlyPressedButtons.value.size > 1) {
      console.log(
        "👆 Multiple buttons pressed simultaneously, ignoring (gesture detected)",
      );
      console.log(
        "Pressed buttons:",
        Array.from(currentlyPressedButtons.value),
      );
      isGesturing.value = true;
      // Clear all pressed buttons since it's a gesture
      currentlyPressedButtons.value.clear();
      return;
    }

    // Check if gesture is already in progress
    if (isGesturing.value) {
      console.log("👆 Gesture in progress, ignoring button press");
      return;
    }

    // Check if button was already released (quick tap)
    if (!currentlyPressedButtons.value.has(buttonId)) {
      console.log("👆 Button already released, executing action");
    }

    // ONLY show visual feedback if it's a valid single button press
    const buttonElement = document.querySelector(
      `[data-button-id="${buttonId}"]`,
    );
    if (buttonElement) {
      addVisualFeedback(buttonElement);
    }

    // Execute the button action
    executeButtonAction(button);
  }, 50);

  // Immediately return to avoid continuing with execution
  return;
};

// Add immediate visual feedback to button
const addVisualFeedback = (element) => {
  if (!element) return;

  // Add scale animation
  element.style.transform = "scale(0.95)";
  element.style.transition = "transform 0.1s ease-out";

  setTimeout(() => {
    element.style.transform = "scale(1)";
    setTimeout(() => {
      // Clean up styles after animation
      element.style.transform = "";
      element.style.transition = "";
    }, 100);
  }, 100);
};

// Separate function to execute button actions
const executeButtonAction = async (button) => {
  if (connectionStore.getStatus !== "paired" || !$socket.value) {
    console.warn("⚠️ Cannot send command: WebSocket not connected");
    return;
  }

  if (!button.action) return;

  const buttonId = button.id;
  const executionMode = button.execution_mode || "run_once";
  console.log("👆 Button pressed:", buttonId, "execution_mode:", executionMode);

  // Add immediate visual feedback for all button executions
  const buttonElement = document.querySelector(
    `[data-button-id="${buttonId}"]`,
  );
  if (!buttonElement) {
    // Fallback: find button by any method available
    const allButtons = document.querySelectorAll(".aspect-square.relative");
    const buttonIndex = sortedButtons.value.findIndex((b) => b.id === buttonId);
    if (buttonIndex >= 0 && allButtons[buttonIndex]) {
      addVisualFeedback(allButtons[buttonIndex]);
    }
  } else {
    addVisualFeedback(buttonElement);
  }

  // Handle different execution modes
  switch (executionMode) {
    case "run_once":
      console.log("▶️ Run once mode - executing:", button.action);
      // Vibrate for single actions
      try {
        await vibrate(50);
      } catch (error) {
        console.warn("Vibration failed:", error);
      }
      useSendCommand(button.action);
      break;

    case "repeat_while_pressing":
      // Prevent multiple presses on the same button
      if (pressedButtons.value.has(buttonId)) {
        console.log("⚠️ Button already pressed, ignoring:", buttonId);
        return;
      }

      // Initial vibration for repeat button press
      try {
        await vibrate(50);
        console.log("📳 Initial repeat button press vibration");
      } catch (error) {
        console.warn("Initial vibration failed:", error);
      }

      pressedButtons.value.add(buttonId);
      console.log(
        "✅ Button press detected, sending start_button_repeat to backend:",
        buttonId,
      );

      // Send WebSocket message to backend to start repeat
      if ($socket.value?.readyState === WebSocket.OPEN) {
        const message = {
          action: "start_button_repeat",
          buttonId: buttonId,
        };
        $sendSignedMessage(message);
        console.log(
          "📤 Sent start_button_repeat message to backend (E2EE signed)",
        );
      } else {
        console.log("❌ WebSocket not connected");
      }
      break;

    case "switch":
      console.log("🔄 Switch mode - executing once:", button.action);
      // Vibrate for single actions
      try {
        await vibrate(50);
      } catch (error) {
        console.warn("Vibration failed:", error);
      }
      useSendCommand(button.action);
      break;

    case "queue":
      console.log("📝 Queue mode - executing once:", button.action);
      // Vibrate for single actions
      try {
        await vibrate(50);
      } catch (error) {
        console.warn("Vibration failed:", error);
      }
      useSendCommand(button.action);
      break;

    default:
      // Vibrate for default actions
      try {
        await vibrate(50);
      } catch (error) {
        console.warn("Vibration failed:", error);
      }
      useSendCommand(button.action);
  }
};

// ✅ IMPROVED: Safe button release handler with cleanup
const handleButtonRelease = (button) => {
  const buttonId = button.id;
  const executionMode = button.execution_mode || "run_once";
  console.log(
    "🔼 Button released:",
    buttonId,
    "execution_mode:",
    executionMode,
  );
  console.log(
    "🐛 DEBUG: Currently pressed buttons before cleanup:",
    Array.from(currentlyPressedButtons.value),
  );
  console.log("🐛 DEBUG: Is gesturing:", isGesturing.value);

  // Remove from currently pressed buttons
  currentlyPressedButtons.value.delete(buttonId);
  console.log(
    "🐛 DEBUG: Currently pressed buttons after cleanup:",
    Array.from(currentlyPressedButtons.value),
  );

  // If no more buttons are pressed, reset gesture state
  if (currentlyPressedButtons.value.size === 0) {
    setTimeout(() => {
      if (currentlyPressedButtons.value.size === 0) {
        isGesturing.value = false;
        console.log("✋ All buttons released, gesturing reset");
      }
    }, 50);
  }

  if (executionMode === "repeat_while_pressing") {
    // Clean up pressed state
    if (pressedButtons.value.has(buttonId)) {
      pressedButtons.value.delete(buttonId);
      console.log("✅ Button removed from pressed set:", buttonId);
    }

    // Send WebSocket message to backend to stop repeat
    if ($socket.value?.readyState === WebSocket.OPEN) {
      const message = {
        action: "stop_button_repeat",
        buttonId: buttonId,
      };
      $sendSignedMessage(message);
      console.log(
        "📤 Sent stop_button_repeat message to backend (E2EE signed)",
      );
    } else {
      console.log("❌ WebSocket not connected");
    }

    // Stop vibration interval and timeout
    if (vibrationIntervals.value.has(buttonId)) {
      clearInterval(vibrationIntervals.value.get(buttonId));
      vibrationIntervals.value.delete(buttonId);
      console.log("🛑 Stopped vibration interval for button", buttonId);
    }

    // Stop vibration timeout
    const vibrationTimeoutKey = buttonId + "_vibration";
    if (buttonTimeouts.value.has(vibrationTimeoutKey)) {
      clearTimeout(buttonTimeouts.value.get(vibrationTimeoutKey));
      buttonTimeouts.value.delete(vibrationTimeoutKey);
      console.log("🛑 Stopped vibration timeout for button", buttonId);
    }

    // Remove from repeating set to hide visual indicator immediately for better UX
    repeatingButtons.value.delete(buttonId);
  }

  // Clean up any running intervals (fallback)
  if (buttonIntervals.value.has(buttonId)) {
    clearInterval(buttonIntervals.value.get(buttonId));
    buttonIntervals.value.delete(buttonId);
  }

  // Clean up any pending timeouts (fallback)
  if (buttonTimeouts.value.has(buttonId)) {
    clearTimeout(buttonTimeouts.value.get(buttonId));
    buttonTimeouts.value.delete(buttonId);
  }
};

// Handle finger movement to detect when finger leaves button area
const handleButtonMove = (button, event) => {
  const buttonId = button.id;
  const executionMode = button.execution_mode || "run_once";

  // Only care about repeat buttons
  if (executionMode !== "repeat_while_pressing") {
    return;
  }

  // Only check if button is currently repeating
  if (!repeatingButtons.value.has(buttonId)) {
    return;
  }

  // Get the button element bounds
  const buttonElement = event.currentTarget;
  const rect = buttonElement.getBoundingClientRect();

  // Get current touch position
  const touch = event.touches[0];
  if (!touch) return;

  const touchX = touch.clientX;
  const touchY = touch.clientY;

  // Check if finger is still within button bounds (with small buffer)
  const buffer = 20; // 20px buffer to prevent accidental stops
  const isWithinBounds =
    touchX >= rect.left - buffer &&
    touchX <= rect.right + buffer &&
    touchY >= rect.top - buffer &&
    touchY <= rect.bottom + buffer;

  if (!isWithinBounds) {
    console.log("👆 Finger moved off button, stopping repeat:", buttonId);
    handleButtonRelease(button);
  }
};

// ========================================
// LOCKED BUTTON HANDLERS
// These handle touch events from the lock overlay
// They mimic regular button behavior for multitouch/gestures but don't execute actions
// ========================================

const handleLockedButtonPress = async (button, payload) => {
  const buttonId = button.id;
  const event = payload.event;

  console.log(
    "🔒 Locked button touch start:",
    buttonId,
    "at position:",
    payload.position,
  );

  // Add to currently pressed buttons for multitouch detection
  currentlyPressedButtons.value.add(buttonId);

  // Use delay to detect gestures (same as regular buttons)
  // 50ms delay allows time to detect swipe gestures (which trigger at 50px movement)
  setTimeout(async () => {
    // Check if multitouch gesture detected
    if (currentlyPressedButtons.value.size > 1) {
      console.log("🔒 Locked button part of multitouch gesture, ignoring");
      isGesturing.value = true;
      currentlyPressedButtons.value.clear();
      return;
    }

    // Check if gesture already in progress
    if (isGesturing.value) {
      console.log("🔒 Gesture in progress, ignoring locked button");
      currentlyPressedButtons.value.delete(buttonId);
      return;
    }

    // Single button press - execute lock feedback
    console.log("🔒 Single locked button press detected, showing feedback");

    // ONLY show visual feedback if it's a valid single button press
    const buttonElement = document.querySelector(
      `[data-button-id="${buttonId}"]`,
    );
    if (buttonElement) {
      addVisualFeedback(buttonElement);
    }

    // Handle lock feedback (vibration, flash animation)
    await buttonLimits.handleLockedButtonClick(button.position);

    // Remove from pressed set
    currentlyPressedButtons.value.delete(buttonId);
  }, 50);
};

const handleLockedButtonRelease = (button, payload) => {
  const buttonId = button.id;
  console.log("🔒 Locked button touch end:", buttonId);

  // Remove from currently pressed buttons
  currentlyPressedButtons.value.delete(buttonId);

  // Reset gesture state if no more buttons pressed
  if (currentlyPressedButtons.value.size === 0) {
    setTimeout(() => {
      if (currentlyPressedButtons.value.size === 0) {
        isGesturing.value = false;
        console.log(
          "✋ All buttons released (including locked), gesturing reset",
        );
      }
    }, 50);
  }
};

const handleLockedButtonMove = (button, payload) => {
  // For locked buttons, we don't need special touchmove handling
  // The event propagates to parent for swipe gesture detection
  console.log("🔒 Locked button touch move:", button.id);
};

const handleLockedButtonClick = async (button) => {
  console.log("🔒 Locked button clicked (non-touch):", button.id);
  // Handle desktop click on locked button
  await buttonLimits.handleLockedButtonClick(button.position);
};

// ✅ IMPROVED: Safe value change handler with connection check
const handleValueChange = (button, value) => {
  if (connectionStore.getStatus !== "paired" || !$socket.value) {
    console.warn("⚠️ Cannot send command: WebSocket not connected");
    return;
  }

  if (button.action) {
    useSendCommand(button.action, value);
  }
};

// ✅ IMPROVED: Non-blocking message sending with proper error handling
const sendMessage = async () => {
  console.log("Sending message");
  await vibrate(2);

  // Check connection status before attempting to send
  if (connectionStore.getStatus !== "paired") {
    console.warn("⚠️ Cannot send message: Not paired with host");
    return;
  }

  // Check if the socket is ready before sending the message
  if ($socket.value && $socket.value.readyState === WebSocket.OPEN) {
    const get_deck_message = {
      action: "get_current_deck",
    };

    const get_all_decks_message = {
      action: "get_all_decks",
    };

    try {
      $sendSignedMessage(get_deck_message);
      $sendSignedMessage(get_all_decks_message);
      console.log("✅ Deck messages sent successfully (E2EE signed)");
    } catch (error) {
      console.error("❌ Failed to send messages:", error);
    }
  } else {
    console.warn(
      "⚠️ WebSocket not ready, current status:",
      connectionStore.getStatus,
    );
  }
};

// ✅ IMPROVED: Only send message if connected
if (connectionStore.getStatus === "paired") {
  sendMessage();
}

onMounted(() => {
  orientation.value = window.screen.orientation.type.split("-")[0];
  window.screen.orientation.addEventListener("change", () => {
    orientation.value = window.screen.orientation.type.split("-")[0];
  });

  // Global gesture detection
  const handleGlobalTouchStart = (event) => {
    activeTouches.value = event.touches.length;
    if (event.touches.length > 1) {
      isGesturing.value = true;
      console.log("🤚 Multi-touch gesture started, buttons disabled");
    }
  };

  const handleGlobalTouchEnd = (event) => {
    activeTouches.value = event.touches.length;
    if (event.touches.length <= 1) {
      // Small delay to prevent race conditions
      setTimeout(() => {
        if (activeTouches.value <= 1) {
          isGesturing.value = false;
          console.log("✋ Gesture ended, buttons re-enabled");
        }
      }, 20);
    }
  };

  // Add global touch listeners
  document.addEventListener("touchstart", handleGlobalTouchStart, {
    passive: true,
  });
  document.addEventListener("touchend", handleGlobalTouchEnd, {
    passive: true,
  });
  document.addEventListener("touchcancel", handleGlobalTouchEnd, {
    passive: true,
  });

  // Cleanup on unmount
  onUnmounted(() => {
    document.removeEventListener("touchstart", handleGlobalTouchStart);
    document.removeEventListener("touchend", handleGlobalTouchEnd);
    document.removeEventListener("touchcancel", handleGlobalTouchEnd);
  });
});

const AsyncLayout = defineAsyncComponent(
  () => import("../../desktop/layouts/deck.vue"),
);
const AsyncClickComponent = defineAsyncComponent(
  () => import("@/components/Deck/Dynamic/Button.vue"),
);

const AsyncRotaryComponent = defineAsyncComponent(
  () => import("@/components/Deck/Buttons/Rotary.vue"),
);

// Helper function to get icon URL - icons are always base64 data URLs from WebSocket
const getIconUrl = (button: any) => {
  // Icons are sent as base64 data URLs via WebSocket for both direct and relay modes
  // Desktop includes icons in all deck messages (get_current_deck, get_all_decks, deck_updated)
  return button.icon || null;
};

const deckStore = useDeckStore();
const syncedWidgets = useSyncedDeckWidgets();
const widgetRegistry = useWidgetRegistry();
const hasSyncedWidgetDocument = computed(
  () =>
    syncedWidgets.deckId.value === deckStore.getActiveDeck?.id &&
    syncedWidgets.revision.value > 0,
);

watch(orientation, (value) => {
  if (value === "portrait" || value === "landscape")
    syncedWidgets.setLayoutKey(value);
});

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

const syncedWidgetPlacementStyle = (instance: DeckWidgetInstance) => {
  const activeLayout =
    orientation.value === "portrait" || orientation.value === "landscape"
      ? orientation.value
      : syncedWidgets.layoutKey.value;
  const placement =
    activeLayout === "portrait" || activeLayout === "landscape"
      ? legacyWidgetPlacementForLayout(instance, activeLayout, {
          portrait: gridDimensions.value.cols,
          landscape: gridDimensions.value.rows,
        })
      : instance.layouts.default;
  if (!placement) return { display: "none" };
  return {
    gridColumn: `${placement.x + 1} / span ${placement.cols}`,
    gridRow: `${placement.y + 1} / span ${placement.rows}`,
  };
};

const widgetInteractionSessionId =
  globalThis.crypto?.randomUUID?.() ??
  `widget-${Date.now()}-${Math.random().toString(16).slice(2)}`;

const sendWidgetIntent = (intent: WidgetIntent) => {
  if (!hasSyncedWidgetDocument.value) return;
  const key = `${intent.instanceId}:${intent.portId}`;
  if (intent.phase === "start") activeWidgetPresses.set(key, intent);
  if (intent.phase === "end" || intent.phase === "cancel")
    activeWidgetPresses.delete(key);
  $sendSignedMessage({
    action: "deck_widget_intent",
    protocolVersion: 1,
    deckId: syncedWidgets.deckId.value,
    documentRevision: syncedWidgets.revision.value,
    intent: {
      ...intent,
      inputSource: "remote",
      interactionSessionId: widgetInteractionSessionId,
    },
    correlationId:
      globalThis.crypto?.randomUUID?.() ??
      `${Date.now()}-${Math.random().toString(16).slice(2)}`,
  });
};

const activeWidgetPresses = new Map<string, WidgetIntent>();
const cancelActiveWidgetPresses = () => {
  for (const intent of activeWidgetPresses.values()) {
    sendWidgetIntent({
      ...intent,
      phase: "cancel",
      sequence: intent.sequence + 1,
      timestamp: Date.now(),
      inputSource: "remote",
    });
  }
  activeWidgetPresses.clear();
};
const handleWidgetVisibility = () => {
  if (document.visibilityState === "hidden") cancelActiveWidgetPresses();
};
onMounted(() =>
  document.addEventListener("visibilitychange", handleWidgetVisibility),
);
onUnmounted(() => {
  document.removeEventListener("visibilitychange", handleWidgetVisibility);
  cancelActiveWidgetPresses();
});

// Extract border-radius from theme to match lock overlay with button shape
const overlayBorderRadius = computed(() => {
  try {
    const theme = deckStore.getActiveTheme;
    const css = theme?.components?.clickButton?.buttonCss || "";

    console.log("🔍 Theme buttonCss:", css);

    // Extract border-radius from inline CSS if present
    const match = css.match(/border-radius:\s*([^;]+);?/);
    if (match) {
      const radius = match[1].trim();
      console.log("✅ Found border-radius in CSS:", radius);
      return radius;
    }

    // Default to rounded-xl (0.75rem) if not found in theme CSS
    // This matches the fallback in Button.vue
    console.log("ℹ️ Using default border-radius: 0.75rem (rounded-xl)");
    return "0.75rem";
  } catch (error) {
    console.error("Error extracting border-radius from theme:", error);
    return "0.75rem";
  }
});

const sortedButtons = computed(() => {
  // based on grid settings render buttons and empty slots
  const gridSettings = deckStore.getActiveDeck?.settings;
  const grid = gridSettings.grid.split(",").map(Number);
  const totalSlots = grid[0] * grid[1];
  console.log("Total slots:", totalSlots);

  // Get all buttons from the active deck
  const allButtons = deckStore.getActiveDeck?.buttons || [];
  console.log("All buttons:", allButtons);

  // Create a result array with exactly totalSlots items
  const result = new Array(totalSlots).fill(null);

  // First, place all buttons into their respective positions in the result array
  // If a position is out of bounds, we'll handle it later
  allButtons.forEach((button) => {
    console.log(button.position);
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
        deck_id: deckStore.getActiveDeck.id,
        mode: button.mode || null,
        execution_mode: button.execution_mode || "run_once",
        execution_config: button.execution_config || null,
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
        deck_id: deckStore.getActiveDeck.id,
        mode: null,
        execution_mode: "run_once",
        execution_config: null,
      };
    }
  }

  return result;
});

const gridColumns = computed(() => {
  const value = Number(deckStore.getActiveDeck?.settings?.grid?.split(",")[0]);
  return Number.isFinite(value) && value > 0 ? value : 3;
});

const gridDimensions = computed(() => {
  const [rawCols, rawRows] = (deckStore.getActiveDeck?.settings?.grid ?? "3,4")
    .split(",")
    .map(Number);
  return {
    cols: Number.isFinite(rawCols) && rawCols > 0 ? rawCols : 3,
    rows: Number.isFinite(rawRows) && rawRows > 0 ? rawRows : 4,
  };
});

const normalizedLegacyButton = (button: any) => ({
  ...button,
  execution_config:
    typeof button.execution_config === "string"
      ? button.execution_config
      : button.execution_config
        ? JSON.stringify(button.execution_config)
        : null,
  icon_updated_at: button.icon_updated_at ?? null,
  icon_fit: button.icon_fit === "contain" ? "contain" : "cover",
});

const widgetForButton = (button: any) =>
  legacyButtonToWidget(normalizedLegacyButton(button), gridColumns.value);

const widgetDefinitionFor = (button: any): WidgetDefinition =>
  button.mode === "rotary"
    ? legacyRotaryDefinition
    : legacyActionButtonDefinition;

const isWidgetLocked = (index: number) =>
  !buttonLimits.isLoading.value && buttonLimits.isButtonLocked(index);

const test = async () => {
  await vibrate(105);
};

const zoomContainer = ref<HTMLElement | null>(null);
const scale = ref(1);
let initialDistance = 0;
let currentScale = 1; // Store last known scale
const zoomLevels = [0.5, 0.75, 1, 1.25, 1.5, 1.75, 2, 2.5, 3]; // More granular zoom levels
let currentZoomIndex = 2; // Start at index 2 (scale 1)
const startX = ref(0);
const startY = ref(0);
const isSwiping = ref(false);

// Reset all gesture and touch state
// Used after deck changes or when state gets stuck
const resetGestureState = () => {
  console.log("🔄 Resetting gesture state");
  isGesturing.value = false;
  isSwiping.value = false;
  currentlyPressedButtons.value.clear();
  activeTouches.value = 0;

  // Also clear any repeat button states for safety
  pressedButtons.value.clear();
  repeatingButtons.value.clear();

  // Clear any running intervals/timeouts
  buttonIntervals.value.forEach((interval) => clearInterval(interval));
  buttonIntervals.value.clear();
  buttonTimeouts.value.forEach((timeout) => clearTimeout(timeout));
  buttonTimeouts.value.clear();
  vibrationIntervals.value.forEach((interval) => clearInterval(interval));
  vibrationIntervals.value.clear();
};

const nextDeck = () => {
  const currentDeckId = deckStore.getActiveDeck?.id;
  const currentIndex = deckStore.getAllDecks.findIndex(
    (deck: any) => deck.id === currentDeckId,
  );

  const nextIndex = (currentIndex + 1) % deckStore.getAllDecks.length;
  const nextDeck = deckStore.getAllDecks[nextIndex];

  if (nextDeck) {
    deckStore.setActiveDeck(nextDeck.id, "mobile");
    // deckStore.updateActiveDeck(nextDeck);
  } else {
    // If no next deck, loop back to first deck
    const firstDeck = deckStore.getAllDecks[0];
    if (firstDeck) {
      deckStore.setActiveDeck(firstDeck.id, "mobile");
      // deckStore.updateActiveDeck(firstDeck);
    }
  }

  // Reset all gesture state after deck change
  resetGestureState();
};

const previousDeck = () => {
  const currentDeckId = deckStore.getActiveDeck?.id;
  const currentIndex = deckStore.getAllDecks.findIndex(
    (deck: any) => deck.id === currentDeckId,
  );

  // Calculate previous index, handling wrap-around to last deck
  const previousIndex =
    currentIndex <= 0 ? deckStore.getAllDecks.length - 1 : currentIndex - 1;
  const previousDeck = deckStore.getAllDecks[previousIndex];

  if (previousDeck) {
    deckStore.setActiveDeck(previousDeck.id, "mobile");
    // deckStore.updateActiveDeck(previousDeck);
  } else {
    // If no previous deck, loop to last deck
    const lastDeck = deckStore.getAllDecks[deckStore.getAllDecks.length - 1];
    if (lastDeck) {
      deckStore.setActiveDeck(lastDeck.id, "mobile");
      // deckStore.updateActiveDeck(lastDeck);
    }
  }

  // Reset all gesture state after deck change
  resetGestureState();
};

const handleTouchStart = (event: TouchEvent) => {
  if (event.touches.length === 1) {
    startX.value = event.touches[0].clientX;
    startY.value = event.touches[0].clientY;
    isSwiping.value = true;
  } else if (event.touches.length === 2) {
    const touch1 = event.touches[0];
    const touch2 = event.touches[1];
    initialDistance = Math.hypot(
      touch2.clientX - touch1.clientX,
      touch2.clientY - touch1.clientY,
    );
    currentScale = scale.value;
    isSwiping.value = false;
  }
};

const handleTouchMove = (event: TouchEvent) => {
  if (event.touches.length === 1 && isSwiping.value) {
    const deltaX = event.touches[0].clientX - startX.value;
    const deltaY = event.touches[0].clientY - startY.value;

    // Early movement detection - if finger moves significantly, it's likely a swipe
    // Mark as gesture early to prevent button execution before reaching full swipe threshold
    if (orientation.value === "portrait") {
      // Detect horizontal movement early (50px threshold)
      if (Math.abs(deltaX) > 30 && Math.abs(deltaY) < 20) {
        isGesturing.value = true;
      }
    } else {
      // Detect vertical movement early (50px threshold)
      if (Math.abs(deltaY) > 30 && Math.abs(deltaX) < 20) {
        isGesturing.value = true;
      }
    }

    if (orientation.value === "portrait") {
      // Left/Right swipe in Portrait
      if (Math.abs(deltaX) > 200 && Math.abs(deltaY) < 60) {
        // Mark as gesture IMMEDIATELY to prevent button execution
        isGesturing.value = true;
        isSwiping.value = false;
        if (deltaX > 0) {
          // Swiped right - go to previous deck
          previousDeck();
        } else {
          // Swiped left - go to next deck
          nextDeck();
        }
      }
    } else {
      // Up/Down swipe in Landscape
      if (Math.abs(deltaY) > 200 && Math.abs(deltaX) < 60) {
        // Mark as gesture IMMEDIATELY to prevent button execution
        isGesturing.value = true;
        isSwiping.value = false;
        if (deltaY > 0) {
          // Swiped down - go to previous deck
          previousDeck();
        } else {
          // Swiped up - go to next deck
          nextDeck();
        }
      }
    }
  } else if (event.touches.length === 2) {
    const touch1 = event.touches[0];
    const touch2 = event.touches[1];
    const currentDistance = Math.hypot(
      touch2.clientX - touch1.clientX,
      touch2.clientY - touch1.clientY,
    );

    if (initialDistance) {
      // Calculate pinch direction (in or out)
      const pinchRatio = currentDistance / initialDistance;

      // Determine zoom direction based on pinch
      // Increased threshold for zoom level changes (from 1.1 to 1.3 and 0.9 to 0.7)
      if (pinchRatio > 1.4) {
        // Pinch out - zoom in (increase index)
        if (currentZoomIndex < zoomLevels.length - 1) {
          currentZoomIndex++;
          scale.value = zoomLevels[currentZoomIndex];
          initialDistance = currentDistance; // Reset for smoother zooming
        }
      } else if (pinchRatio < 0.6) {
        // Pinch in - zoom out (decrease index)
        if (currentZoomIndex > 0) {
          currentZoomIndex--;
          scale.value = zoomLevels[currentZoomIndex];
          initialDistance = currentDistance; // Reset for smoother zooming
        }
      }
    }
  }
};

const handleTouchEnd = () => {
  currentScale = scale.value;
};

// Function to zoom in one level
const zoomIn = () => {
  if (currentZoomIndex < zoomLevels.length - 1) {
    currentZoomIndex++;
    scale.value = zoomLevels[currentZoomIndex];
    updateGridBasedOnZoom(scale.value, zoomLevels[currentZoomIndex - 1]);
  }
};

// Function to zoom out one level
const zoomOut = () => {
  if (currentZoomIndex > 0) {
    currentZoomIndex--;
    scale.value = zoomLevels[currentZoomIndex];
    updateGridBasedOnZoom(scale.value, zoomLevels[currentZoomIndex + 1]);
  }
};

const updateGridBasedOnZoom = (newScale: number, oldScale: number) => {
  if (!newScale || !oldScale) return;

  // Get current grid dimensions
  const grid = deckStore.getActiveDeck.settings.grid.split(",").map(Number);
  const delta = newScale > oldScale ? 1 : -1; // Convert scale change to +/-1

  // Determine if landscape or portrait based on current ratio
  const isLandscape = grid[0] > grid[1];

  // Set max/min dimensions based on orientation
  const maxDims = isLandscape ? [11, 9] : [9, 11];
  const minDims = [1, 3];

  // Calculate new dimensions while respecting limits
  const newGrid = [
    Math.min(Math.max(grid[0] + delta, minDims[0]), maxDims[0]),
    Math.min(Math.max(grid[1] + delta, minDims[1]), maxDims[1]),
  ];

  // Only update if within bounds
  if (
    newGrid[0] >= minDims[0] &&
    newGrid[0] <= maxDims[0] &&
    newGrid[1] >= minDims[1] &&
    newGrid[1] <= maxDims[1]
  ) {
    deckStore.getActiveDeck.settings.grid = newGrid.join(",");
  }
};

watch(
  () => scale.value,
  (newScale, oldScale) => {
    if (newScale !== undefined && oldScale !== undefined) {
      updateGridBasedOnZoom(newScale, oldScale);
    }
  },
  { immediate: true },
);

// Function to get repeat indicator animation style based on button config
const getRepeatIndicatorStyle = (button) => {
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

// WebSocket message handlers for backend repeat notifications
onMounted(() => {
  if ($socket.value) {
    const handleWebSocketMessage = async (event) => {
      try {
        const data = JSON.parse(event.data);
        console.log("📨 WebSocket message received:", data);

        // Handle repeat start notification
        if (data.action === "button_repeat_started") {
          const buttonId = data.buttonId;
          console.log(
            "🟢 Backend notified: repeat started for button",
            buttonId,
          );
          repeatingButtons.value.add(buttonId);

          // Start vibration interval matching the button's repeat timing
          const button = deckStore.getButtonById(buttonId);
          if (button) {
            const config = button.execution_config
              ? JSON.parse(button.execution_config)
              : null;
            const repeatInterval = config?.repeat_interval_ms || 100;

            console.log(
              "🕐 Backend has started repeating, vibrating immediately and then every:",
              repeatInterval,
              "ms for button",
              buttonId,
            );

            // Backend has already executed the first action when this message is received
            // So vibrate immediately to match the first execution
            vibrate(30)
              .then(() => {
                console.log(
                  "📳 First repeat execution vibration for button",
                  buttonId,
                );
              })
              .catch((error) =>
                console.warn("First repeat vibration failed:", error),
              );

            // Start interval immediately for subsequent executions to match backend timing
            const vibrationInterval = setInterval(async () => {
              if (
                repeatingButtons.value.has(buttonId) &&
                pressedButtons.value.has(buttonId)
              ) {
                try {
                  await vibrate(30);
                  console.log("📳 Interval vibration for button", buttonId);
                } catch (error) {
                  console.warn("Interval vibration failed:", error);
                }
              } else {
                // Stop interval if button is no longer repeating
                clearInterval(vibrationInterval);
                vibrationIntervals.value.delete(buttonId);
                console.log(
                  "🛑 Stopped vibration interval for button",
                  buttonId,
                );
              }
            }, repeatInterval);

            vibrationIntervals.value.set(buttonId, vibrationInterval);
          }
        }

        // Remove the generic message handling since we now use intervals

        // Note: We don't handle 'button_repeat_stopped' because we remove the indicator
        // immediately on release for better UX responsiveness
      } catch (error) {
        // Ignore non-JSON messages - this is normal for non-JSON WebSocket data
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

// Cleanup when component unmounts
onUnmounted(() => {
  // Clear pressed buttons set
  pressedButtons.value.clear();

  // Clear repeating buttons set
  repeatingButtons.value.clear();

  // Clear all vibration intervals
  vibrationIntervals.value.forEach((interval) => {
    clearInterval(interval);
  });
  vibrationIntervals.value.clear();
});

const fetchbuttons = () => {};
</script>

<style lang="scss">
.grid-container--fill {
  grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));
}

.grid-container--fit {
  grid-template-columns: repeat(auto-fit, minmax(5rem, 1fr));
}

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
</style>
