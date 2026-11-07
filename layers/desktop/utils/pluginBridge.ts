/**
 * Plugin Bridge Script
 *
 * This script is injected into every plugin webview to provide a sandboxed API.
 * Plugins use window.vortideck.api to interact with the host application.
 *
 * ARCHITECTURE (like VS Code):
 * - Plugin webview has NO direct access to Tauri invoke()
 * - All calls go through postMessage to main window
 * - Main window acts as gatekeeper and proxies allowed calls
 * - This ensures plugins can ONLY call the API we expose
 */

export interface PluginAPI {
  /** Call a function in this plugin's WASM module */
  callPlugin(functionName: string, data: any): Promise<any>;

  /** Run a flow (requires "flows" permission) */
  runFlow(flowName: string, input: any): Promise<any>;

  /** Plugin-scoped storage (requires "storage" permission) */
  storage: {
    get(key: string): Promise<any>;
    set(key: string, value: any): Promise<void>;
  };

  /** Get the plugin's ID */
  getPluginId(): string;

  /** Register an event listener */
  on(eventType: string, callback: (data: any) => void): void;

  /** Unregister an event listener */
  off(eventType: string, callback: (data: any) => void): void;
}

// Note: createPluginBridge is not used in the webview context
// The webview uses generateBridgeScript which creates a postMessage-based API

/**
 * Generate the bridge script to be injected into a plugin webview
 * This creates a self-contained script that initializes window.vortideck
 *
 * SECURITY: Uses postMessage to communicate with main window.
 * Plugin webview NEVER gets direct access to Tauri invoke().
 * Main window acts as gatekeeper for all plugin API calls.
 *
 * @param pluginId - Host-resolved package ID exposed for display only
 * @param sessionId - Random host-issued authority for this exact frame lifetime
 * @returns JavaScript code to inject
 */
export function generateBridgeScript(
  pluginId: string,
  sessionId: string,
): string {
  return `
(function() {
  'use strict';
  
  const PLUGIN_ID = ${JSON.stringify(pluginId)};
  const SESSION_ID = ${JSON.stringify(sessionId)};
  const HOST_WINDOW = window.opener || window.parent;
  
  console.log('[VortiDeck Plugin Bridge] Initializing for plugin:', PLUGIN_ID);
  console.log('[VortiDeck Plugin Bridge] Using postMessage architecture (like VS Code)');
  
  // Message ID counter for request-response correlation
  let messageId = 0;
  const pendingRequests = new Map();
  const listeners = new Map();
  
  /**
   * Send a request to the main window and wait for response
   * @param {string} type - The request type (e.g., 'callPlugin', 'runFlow')
   * @param {object} payload - The request payload
   * @returns {Promise<any>} The response from main window
   */
  function sendRequest(type, payload) {
    return new Promise((resolve, reject) => {
      const id = ++messageId;
      const timeoutMs = 30000; // 30 second timeout
      
      // Store the pending request
      const timeout = setTimeout(() => {
        pendingRequests.delete(id);
        reject(new Error(\`Plugin API request timed out: \${type}\`));
      }, timeoutMs);
      
      pendingRequests.set(id, { resolve, reject, timeout });
      
      // Send message to parent/opener window
      if (!HOST_WINDOW || HOST_WINDOW === window) {
        pendingRequests.delete(id);
        clearTimeout(timeout);
        reject(new Error('No parent window available for plugin communication'));
        return;
      }
      
      HOST_WINDOW.postMessage({
        source: 'vortideck-plugin',
        sessionId: SESSION_ID,
        pluginId: PLUGIN_ID,
        id,
        type,
        payload
      }, '*');
    });
  }
  
  // Listen for responses and events from main window
  window.addEventListener('message', (event) => {
    const data = event.data;
    if (event.source !== HOST_WINDOW) return;

    if (
      data?.source === 'vortideck-host'
      && data?.sessionId === SESSION_ID
      && data?.pluginId === PLUGIN_ID
      && data?.type === 'sessionRevoked'
    ) {
      for (const pending of pendingRequests.values()) {
        clearTimeout(pending.timeout);
        pending.reject(new Error(data.error || 'Plugin frame session was revoked'));
      }
      pendingRequests.clear();
      window.dispatchEvent(new CustomEvent('vortideck:session-revoked'));
      return;
    }
    
    // Check if it's a forwarded event from the host event bus
    if (data?.source === 'vortideck-event' && data?.sessionId === SESSION_ID) {
      const callbacks = listeners.get(data.eventType);
      if (callbacks) {
        callbacks.forEach(cb => {
          try {
            cb(data.eventData);
          } catch (err) {
            console.error('[VortiDeck Plugin Bridge] Error in event callback:', err);
          }
        });
      }
      return;
    }
    
    // Only handle responses to our requests
    if (
      data?.source !== 'vortideck-host'
      || data?.sessionId !== SESSION_ID
      || data?.pluginId !== PLUGIN_ID
    ) {
      return;
    }
    
    if (!data.id) return;
    
    const pending = pendingRequests.get(data.id);
    if (!pending) {
      console.warn('[VortiDeck Plugin Bridge] Received response for unknown request:', data.id);
      return;
    }
    
    pendingRequests.delete(data.id);
    clearTimeout(pending.timeout);
    
    if (data.error) {
      pending.reject(new Error(data.error));
    } else {
      pending.resolve(data.result);
    }
  });
  
  // Create the plugin API (uses postMessage, NOT invoke)
  window.vortideck = {
    api: {
      /**
       * Call a function in this plugin's WASM module
       * @param {string} functionName - Name of the function to call
       * @param {any} data - Data to pass to the function
       * @returns {Promise<any>} Result from the WASM function
       */
      async callPlugin(functionName, data) {
        console.log(\`[Plugin \${PLUGIN_ID}] Calling WASM function: \${functionName}\`);
        return sendRequest('callPlugin', { functionName, data });
      },
      
      /**
       * Run a flow (requires "flows" permission)
       * @param {string} flowName - Name of the flow to run
       * @param {any} input - Input data for the flow
       * @returns {Promise<any>} Flow result
       */
      async runFlow(flowName, input) {
        console.log(\`[Plugin \${PLUGIN_ID}] Running flow: \${flowName}\`);
        return sendRequest('runFlow', { flowName, input });
      },
      
      /**
       * Plugin-scoped storage (requires "storage" permission)
       */
      storage: {
        /**
         * Get a value from plugin storage
         * @param {string} key - Storage key
         * @returns {Promise<any>} Stored value or null
         */
        async get(key) {
          console.log(\`[Plugin \${PLUGIN_ID}] Storage get: \${key}\`);
          return sendRequest('storageGet', { key });
        },
        
        /**
         * Set a value in plugin storage
         * @param {string} key - Storage key
         * @param {any} value - Value to store
         * @returns {Promise<void>}
         */
        async set(key, value) {
          console.log(\`[Plugin \${PLUGIN_ID}] Storage set: \${key}\`);
          return sendRequest('storageSet', { key, value });
        }
      },
      
      /**
       * Get the plugin's ID
       * @returns {string} Plugin ID
       */
      getPluginId() {
        return PLUGIN_ID;
      },
      
      /**
       * Register a callback for events
       * @param {string} eventType - The type of event (e.g. 'stream_event', 'generic_event')
       * @param {function} callback - The callback function
       */
      on(eventType, callback) {
        console.log(\`[Plugin \${PLUGIN_ID}] Subscribing to event: \${eventType}\`);
        if (!listeners.has(eventType)) {
          listeners.set(eventType, []);
        }
        listeners.get(eventType).push(callback);
      },
      
      /**
       * Unregister a callback for events
       * @param {string} eventType - The type of event
       * @param {function} callback - The callback function to remove
       */
      off(eventType, callback) {
        console.log(\`[Plugin \${PLUGIN_ID}] Unsubscribing from event: \${eventType}\`);
        const list = listeners.get(eventType);
        if (list) {
          const index = list.indexOf(callback);
          if (index !== -1) {
            list.splice(index, 1);
          }
        }
      }
    }
  };
  
  console.log('[VortiDeck Plugin Bridge] API initialized successfully (postMessage mode)');
  
  // Dispatch ready event for the plugin to know API is available
  window.dispatchEvent(new CustomEvent('vortideck:ready', {
    detail: { pluginId: PLUGIN_ID }
  }));
})();
  `.trim();
}
