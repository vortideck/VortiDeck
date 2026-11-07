import { ref, computed, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export interface ServerStatus {
  server_running: boolean;  // ✅ Backend uses snake_case
  elements_count?: number;
  server_url: string | null;
  serverPort?: number;
  overlayHtml?: string;
  overlayUrl?: string;
  error?: string | null;
  canvas_size?: {
    width: number;
    height: number;
  };
}

export const useServerConnection = () => {
  // Native overlay server state
  const serverRunning = ref(false);
  const serverUrl = ref<string>("");
  const overlayStatus = ref<ServerStatus | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  
  // Event listeners for cleanup
  const eventUnlisteners = ref<Array<() => void>>([]);

  // Computed properties
  const isConnected = computed(() => serverRunning.value);
  
  const serverInfo = computed(() => ({
    running: serverRunning.value,
    url: serverUrl.value,
    status: overlayStatus.value
  }));

  // Start the native overlay server
  const startServer = async () => {
    loading.value = true;
    error.value = null;
    
    try {
      console.log('🌐 Starting overlay server');
      
      // Add timeout to prevent infinite loading (from v1)
      const timeoutPromise = new Promise<never>((_, reject) => {
        setTimeout(() => reject(new Error('Server start timeout after 10 seconds')), 10000);
      });
      
      const startPromise = invoke<string>('start_overlay_server_command');
      
      const result = await Promise.race([startPromise, timeoutPromise]);
      
      console.log('✅ Overlay server started');
      
      // Server status will be updated via event listeners
      return result;
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err);
      console.error('❌ Failed to start overlay server:', err);
      throw err;
    } finally {
      loading.value = false;
    }
  };

  // Stop overlay server - EXACT v1 implementation
  const stopServer = async () => {
    loading.value = true;
    error.value = null;
    
    try {
      console.log('🛑 Stopping overlay server');
      
      const result: string = await invoke('stop_overlay_server_command');
      
      console.log('✅ Overlay server stopped');
      
      // Server status will be updated via event listeners
      return result;
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err);
      console.error('❌ Failed to stop overlay server:', err);
      throw err;
    } finally {
      loading.value = false;
    }
  };

  // Refresh overlay status - from v1
  const refreshStatus = async () => {
    try {
      const status = await invoke<ServerStatus>("get_overlay_status");
      overlayStatus.value = status;
      
      if (status) {
        // ✅ Backend returns snake_case, convert to local camelCase
        serverRunning.value = status.server_running;
        serverUrl.value = status.server_url || "";
      }
    } catch (err) {
      console.error("Failed to refresh overlay status:", err);
    }
  };

  // Read the canonical native HTML
  const generateOverlayHtml = async () => {
    loading.value = true;
    error.value = null;
    
    try {
      console.log('🎨 Generating overlay HTML');
      const html = await invoke<string>('generate_overlay_html_command');
      console.log('✅ Generated overlay HTML');
      return html;
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err);
      console.error('❌ Failed to generate HTML:', err);
      throw err;
    } finally {
      loading.value = false;
    }
  };

  // Observe native server status
  const setupServerStatusListeners = async () => {
    try {
      // Listen for server started events
      const unlistenStarted = await listen("overlay-server-started", (event) => {
        const payload = event.payload as { url: string };
        serverRunning.value = true;
        serverUrl.value = payload.url;
        // The URL contains the process-scoped overlay capability; never log it.
        console.log('✅ Server started');
      });
      eventUnlisteners.value.push(unlistenStarted);

      // Listen for server stopped events
      const unlistenStopped = await listen("overlay-server-stopped", () => {
        serverRunning.value = false;
        serverUrl.value = "";
        console.log("🛑 Server stopped");
      });
      eventUnlisteners.value.push(unlistenStopped);

      // Listen for server error events
      const unlistenError = await listen("overlay-server-error", (event) => {
        const payload = event.payload as { error: string };
        serverRunning.value = false;
        serverUrl.value = "";
        error.value = payload.error;
        console.error(`❌ Server error: ${payload.error}`);
      });
      eventUnlisteners.value.push(unlistenError);

      // Listen for overlay status changes
      const unlistenStatus = await listen("overlay-status-changed", (event) => {
        const payload = event.payload as ServerStatus;
        overlayStatus.value = payload;
        // ✅ Backend uses snake_case
        serverRunning.value = payload.server_running;
        serverUrl.value = payload.server_url || "";
      });
      eventUnlisteners.value.push(unlistenStatus);

      console.log("✅ Event listeners setup complete");
    } catch (err) {
      console.error("❌ Failed to setup event listeners:", err);
      error.value = err instanceof Error ? err.message : String(err);
    }
  };

  // Load initial canvas size from persistent store
  const loadInitialCanvasSize = async () => {
    try {
      const savedCanvas = await invoke<{ width: number; height: number; found: boolean }>('get_saved_canvas_size');
      
      if (savedCanvas.found) {
        console.log(`📐 Loaded saved canvas size: ${savedCanvas.width}×${savedCanvas.height}`);
        
        // Update status with saved canvas size
        if (overlayStatus.value) {
          overlayStatus.value.canvas_size = {
            width: savedCanvas.width,
            height: savedCanvas.height
          };
        } else {
          overlayStatus.value = {
            server_running: false,
            server_url: null,
            canvas_size: {
              width: savedCanvas.width,
              height: savedCanvas.height
            }
          };
        }
      }
    } catch (err) {
      console.warn('Failed to load saved canvas size:', err);
    }
  };

  // Initialize on mount
  const initialize = async () => {
    await setupServerStatusListeners();
    await loadInitialCanvasSize(); // Load saved canvas size first
    await refreshStatus();
    
    // Set up status refresh interval (from v1)
    const intervalId = setInterval(() => {
      if (serverRunning.value) {
        refreshStatus();
      }
    }, 5000); // Refresh every 5 seconds when running
    
    // Store interval for cleanup
    eventUnlisteners.value.push(() => clearInterval(intervalId));
  };

  // Cleanup function
  const cleanup = () => {
    // Clean up all event listeners
    eventUnlisteners.value.forEach(unlisten => {
      try {
        unlisten();
      } catch (err) {
        console.error("Error cleaning up listener:", err);
      }
    });
    eventUnlisteners.value = [];
  };

  // Auto cleanup on unmount
  onUnmounted(() => {
    cleanup();
  });

  return {
    // State
    serverRunning,
    serverUrl,
    overlayStatus,
    loading,
    error,
    
    // Computed
    isConnected,
    serverInfo,
    
    // Server methods
    startServer,
    stopServer,
    refreshStatus,
    generateOverlayHtml,
    initialize,
    cleanup,
    
    // For direct access if needed
    setupServerStatusListeners
  };
};
