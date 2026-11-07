<template>
  <div class="plugin-window">
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <p>Loading plugin...</p>
    </div>
    <div v-else-if="error" class="error-state">
      <p class="error-message">{{ error }}</p>
    </div>
    <div v-else ref="iframeContainer" class="iframe-container"></div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { useRoute } from "vue-router";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { generateBridgeScript } from "../utils/pluginBridge";
import { usePluginExtensionsSandboxed } from "../composables/usePluginExtensionsSandboxed";
import {
  registerPluginFrame,
  usePluginMessageHandler,
  type IssuedPluginFrameSession,
} from "../composables/usePluginMessageHandler";
import {
  permitsPluginEvent,
  permitsStreamEvent,
  type PluginEventScope,
  type ScopedPluginEvent,
} from "../utils/pluginEventScope";

const route = useRoute();
const pluginId = ref<string | null>(null);
const loading = ref(true);
const error = ref<string | null>(null);
const iframeContainer = ref<HTMLDivElement | null>(null);
const pluginIframe = ref<HTMLIFrameElement | null>(null);
const iframeBlobUrl = ref<string | null>(null);
const frameSession = ref<IssuedPluginFrameSession | null>(null);
let unregisterFrame: (() => Promise<void>) | null = null;

const { getPluginWebExtension } = usePluginExtensionsSandboxed();
usePluginMessageHandler();
const unlistenFns = ref<Array<() => void>>([]);

onMounted(async () => {
  // Get plugin ID from URL query
  pluginId.value = route.query.id as string;

  if (!pluginId.value) {
    error.value = "No plugin ID provided";
    loading.value = false;
    return;
  }

  try {
    // Set loading to false first so the container renders
    loading.value = false;

    // Wait for next tick to ensure DOM is updated
    await nextTick();

    // Now create the iframe
    await createPluginIframe(pluginId.value);

    const session = frameSession.value;
    if (!session) throw new Error("Plugin frame session was not established");
    const scope = await invoke<PluginEventScope>("get_plugin_frame_scope", {
      sessionId: session.sessionId,
    });

    // Flow and generic events are intentionally not exposed: they contain host
    // data without a named plugin subscription contract. Stream delivery is
    // exact-type filtered against the native effective grant.
    const unlisten = await listen<Record<string, unknown>>("stream_event", (event) => {
      if (!permitsStreamEvent(scope, event.payload)) return;
      const eventType = String(event.payload.type);
      if (pluginIframe.value?.contentWindow && frameSession.value === session) {
        pluginIframe.value.contentWindow.postMessage({
          source: "vortideck-event",
          sessionId: session.sessionId,
          eventType,
          eventData: event.payload,
        }, "*");
      }
    });
    unlistenFns.value.push(unlisten);

    const unlistenPlugin = await listen<ScopedPluginEvent>("plugin_event", (event) => {
      if (!permitsPluginEvent(scope, session.packageId, event.payload)
        || !pluginIframe.value?.contentWindow
        || frameSession.value !== session) return;
      pluginIframe.value.contentWindow.postMessage({
        source: "vortideck-event",
        sessionId: session.sessionId,
        eventType: event.payload.eventType,
        eventData: event.payload?.payload,
      }, "*");
    });
    unlistenFns.value.push(unlistenPlugin);
  } catch (err) {
    destroyPluginIframe();
    error.value = err instanceof Error ? err.message : String(err);
  }
});

onUnmounted(() => {
  destroyPluginIframe();
});

definePageMeta({
  layout: "clean",
});

/**
 * Create sandboxed iframe for the plugin
 */
async function createPluginIframe(pid: string) {
  if (!iframeContainer.value) {
    throw new Error("iframe container not found");
  }

  // Get plugin's web extension HTML (combined HTML/CSS/JS)
  const webExtensionHtml = await getPluginWebExtension(pid);

  // Generate the complete HTML with bridge script
  // Append an inert frame first so the exact WindowProxy can be registered
  // before any untrusted plugin script starts executing.
  const iframe = document.createElement("iframe");
  iframe.sandbox.add("allow-scripts"); // Only allow scripts, nothing else
  iframe.style.cssText = "width: 100%; height: 100%; border: none;";
  iframeContainer.value.appendChild(iframe);
  pluginIframe.value = iframe;

  const sourceWindow = iframe.contentWindow;
  if (!sourceWindow) throw new Error("Plugin frame window is unavailable");
  const registration = await registerPluginFrame(pid, sourceWindow, undefined, () => {
    destroyPluginIframe();
    error.value = "Plugin access changed. Reopen this window after reviewing the package.";
  });
  frameSession.value = registration.session;
  unregisterFrame = registration.unregister;

  const completeHtml = generatePluginIframeHtml(
    registration.session.packageId,
    registration.session.sessionId,
    webExtensionHtml,
  );
  const blob = new Blob([completeHtml], { type: "text/html" });
  const blobUrl = URL.createObjectURL(blob);
  iframeBlobUrl.value = blobUrl;
  iframe.src = blobUrl;

  console.log(`✅ Plugin iframe created for: ${pid}`);
}

/**
 * Destroy the plugin iframe and clean up resources
 */
function destroyPluginIframe() {
  for (const unlisten of unlistenFns.value.splice(0)) unlisten();
  const revoke = unregisterFrame;
  unregisterFrame = null;
  frameSession.value = null;
  if (revoke) void revoke();

  if (pluginIframe.value) {
    pluginIframe.value.remove();
    pluginIframe.value = null;
  }

  if (iframeBlobUrl.value) {
    URL.revokeObjectURL(iframeBlobUrl.value);
    iframeBlobUrl.value = null;
  }
}

/**
 * Generate HTML for the plugin iframe
 * This HTML runs in a sandboxed, cross-origin context
 */
function generatePluginIframeHtml(
  pid: string,
  sessionId: string,
  webExtensionHtml: string,
): string {
  const bridgeScript = generateBridgeScript(pid, sessionId);

  // Build HTML string with escaped closing tags to avoid Vue parser issues
  const htmlParts = [
    "<!DOCTYPE html>",
    "<html>",
    "<head>",
    '  <meta charset="UTF-8">',
    '  <meta name="viewport" content="width=device-width, initial-scale=1.0">',
    `  <title>Plugin: ${pid}<` + `/title>`,
    "  <style>",
    "    * {",
    "      margin: 0;",
    "      padding: 0;",
    "      box-sizing: border-box;",
    "    }",
    "    body {",
    '      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;',
    "      overflow: auto;",
    "    }",
    "  <" + `/style>`,
    "<" + `/head>`,
    "<body>",
    "  <!-- Plugin Bridge Script (injected BEFORE plugin code) -->",
    "  <script>",
    `    ${bridgeScript}`,
    "  <" + `/script>`,
    "",
    "  <!-- Standard plugin mount points (always provided) -->",
    "  <div id=\"plugin-root\">",
    "    <div id=\"app\"><" + `/div>`,
    "  <" + `/div>`,
    "",
    "  <!-- Plugin's Web Extension Content (HTML/CSS/JS) -->",
    `  ${webExtensionHtml}`,
    "<" + `/body>`,
    "<" + `/html>`,
  ];

  return htmlParts.join("\n");
}
</script>

<style scoped>
.plugin-window {
  width: 100vw;
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: #f9fafb;
  overflow: hidden;
}

.loading-state,
.error-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  gap: 1rem;
}

.spinner {
  width: 48px;
  height: 48px;
  border: 4px solid #e5e7eb;
  border-top-color: #3b82f6;
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.error-message {
  color: #dc2626;
  font-size: 1rem;
  padding: 1rem 2rem;
  background: #fef2f2;
  border-radius: 8px;
  border: 1px solid #fecaca;
}

.iframe-container {
  flex: 1;
  width: 100%;
  height: 100%;
  overflow: hidden;
}

.iframe-container iframe {
  width: 100%;
  height: 100%;
  border: none;
}
</style>
