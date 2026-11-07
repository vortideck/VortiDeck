import { ref, computed, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useServerConnection } from './useServerConnection';
import { useOBSIntegration } from './useOBSIntegration';
import { validateCanvasDimensions } from './canvasGeometry';

export interface OverlayCanvas {
  width: number;
  height: number;
  scale: number;
  actualWidth: number;
  actualHeight: number;
}

export interface OverlayState {
  serverRunning: boolean;
  serverUrl: string;
  obsConnected: boolean;
  obsCanvasSize: { width: number; height: number } | null;
  elementsCount: number;
  lastSync: number | null;
}

export const useOverlayManager = () => {
  // Initialize sub-composables
  const serverConnection = useServerConnection();


  // Canvas state - will be initialized from saved data or defaults
  const canvasWidth = ref(1920);
  const canvasHeight = ref(1080);
  const obsIntegration = useOBSIntegration(() => ({ width: canvasWidth.value, height: canvasHeight.value }));

  // Load saved canvas size immediately to avoid flash
  const loadSavedCanvasSize = async () => {
    try {
      const result = await invoke<{ width: number; height: number; found: boolean }>('get_saved_canvas_size');
      if (result.found) {
        validateCanvasDimensions(result);
        canvasWidth.value = result.width;
        canvasHeight.value = result.height;
        console.log(`📐 Loaded saved canvas size: ${result.width}×${result.height}`);
      }
    } catch (err) {
      console.warn('Failed to load saved canvas size:', err);
    }
  };

  // Watch for backend status changes to update canvas size
  const updateCanvasFromStatus = () => {
    if (serverConnection.overlayStatus.value?.canvas_size) {
      const { width, height } = serverConnection.overlayStatus.value.canvas_size;
      if (width && height) {
        validateCanvasDimensions({ width, height });
        canvasWidth.value = width;
        canvasHeight.value = height;
        console.log(`📐 Canvas size updated from backend: ${width}x${height}`);
      }
    }
  };
  const canvasScale = ref(0.5);

  const setCanvasDimensions = (width: number, height: number) => {
    validateCanvasDimensions({ width, height });
    canvasWidth.value = width;
    canvasHeight.value = height;
  };

  // Loading states
  const loading = ref(false);
  const error = ref<string | null>(null);
  let initializePromise: Promise<void> | null = null;

  // Computed states
  const overlayState = computed((): OverlayState => ({
    serverRunning: serverConnection.serverRunning.value,
    serverUrl: serverConnection.serverUrl.value,
    obsConnected: obsIntegration.isConnected.value,
    obsCanvasSize: obsIntegration.canvasSize.value,
    elementsCount: serverConnection.overlayStatus.value?.elements_count ?? 0,
    lastSync: obsIntegration.lastObsSync.value
  }));

  const canvasInfo = computed((): OverlayCanvas => ({
    width: canvasWidth.value,
    height: canvasHeight.value,
    scale: canvasScale.value,
    actualWidth: canvasWidth.value * canvasScale.value,
    actualHeight: canvasHeight.value * canvasScale.value
  }));

  const isReady = computed(() =>
    overlayState.value.serverRunning && overlayState.value.elementsCount > 0
  );

  // Server management - shared native runtime
  const startServer = async () => {
    loading.value = true;
    error.value = null;

    try {
      console.log('🚀 Starting overlay server with streaming elements');

      // Generate HTML with current streaming elements
      await generateOverlayHtml();

      // Start the server
      const result = await serverConnection.startServer();

      console.log('✅ Overlay server started successfully');
      return result;
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err);
      console.error('❌ Failed to start overlay server:', err);
      throw err;
    } finally {
      loading.value = false;
    }
  };

  const stopServer = async () => {
    loading.value = true;
    error.value = null;

    try {
      console.log('🛑 Stopping overlay server');
      const result = await serverConnection.stopServer();
      console.log('✅ Overlay server stopped');
      return result;
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err);
      console.error('❌ Failed to stop overlay server:', err);
      throw err;
    } finally {
      loading.value = false;
    }
  };

  // HTML generation with streaming elements
  const generateOverlayHtml = async () => {
    loading.value = true;
    error.value = null;

    try {
      console.log('🎨 Generating overlay HTML with streaming elements');

      // Native OverlayState owns the active composition. Generation must not
      // replace it with the template manager's separate debug/demo elements.
      // Generate the actual HTML
      const html = await serverConnection.generateOverlayHtml();

      console.log('✅ Generated overlay HTML with streaming elements');
      return html;
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err);
      console.error('❌ Failed to generate overlay HTML:', err);
      throw err;
    } finally {
      loading.value = false;
    }
  };

  // OBS integration - companion control
  const sendToObs = async () => {
    loading.value = true;
    error.value = null;

    try {
      console.log('🎬 Sending overlay to OBS');

      // Ensure HTML is up to date with current elements
      await generateOverlayHtml();

      // Send to OBS
      const success = await obsIntegration.sendOverlayToObs();

      if (success) {
        console.log('✅ Successfully sent overlay to OBS');
      } else {
        throw new Error('Failed to send overlay to OBS');
      }

      return success;
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err);
      console.error('❌ Failed to send overlay to OBS:', err);
      return false;
    } finally {
      loading.value = false;
    }
  };

  // Canvas synchronization with OBS - observation only
  const syncWithObs = async () => {
    loading.value = true;
    error.value = null;

    try {
      console.log('🔄 Syncing canvas with OBS');

      // Request canvas observation - no HTML regeneration during sync
      const requested = await obsIntegration.syncWithObs();
      if (!requested) return false;

      console.log('✅ Canvas size request sent to OBS plugin');
      // OBS reports its canvas; saved composition dimensions are unchanged.

      return true;
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err);
      console.error('❌ Failed to sync with OBS:', err);
      return false;
    } finally {
      loading.value = false;
    }
  };

  // Initialize all systems
  const initialize = async () => {
    if (initializePromise) return initializePromise;
    initializePromise = (async () => {
      console.log('🚀 Initializing overlay services');

      // Load saved canvas size first to avoid 1080p flash
      await loadSavedCanvasSize();

      // Initialize all sub-systems
      await Promise.all([
        serverConnection.initialize(),
        obsIntegration.initialize()
      ]);

      // Initialize canvas size from backend status
      updateCanvasFromStatus();

      console.log('✅ Overlay Manager V3 initialized successfully');
    })();
    try {
      await initializePromise;
    } catch (err) {
      initializePromise = null;
      error.value = err instanceof Error ? err.message : String(err);
      console.error('❌ Failed to initialize Overlay Manager:', err);
      throw err;
    }
  };

  // Cleanup
  const cleanup = async () => {
    await Promise.all([
      serverConnection.cleanup(),
      obsIntegration.cleanup()
    ]);
    initializePromise = null;
  };

  onUnmounted(() => {
    cleanup();
  });

  return {
    // State
    loading,
    error,
    canvasWidth,
    canvasHeight,
    canvasScale,
    setCanvasDimensions,

    // Computed
    overlayState,
    canvasInfo,
    isReady,

    // Server management - shared native runtime
    startServer,
    stopServer,
    generateOverlayHtml,

    // OBS integration - shared native runtime
    sendToObs,
    syncWithObs,

    // Sub-composables access
    serverConnection,
    obsIntegration,

    // System management
    initialize,
    cleanup
  };
};
