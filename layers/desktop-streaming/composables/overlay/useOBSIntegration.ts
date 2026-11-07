import { ref, computed, onUnmounted, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { validateCanvasDimensions } from './canvasGeometry';

export interface OBSStatus {
  connected: boolean;
  streaming: boolean;
  recording: boolean;
  canvasWidth?: number;
  canvasHeight?: number;
  syncedAt?: number;
}

export interface OBSCanvasApplyResult {
  type: 'OBS_CANVAS_APPLY_RESULT';
  request_id: string;
  status: 'applied' | 'rejected';
  width?: number;
  height?: number;
  output_width?: number;
  output_height?: number;
  managed_source_found?: boolean;
  managed_source_width?: number;
  managed_source_height?: number;
  managed_source_matches?: boolean;
  error_code?: string;
  error?: string;
}

export const useOBSIntegration = (desiredCanvas?: () => { width: number; height: number }) => {
  // OBS sync state
  const obsSyncStatus = ref<"disconnected" | "syncing" | "observed" | "synced" | "error">("disconnected");
  const lastObsSync = ref<number | null>(null);
  const obsStatus = ref<OBSStatus>({
    connected: false,
    streaming: false,
    recording: false
  });
  
  // Event listeners for cleanup
  const eventUnlisteners = ref<Array<() => void>>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const pendingCanvasApplies = new Map<string, {
    resolve: (result: OBSCanvasApplyResult) => void;
    reject: (error: Error) => void;
    timeout: ReturnType<typeof setTimeout>;
    width: number;
    height: number;
  }>();
  let initializePromise: Promise<void> | null = null;
  const sourceConfirmed = ref(false);
  let requestTimer: ReturnType<typeof setTimeout> | undefined;
  let reconnectTimer: ReturnType<typeof setTimeout> | undefined;
  const reconcile = () => {
    const desired = desiredCanvas?.();
    const same = !desired || (desired.width === obsStatus.value.canvasWidth && desired.height === obsStatus.value.canvasHeight);
    obsSyncStatus.value = !obsStatus.value.connected ? 'disconnected' : same && sourceConfirmed.value ? 'synced' : 'observed';
  };
  if (desiredCanvas) watch(desiredCanvas, reconcile, { flush: "sync" });


  // Computed properties
  const isConnected = computed(() => obsStatus.value.connected);
  const isStreaming = computed(() => obsStatus.value.streaming);
  const isRecording = computed(() => obsStatus.value.recording);
  
  const canvasSize = computed(() => {
    if (obsStatus.value.canvasWidth && obsStatus.value.canvasHeight) {
      return {
        width: obsStatus.value.canvasWidth,
        height: obsStatus.value.canvasHeight
      };
    }
    return null;
  });

  const syncStatusText = computed(() => {
    switch (obsSyncStatus.value) {
      case 'synced': return 'Canvas and OBS source sizes match';
      case 'observed': return 'OBS connected; canvas/source agreement unconfirmed';
      case 'syncing': return 'Syncing...';
      case 'error': return 'Sync Error';
      default: return 'Not Connected';
    }
  });


  // Request OBS canvas size
  const syncWithObs = async () => {
    try {
      obsSyncStatus.value = "syncing";
      console.log("📐 Requesting canvas size from OBS...");

      // Arm before invoking: a fast native response can arrive during invoke.
      clearTimeout(requestTimer);
      requestTimer = setTimeout(() => {
        if (obsSyncStatus.value === "syncing") {
          obsSyncStatus.value = "error";
          console.log("❌ OBS sync timeout - check OBS plugin connection");
        }
      }, 5000);
      await invoke("request_obs_canvas_size");
      
      return true;
    } catch (err) {
      clearTimeout(requestTimer);
      obsSyncStatus.value = "error";
      error.value = err instanceof Error ? err.message : String(err);
      console.error("❌ Failed to sync with OBS:", err);
      return false;
    }
  };

  // Setup OBS event listeners
  const setupOBSListeners = async () => {
    try {
      // Listen for OBS canvas size changes
      const unlistenObsCanvas = await listen("obs-canvas-size-changed", (event) => {
        const payload = event.payload as { width: number; height: number; synced_at: number; streaming?: boolean; recording?: boolean; managed_source_found?: boolean; managed_source_matches?: boolean };
        if (!obsStatus.value.connected) return;
        try { validateCanvasDimensions(payload); } catch { return; }
        clearTimeout(requestTimer);
        obsStatus.value = { ...obsStatus.value, canvasWidth: payload.width, canvasHeight: payload.height, connected: true, syncedAt: payload.synced_at };
        if (typeof payload.streaming === "boolean") obsStatus.value.streaming = payload.streaming;
        if (typeof payload.recording === "boolean") obsStatus.value.recording = payload.recording;
        sourceConfirmed.value = payload.managed_source_found === true && payload.managed_source_matches === true;
        lastObsSync.value = payload.synced_at;
        reconcile();
      });
      eventUnlisteners.value.push(unlistenObsCanvas);

      const unlistenCanvasApply = await listen<OBSCanvasApplyResult>("obs-canvas-apply-result", ({ payload }) => {
        const pending = pendingCanvasApplies.get(payload.request_id);
        if (!pending) return;
        clearTimeout(pending.timeout);
        pendingCanvasApplies.delete(payload.request_id);
        if (payload.status !== 'applied' || payload.width !== pending.width || payload.height !== pending.height) {
          const message = payload.error || (payload.status === 'applied' ? 'OBS readback does not match the requested canvas' : 'OBS rejected the canvas change');
          obsSyncStatus.value = 'error';
          error.value = message;
          pending.reject(new Error(message));
          return;
        }
        obsStatus.value = {
          ...obsStatus.value,
          canvasWidth: payload.width,
          canvasHeight: payload.height,
          connected: true,
        };
        sourceConfirmed.value = payload.managed_source_found === true && payload.managed_source_matches === true;
        reconcile();
        error.value = null;
        pending.resolve(payload);
      });
      eventUnlisteners.value.push(unlistenCanvasApply);

      // Listen for OBS connection status
      const unlistenObsStatus = await listen<boolean>("obs_plugin_status", (event) => {
        const payload = { connected: event.payload };
        const wasConnected = obsStatus.value.connected;
        obsStatus.value = { ...obsStatus.value, ...payload };
        if (!payload.connected) {
          sourceConfirmed.value = false;
          obsStatus.value.canvasWidth = undefined;
          obsStatus.value.canvasHeight = undefined;
          clearTimeout(requestTimer);
          obsStatus.value.streaming = false;
          obsStatus.value.recording = false;
          clearTimeout(reconnectTimer);
          for (const pending of pendingCanvasApplies.values()) {
            clearTimeout(pending.timeout);
            pending.reject(new Error('OBS disconnected; saved canvas may differ'));
          }
          pendingCanvasApplies.clear();
        }
        reconcile();
        console.log("🎬 OBS status updated:", payload);
        
        // Auto-sync when OBS first connects
        if (!wasConnected && payload.connected && enableAutoSync.value) {
          console.log('🔄 OBS connected - auto-syncing canvas and pushing overlay...');
          clearTimeout(reconnectTimer);
          reconnectTimer = setTimeout(async () => {
            await syncWithObs();
            await sendOverlayToObs();
          }, 1000); // Small delay to ensure connection is stable
        }
      });
      eventUnlisteners.value.push(unlistenObsStatus);

      console.log("✅ OBS event listeners setup complete");
    } catch (err) {
      console.error("❌ Failed to setup OBS event listeners:", err);
      error.value = err instanceof Error ? err.message : String(err);
      eventUnlisteners.value.splice(0).forEach(unlisten => unlisten());
      throw err;
    }
  };

  const applyCanvasSize = async (width: number, height: number) => {
    validateCanvasDimensions({ width, height });
    if (pendingCanvasApplies.size) throw new Error('An OBS canvas change is already pending');
    clearTimeout(requestTimer);
    const requestId = crypto.randomUUID();
    obsSyncStatus.value = 'syncing';
    error.value = null;
    const result = new Promise<OBSCanvasApplyResult>((resolve, reject) => {
      const timeout = setTimeout(() => {
        pendingCanvasApplies.delete(requestId);
        obsSyncStatus.value = 'error';
        const timeoutError = new Error('Timed out waiting for OBS to confirm the canvas change');
        error.value = timeoutError.message;
        reject(timeoutError);
      }, 10_000);
      pendingCanvasApplies.set(requestId, { resolve, reject, timeout, width, height });
    });
    void invoke('apply_obs_canvas_size', { width, height, requestId }).catch((cause) => {
      const pending = pendingCanvasApplies.get(requestId);
      if (!pending) return;
      clearTimeout(pending.timeout);
      pendingCanvasApplies.delete(requestId);
      obsSyncStatus.value = 'error';
      error.value = String(cause);
      pending.reject(cause instanceof Error ? cause : new Error(String(cause)));
    });
    return await result;
  };

  // Get OBS status
  const refreshOBSStatus = async () => {
    try {
      const status = await invoke<{ status: "Connected" | "Disconnected" | "Connecting" | { Error: string } } | null>("get_integration_status", { integrationId: "obs" });
      const connected = status?.status === "Connected";
      obsStatus.value = { ...obsStatus.value, connected };
      if (!connected) sourceConfirmed.value = false;
      reconcile();
      return obsStatus.value;
    } catch (err) {
      console.error("Failed to get OBS status:", err);
      return null;
    }
  };

  // Send overlay to OBS with current streaming elements
  const sendOverlayToObs = async () => {
    loading.value = true;
    error.value = null;
    
    try {
      console.log('🎬 Sending overlay to OBS');
      await invoke('send_overlay_to_obs');
      console.log('Overlay capability queued for OBS; browser rendering is confirmed separately');
      return true;
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err);
      console.error('❌ Failed to send overlay to OBS:', err);
      return false;
    } finally {
      loading.value = false;
    }
  };

  // Auto-sync with OBS when connection is established
  const enableAutoSync = ref(true);
  
  // Initialize OBS integration
  const initialize = async () => {
    if (initializePromise) return initializePromise;
    initializePromise = (async () => {
      await setupOBSListeners();
      await refreshOBSStatus();
      if (isConnected.value) await syncWithObs();
      console.log('✅ OBS integration initialized - event-based sync enabled');
    })();
    try {
      await initializePromise;
    } catch (cause) {
      initializePromise = null;
      throw cause;
    }
  };

  // Cleanup function
  const cleanup = () => {
    clearTimeout(requestTimer);
    clearTimeout(reconnectTimer);
    // Clean up all event listeners
    eventUnlisteners.value.forEach(unlisten => {
      try {
        unlisten();
      } catch (err) {
        console.error("Error cleaning up OBS listener:", err);
      }
    });
    eventUnlisteners.value = [];
    for (const pending of pendingCanvasApplies.values()) {
      clearTimeout(pending.timeout);
      pending.reject(new Error('OBS canvas Apply was cancelled'));
    }
    pendingCanvasApplies.clear();
    initializePromise = null;
  };

  // Auto cleanup on unmount
  onUnmounted(() => {
    cleanup();
  });

  return {
    // State
    obsStatus,
    obsSyncStatus,
    lastObsSync,
    loading,
    error,
    enableAutoSync,
    
    // Computed
    isConnected,
    isStreaming,
    isRecording,
    canvasSize,
    syncStatusText,
    
    // Methods
    sendOverlayToObs,
    syncWithObs,
    refreshOBSStatus,
    applyCanvasSize,
    // Initialization
    initialize,
    cleanup,
    setupOBSListeners
  };
};
