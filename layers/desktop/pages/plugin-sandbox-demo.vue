<template>
  <div class="plugin-sandbox-demo">
    <h1>Plugin Sandboxing Demo</h1>
    <p class="subtitle">Testing secure plugin iframes with text-processor plugin (VS Code style)</p>

    <!-- Status Section -->
    <div class="status-section">
      <h2>System Status</h2>
      <div class="status-grid">
        <div class="status-card">
          <span class="label">Sandboxed Mode:</span>
          <span class="value success">✓ iframe + blob: URL</span>
        </div>
        <div class="status-card">
          <span class="label">Plugin iframe:</span>
          <span :class="['value', pluginIframeActive ? 'success' : '']">
            {{ pluginIframeActive ? '✓ Active' : 'Not loaded' }}
          </span>
        </div>
        <div class="status-card">
          <span class="label">Message Handler:</span>
          <span :class="['value', isListening ? 'success' : 'error']">
            {{ isListening ? '✓ Listening' : '✗ Not active' }}
          </span>
        </div>
        <div class="status-card">
          <span class="label">Requests Handled:</span>
          <span class="value">{{ requestCount }}</span>
        </div>
      </div>
    </div>

    <!-- Available Plugins -->
    <div class="plugins-section">
      <h2>Available Plugins with Web Extensions</h2>
      <div v-if="availablePlugins.length === 0" class="empty-state">
        No plugins with web extensions found. Install a plugin first.
      </div>
      <div v-else class="plugin-list">
        <div v-for="plugin in availablePlugins" :key="plugin.id" class="plugin-card">
          <div class="plugin-info">
            <h3>{{ plugin.name }}</h3>
            <p class="plugin-id">ID: {{ plugin.id }}</p>
          </div>
          <div class="plugin-actions">
            <button @click="loadPluginInIframe(plugin.id)" :disabled="pluginIframeActive" class="btn btn-primary">
              Load in iframe
            </button>
            <button @click="openPluginInWindowDynamic(plugin.id)" class="btn btn-secondary">
              Open in window
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Actions -->
    <div class="actions-section">
      <h2>Actions</h2>
      <div class="button-group">
        <button @click="refreshPlugins" class="btn btn-accent">
          Refresh Plugin List
        </button>
        <button @click="destroyPluginIframe" :disabled="!pluginIframeActive" class="btn btn-secondary">
          Destroy iframe
        </button>
      </div>
    </div>

    <!-- Embedded iframe Container with Draggable Tab -->
    <div class="embedded-section">
      <div class="tab-bar">
        <div
          v-if="pluginIframeActive"
          class="plugin-tab"
        >
          <span class="tab-icon">🔌</span>
          <span class="tab-title">{{ currentPluginId || 'Plugin' }}</span>
          <button
            @click="currentPluginId && openPluginInWindowDynamic(currentPluginId)"
            class="popout-btn"
            title="Open in separate window"
            :disabled="!currentPluginId"
          >
            ⬜
          </button>
        </div>
        <div v-else class="tab-placeholder">
          No plugin loaded
        </div>
      </div>
      <p class="section-info">
        Plugin runs in sandboxed iframe with NO access to Tauri or parent window memory.
        Communication only via postMessage (like VS Code extensions).
      </p>
      <div
        ref="iframeContainer"
        class="embedded-container"
        :class="{ 'has-iframe': pluginIframeActive }"
      >
        <div v-if="!pluginIframeActive" class="empty-state">
          Click "Create Plugin iframe" to embed plugin UI here
        </div>
        <!-- iframe will be inserted here dynamically -->
      </div>
    </div>


    <!-- Test Results -->
    <div v-if="testResults.length > 0" class="results-section">
      <h2>Test Results</h2>
      <div v-for="(result, index) in testResults" :key="index" class="result-card">
        <div class="result-header">
          <span class="timestamp">{{ result.timestamp }}</span>
          <span :class="['status', result.success ? 'success' : 'error']">
            {{ result.success ? '✓' : '✗' }}
          </span>
        </div>
        <div class="result-body">
          <strong>{{ result.action }}</strong>
          <pre v-if="result.data">{{ result.data }}</pre>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { usePluginExtensionsSandboxed } from '../composables/usePluginExtensionsSandboxed';
import {
  registerPluginFrame,
  usePluginMessageHandler,
} from '../composables/usePluginMessageHandler';
import { invoke } from '@tauri-apps/api/core';
import { generateBridgeScript } from '../utils/pluginBridge';

const { getPluginWebExtension } = usePluginExtensionsSandboxed();

// Message handler for plugin communication (gatekeeper)
const { isListening, requestCount } = usePluginMessageHandler();

interface TestResult {
  timestamp: string;
  action: string;
  success: boolean;
  data?: any;
}

const testResults = ref<TestResult[]>([]);
const iframeContainer = ref<HTMLDivElement | null>(null);
const pluginIframe = ref<HTMLIFrameElement | null>(null);
const pluginIframeActive = computed(() => pluginIframe.value !== null);
const iframeBlobUrl = ref<string | null>(null);
const availablePlugins = ref<Array<{ id: string; name: string }>>([]);
const currentPluginId = ref<string | null>(null);
let unregisterFrame: (() => Promise<void>) | null = null;

// Fetch available plugins with web extensions
async function refreshPlugins() {
  try {
    const result = await invoke('get_detailed_plugin_info');
    const pluginData = result as any;

    console.log('[DEBUG] Raw plugin data from backend:', pluginData);

    if (pluginData?.plugins) {
      console.log('[DEBUG] Total plugins found:', pluginData.plugins.length);
      console.log('[DEBUG] All plugins:', pluginData.plugins.map((p: any) => ({
        id: p.id,
        name: p.name,
        web_extensions: p.web_extensions
      })));

      availablePlugins.value = pluginData.plugins
        .filter((p: any) => p.web_extensions === true)
        .map((p: any) => ({
          id: p.id,
          name: p.name || p.id
        }));

      console.log('[DEBUG] Filtered plugins with web_extensions:', availablePlugins.value);
      addTestResult(`Found ${availablePlugins.value.length} plugins with web extensions`, true);
    } else {
      console.log('[DEBUG] No plugins array in response');
      addTestResult('No plugins array in response', false);
    }
  } catch (error) {
    console.error('[DEBUG] Error fetching plugins:', error);
    addTestResult('Failed to fetch plugins', false, String(error));
  }
}

// Load plugin in iframe
async function loadPluginInIframe(pluginId: string) {
  currentPluginId.value = pluginId;
  await createPluginIframe(pluginId);
}

// Open plugin in window
async function openPluginInWindowDynamic(pluginId: string) {
  try {
    await invoke('open_plugin_window', {
      pluginId
    });
    addTestResult(`Plugin window opened: ${pluginId}`, true);
  } catch (err) {
    console.error('Failed to open plugin window:', err);
    addTestResult('Failed to open plugin window: ' + (err instanceof Error ? err.message : String(err)), false);
  }
}

/**
 * Create a sandboxed iframe for the plugin
 * This is the VS Code-style approach - plugin runs in cross-origin iframe
 * with NO access to Tauri IPC or parent window memory
 */
async function createPluginIframe(pluginId: string) {
  if (!iframeContainer.value || pluginIframe.value) {
    addTestResult('Cannot create iframe', false);
    return;
  }

  addTestResult('Creating sandboxed plugin iframe', true, { pluginId });

  // Bind the inert frame before setting its untrusted blob document.
  const iframe = document.createElement('iframe');
  iframe.sandbox.add('allow-scripts'); // Only allow scripts, nothing else
  iframe.style.cssText = 'width: 100%; height: 100%; border: none; border-radius: 6px;';

  // Clear the container and add iframe
  const emptyState = iframeContainer.value.querySelector('.empty-state') as HTMLElement | null;
  if (emptyState) {
    emptyState.style.display = 'none';
  }
  iframeContainer.value.appendChild(iframe);
  pluginIframe.value = iframe;

  try {
    const sourceWindow = iframe.contentWindow;
    if (!sourceWindow) throw new Error('Plugin frame window is unavailable');
    const registration = await registerPluginFrame(pluginId, sourceWindow, undefined, () => {
      destroyPluginIframe();
      addTestResult('Plugin frame closed because package authority changed', true);
    });
    unregisterFrame = registration.unregister;

    const html = generatePluginIframeHtml(
      registration.session.packageId,
      registration.session.sessionId,
    );
    const blob = new Blob([html], { type: 'text/html' });
    const blobUrl = URL.createObjectURL(blob);
    iframeBlobUrl.value = blobUrl;
    iframe.src = blobUrl;
  } catch (error) {
    destroyPluginIframe();
    addTestResult('Plugin iframe registration failed', false, String(error));
    return;
  }

  addTestResult('Plugin iframe created!', true, {
    sandbox: 'allow-scripts',
    origin: 'blob: (cross-origin)',
    hasTauri: false,
    hasParentAccess: false
  });
}

/**
 * Destroy the plugin iframe
 */
function destroyPluginIframe() {
  if (!pluginIframe.value) return;

  const revoke = unregisterFrame;
  unregisterFrame = null;
  if (revoke) void revoke();

  // Remove iframe from DOM
  pluginIframe.value.remove();
  pluginIframe.value = null;

  // Revoke the blob URL
  if (iframeBlobUrl.value) {
    URL.revokeObjectURL(iframeBlobUrl.value);
    iframeBlobUrl.value = null;
  }

  // Show empty state again
  if (iframeContainer.value) {
    const emptyState = iframeContainer.value.querySelector('.empty-state') as HTMLElement;
    if (emptyState) {
      emptyState.style.display = '';
    }
  }

  addTestResult('Plugin iframe destroyed', true);
}


/**
 * Generate HTML for the plugin iframe
 * This HTML runs in a sandboxed, cross-origin context
 */
function generatePluginIframeHtml(pluginId: string, sessionId: string): string {
  const bridgeScript = generateBridgeScript(pluginId, sessionId);

  return `<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <meta http-equiv="Content-Security-Policy" content="default-src 'self' 'unsafe-inline'; script-src 'unsafe-inline';">
  <title>Plugin: ${pluginId}</title>
  <style>
    * { box-sizing: border-box; }
    body {
      margin: 0;
      padding: 1rem;
      font-family: system-ui, -apple-system, sans-serif;
      background: #1e293b;
      color: #f1f5f9;
      min-height: 100vh;
    }
    h2 { margin: 0 0 1rem 0; color: #38bdf8; font-size: 1.25rem; }
    .security-badge {
      display: inline-flex;
      align-items: center;
      gap: 0.5rem;
      background: #065f46;
      color: #6ee7b7;
      padding: 0.25rem 0.75rem;
      border-radius: 9999px;
      font-size: 0.75rem;
      font-weight: 600;
      margin-bottom: 1rem;
    }
    .info {
      background: #334155;
      padding: 1rem;
      border-radius: 8px;
      margin-bottom: 1rem;
      font-size: 0.875rem;
    }
    .info p { margin: 0.25rem 0; }
    .info strong { color: #94a3b8; }
    .button-group {
      display: flex;
      gap: 0.5rem;
      flex-wrap: wrap;
      margin-bottom: 1rem;
    }
    button {
      background: #3b82f6;
      color: white;
      border: none;
      padding: 0.5rem 1rem;
      border-radius: 4px;
      cursor: pointer;
      font-size: 0.875rem;
      font-weight: 500;
    }
    button:hover { background: #2563eb; }
    button:active { background: #1d4ed8; }
    .result {
      margin-top: 1rem;
      padding: 1rem;
      background: #0f172a;
      border-radius: 4px;
      font-family: monospace;
      font-size: 0.875rem;
      white-space: pre-wrap;
      word-break: break-all;
    }
    .result.success { border-left: 3px solid #10b981; }
    .result.error { border-left: 3px solid #ef4444; }
    .result.pending { border-left: 3px solid #f59e0b; }
  </style>
</head>
<body>
  <div class="security-badge">🔒 Sandboxed iframe</div>
  <h2>Text Processor Suite (${pluginId})</h2>

  <div class="info">
    <p><strong>Mode:</strong> Cross-origin iframe (blob:)</p>
    <p><strong>Sandbox:</strong> allow-scripts only</p>
    <p><strong>Tauri Access:</strong> ❌ None</p>
    <p><strong>Parent Memory:</strong> ❌ Isolated</p>
  </div>

  <div class="button-group">
    <button id="testBtn">Test WASM Call</button>
    <button id="checkApi">Check Security</button>
    <button id="testStorage">Test Storage</button>
  </div>

  <div id="result" class="result pending">Click a button to test the plugin API...</div>

  <script>${bridgeScript}<\/script>
  <script>
    const resultDiv = document.getElementById('result');

    function showResult(text, type = 'success') {
      resultDiv.textContent = text;
      resultDiv.className = 'result ' + type;
    }

    // Test WASM call via postMessage bridge
    document.getElementById('testBtn').addEventListener('click', async () => {
      showResult('Calling WASM via postMessage...', 'pending');
      try {
        // XTP plugins export 'execute' as the main function
        // The plugin expects flow-style parameters
        const result = await window.vortideck.api.callPlugin('execute', {
          inputs: {
            "": "hello from sandboxed iframe!"  // Main input
          },
          params: {
            uppercase: true,
            addPrefix: "🔒 "
          }
        });
        showResult('✓ WASM Result: ' + JSON.stringify(result, null, 2), 'success');
      } catch (err) {
        showResult('✗ Error: ' + err.message, 'error');
      }
    });

    // Check security isolation
    document.getElementById('checkApi').addEventListener('click', () => {
      const checks = {
        'window.vortideck': !!window.vortideck,
        'window.vortideck.api': !!window.vortideck?.api,
        'window.__TAURI__': typeof window.__TAURI__,
        'window.parent === window': window.parent === window,
        'can access parent.document': (() => {
          try { return !!window.parent.document.body; }
          catch { return 'BLOCKED (good!)'; }
        })(),
        'pluginId': window.vortideck?.api?.getPluginId?.() || 'N/A'
      };
      showResult('Security Check:\\n' + JSON.stringify(checks, null, 2), 'success');
    });

    // Test storage API
    document.getElementById('testStorage').addEventListener('click', async () => {
      showResult('Testing storage API...', 'pending');
      try {
        await window.vortideck.api.storage.set('test-key', { saved: true, time: Date.now() });
        const value = await window.vortideck.api.storage.get('test-key');
        showResult('✓ Storage works!\\nSaved: ' + JSON.stringify(value, null, 2), 'success');
      } catch (err) {
        showResult('✗ Storage Error: ' + err.message, 'error');
      }
    });
  <\/script>
</body>
</html>`;
}

function addTestResult(action: string, success: boolean, data?: any) {
  testResults.value.unshift({
    timestamp: new Date().toLocaleTimeString(),
    action,
    success,
    data,
  });
}

// Load plugins on mount
onMounted(() => {
  refreshPlugins();
});

// Cleanup on unmount
onUnmounted(() => {
  destroyPluginIframe();
});
</script>

<style scoped>
.plugin-sandbox-demo {
  padding: 2rem;
  max-width: 1200px;
  margin: 0 auto;
}

h1 {
  font-size: 2rem;
  font-weight: 700;
  margin: 0 0 0.5rem 0;
  color: #1f2937;
}

.subtitle {
  color: #6b7280;
  margin: 0 0 2rem 0;
}

h2 {
  font-size: 1.25rem;
  font-weight: 600;
  margin: 0 0 1rem 0;
  color: #374151;
}

.status-section,
.actions-section,
.plugins-section,
.errors-section,
.results-section {
  margin-bottom: 2rem;
  padding: 1.5rem;
  background: white;
  border-radius: 8px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

.status-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 1rem;
}

.status-card {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 1rem;
  background: #f9fafb;
  border-radius: 6px;
}

.label {
  font-weight: 500;
  color: #6b7280;
}

.value {
  font-weight: 600;
  color: #1f2937;
}

.value.success {
  color: #10b981;
}

.value.error {
  color: #ef4444;
}

.button-group {
  display: flex;
  gap: 0.75rem;
  flex-wrap: wrap;
}

.btn {
  padding: 0.75rem 1.5rem;
  border: none;
  border-radius: 6px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-primary {
  background: #3b82f6;
  color: white;
}

.btn-primary:hover:not(:disabled) {
  background: #2563eb;
}

.btn-secondary {
  background: #6b7280;
  color: white;
}

.btn-secondary:hover:not(:disabled) {
  background: #4b5563;
}

.btn-accent {
  background: #8b5cf6;
  color: white;
}

.btn-accent:hover:not(:disabled) {
  background: #7c3aed;
}

.empty-state {
  padding: 2rem;
  text-align: center;
  color: #9ca3af;
  font-style: italic;
}

.embedded-section {
  margin-bottom: 2rem;
  padding: 1.5rem;
  background: white;
  border-radius: 8px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

.tab-bar {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 0.5rem;
  border-bottom: 1px solid #e5e7eb;
  padding-bottom: 0.5rem;
}

.plugin-tab {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 1rem;
  background: #f3f4f6;
  border-radius: 6px 6px 0 0;
  cursor: grab;
  user-select: none;
  transition: all 0.2s;
}

.plugin-tab:active {
  cursor: grabbing;
  opacity: 0.7;
}

.plugin-tab:hover {
  background: #e5e7eb;
}

.plugin-tab[dragging="true"] {
  opacity: 0.5;
  transform: scale(0.95);
}

.tab-icon {
  font-size: 1rem;
}

.tab-title {
  font-weight: 600;
  font-size: 0.875rem;
  color: #374151;
}

.popout-btn {
  margin-left: auto;
  background: none;
  border: none;
  font-size: 1rem;
  cursor: pointer;
  padding: 0.25rem;
  color: #6b7280;
  transition: color 0.2s;
}

.popout-btn:hover {
  color: #3b82f6;
}

.tab-placeholder {
  padding: 0.5rem 1rem;
  color: #9ca3af;
  font-size: 0.875rem;
  font-style: italic;
}

.embedded-container {
  height: 600px;
  border: 2px dashed #d1d5db;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #f9fafb;
  position: relative;
  overflow: hidden;
}

.embedded-container.has-iframe {
  border-style: solid;
  border-color: #10b981;
  background: transparent;
  /* Remove flex centering when iframe is active */
  display: block;
  align-items: stretch;
  justify-content: stretch;
}

.embedded-container.drag-over {
  border-color: #3b82f6;
  background: rgba(59, 130, 246, 0.05);
}

.embedded-container.has-iframe iframe {
  width: 100%;
  height: 100%;
}

.section-info {
  font-size: 0.875rem;
  color: #6b7280;
  margin: 0 0 1rem 0;
}

.plugin-list {
  display: grid;
  gap: 1rem;
}

.plugin-card {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 1rem;
  background: #f9fafb;
  border: 1px solid #e5e7eb;
  border-radius: 8px;
}

.plugin-info h3 {
  margin: 0 0 0.25rem 0;
  font-size: 1rem;
  font-weight: 600;
  color: #1f2937;
}

.plugin-id {
  margin: 0;
  font-size: 0.875rem;
  color: #6b7280;
  font-family: monospace;
}

.plugin-actions {
  display: flex;
  gap: 0.5rem;
}

.plugin-item {
  padding: 1rem;
  border: 1px solid #e5e7eb;
  border-radius: 6px;
  background: #f9fafb;
}

.plugin-info {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.badge {
  padding: 0.25rem 0.75rem;
  background: #10b981;
  color: white;
  border-radius: 9999px;
  font-size: 0.75rem;
  font-weight: 600;
}

.error-card {
  padding: 1rem;
  background: #fef2f2;
  border-left: 4px solid #ef4444;
  border-radius: 4px;
  margin-bottom: 0.75rem;
  color: #991b1b;
}

.result-card {
  padding: 1rem;
  background: #f9fafb;
  border-radius: 6px;
  margin-bottom: 0.75rem;
}

.result-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.5rem;
}

.timestamp {
  font-size: 0.875rem;
  color: #6b7280;
}

.status {
  font-weight: 600;
}

.result-body strong {
  display: block;
  margin-bottom: 0.5rem;
  color: #374151;
}

pre {
  margin: 0;
  padding: 0.75rem;
  background: white;
  border: 1px solid #e5e7eb;
  border-radius: 4px;
  font-size: 0.875rem;
  overflow-x: auto;
}
</style>
