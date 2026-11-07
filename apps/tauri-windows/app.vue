<template>
  <Html
    :class="[
      'select-none bg-themeBackground',
      mainStore.getTheme === 'dark' ? 'theme-default-dark' : 'theme-default-light',
    ]"
  >
    <meta name="google-adsense-account" content="ca-pub-4811238198188616" />
    <NuxtLayout>
      <NuxtLoadingIndicator :height="5" :duration="3000" :throttle="400" />
      <NuxtPage class="" />
      <template v-if="currentWindow.label === 'main'">
        <Notifications :notifications="mainStore.getOpenNotificationsForWindow('main')" />
        <LazyModalReleaseNotes
          v-if="mainStore.getUpdate?.update"
          :isOpen="true"
          :update="mainStore.getUpdate?.update"
        />
        <AnalyticsConsentBanner />
      </template>

      <!-- Show notifications in flow editor window as well -->
      <template v-if="currentWindow.label === 'flow_editor'">
        <Notifications
          :notifications="mainStore.getOpenNotificationsForWindow('flow_editor')"
          placement="top-right"
        />
        <WorkflowApprovalCenter />
      </template>


      <!-- Global Context Menu -->
      <GlobalContextMenu />

      <!-- Global Capability Dialog (package install capability review) -->
      <LazyModalCapabilityDialog
        v-if="capabilityDialogOpen && capabilityDialogManifest"
        :show="capabilityDialogOpen"
        :manifest="capabilityDialogManifest"
        :busy="capabilityDialogBusy"
        :error="capabilityDialogError"
        :progress="capabilityDialogProgress"
        :status="capabilityDialogStatus"
        @grant="handleCapabilityGrant"
        @cancel="handleCapabilityCancel"
      />

      <!-- Global Settings Modal -->
      <LazyModalSettingsModal v-if="settingsModal.isOpen.value" />
    </NuxtLayout>
  </Html>
</template>

<script lang="ts" setup>
import {
  getAllWebviewWindows,
  getCurrentWebviewWindow,
} from "@tauri-apps/api/webviewWindow";
import { useMainStore } from "@/stores/Main";
import { useSettingsModal } from "../../layers/desktop/composables/useSettingsModal";
import { useConnectionStore } from "@/stores/Connection";
import { useAIModelsStore } from "@/stores/AIModels";
import { usePluginsStore } from "../../layers/desktop/stores/plugins";
import { listen } from "@tauri-apps/api/event";
import { useDesktopUtils } from "../../layers/desktop/stores/desktopUtils";
import { usePluginMessageHandler } from "../../layers/desktop/composables/usePluginMessageHandler";
import { invoke } from "@tauri-apps/api/core";
import { useCapabilityDialog } from "../../layers/desktop/composables/useCapabilityDialog";
import type { PeerInfo } from "../../layers/base/types/Peer";
import type { NotificationInput } from "../../layers/base/types/Notification";
import { ref } from "vue";
import {
  isPermissionGranted,
  onAction as onNativeNotificationAction,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import WorkflowApprovalCenter from "../../layers/flow/components/Flow/Modal/ApprovalCenter.vue";
import {
  canReviewApprovalOnDesktop,
  type WorkflowApprovalFlowEvent,
  type WorkflowApprovalWake,
} from "../../layers/flow/types/workflowApproval";
import { useStateBridgeSync } from "../../layers/desktop/composables/useStateBridgeSync";

interface AppConfigResponse {
  data: {
    language: string;
    mcp_enabled: boolean;
  };
  status: number;
  message: string;
}

type AppLocale = "en" | "nl" | "pl";

const appLocale = (language: string): AppLocale =>
  language === "nl" || language === "pl" ? language : "en";

interface McpServerConnectedPayload {
  server_name: string;
  tools_count: number;
}

interface ContextMenuResultPayload {
  success: boolean;
  title: string;
  message: string;
}

interface AiProviderConfigEntry {
  apiKey?: string;
  provider?: string;
  url?: string;
  modelsUrl?: string;
}

const { setLocale } = useI18n();
const { finalizePendingLocaleChange } = useI18n();

const desktopUtils = useDesktopUtils();
const mainStore = useMainStore();
const settingsModal = useSettingsModal();
const pluginsStore = usePluginsStore();
const { refreshVolatileState } = useStateBridgeSync();
// Global Capability Dialog controller (mounted once below; opened from any install flow)
// Destructured so the refs auto-unwrap in the template (plain objects don't).
const {
  isOpen: capabilityDialogOpen,
  isGranting: capabilityDialogBusy,
  grantError: capabilityDialogError,
  grantProgress: capabilityDialogProgress,
  grantStatus: capabilityDialogStatus,
  currentManifest: capabilityDialogManifest,
  grant: handleCapabilityGrant,
  cancel: handleCapabilityCancel,
} = useCapabilityDialog();

// Set up plugin message handler for iframe communication (main window and plugin windows)
const currentWindowLabel = getCurrentWebviewWindow().label;
const APPROVAL_NOTIFICATION_PREFIX = "workflow-approval:";
const nativeApprovalNotificationWakeIds = new Set<string>();

const reviewWorkflowApproval = async (wakeId: string) => {
  try {
    await invoke("open_workflow_approval", { wakeId });
  } catch (error) {
    mainStore.addNotification(
      {
        title: "Could not open approval",
        message: String(error),
        type: "error",
      },
      "main",
    );
  }
};

const addApprovalNotification = (wake: WorkflowApprovalWake) => {
  if (!canReviewApprovalOnDesktop(wake)) return;
  mainStore.addNotification(
    {
      key: `${APPROVAL_NOTIFICATION_PREFIX}${wake.id}`,
      title: wake.payload?.title || "Workflow approval required",
      message: wake.payload?.message || "A workflow is paused until you approve or deny it.",
      buttonText: "Review",
      buttonFunction: () => { void reviewWorkflowApproval(wake.id); },
      autoClose: false,
      duration: 0,
      type: "warning",
      color: "bg-amber-500",
    },
    "main",
  );
};

const approvalWindowFocus = async () => {
  try {
    const windows = await getAllWebviewWindows();
    const focus = await Promise.all(
      windows.map(async (window) => ({
        label: window.label,
        focused: await window.isFocused(),
      })),
    );
    return {
      anyFocused: focus.some(({ focused }) => focused),
      flowEditorFocused: focus.some(
        ({ label, focused }) => label === "flow_editor" && focused,
      ),
    };
  } catch {
    return { anyFocused: false, flowEditorFocused: false };
  }
};

const sendNativeApprovalNotification = async (wake: WorkflowApprovalWake) => {
  if (!mainStore.getNativeWorkflowApprovalNotifications) return;
  if (nativeApprovalNotificationWakeIds.has(wake.id)) return;
  const { anyFocused } = await approvalWindowFocus();
  if (anyFocused || !(await isPermissionGranted())) return;

  sendNotification({
    title: wake.payload?.title || "Workflow approval required",
    body: wake.payload?.message || "A workflow is waiting for your decision.",
    group: "workflow-approvals",
    autoCancel: true,
    extra: { wakeId: wake.id },
  });
  nativeApprovalNotificationWakeIds.add(wake.id);
};

const syncPendingApprovalNotifications = async () => {
  if (currentWindowLabel !== "main") return;
  try {
    const wakes = await invoke<WorkflowApprovalWake[]>("list_workflow_wakes");
    const pending = wakes.filter(canReviewApprovalOnDesktop);
    const pendingWakeIds = new Set(pending.map(({ id }) => id));
    for (const wakeId of nativeApprovalNotificationWakeIds) {
      if (!pendingWakeIds.has(wakeId)) nativeApprovalNotificationWakeIds.delete(wakeId);
    }
    const activeKeys = pending.map(({ id }) => `${APPROVAL_NOTIFICATION_PREFIX}${id}`);
    mainStore.removeNotificationsByKeyPrefix(APPROVAL_NOTIFICATION_PREFIX, activeKeys);
    for (const wake of pending) addApprovalNotification(wake);
  } catch (error) {
    console.error("Could not synchronize pending workflow approvals:", error);
  }
};

if (currentWindowLabel === "main" || currentWindowLabel.startsWith("plugin-")) {
  usePluginMessageHandler();
}

listen("obs_plugin_status", (event) => {
  const status = event.payload as boolean;
  if (status === true) {
    desktopUtils.setIsObsPluginActive(true);
  } else {
    desktopUtils.setIsObsPluginActive(false);
  }
});

// Listen for plugin changes (install/uninstall via file watcher)
listen("plugin-changed", async (event) => {
  console.log("🔄 Plugin changed, refreshing package store:", event.payload);
  await pluginsStore.refresh();
});

listen<{ droppedEvents: number }>("state_bridge_resync_required", async (event) => {
  console.warn(
    `State update bridge dropped ${event.payload.droppedEvents} event(s); refreshing authoritative snapshots`,
  );
  try {
    await refreshVolatileState();
  } catch (error) {
    console.error("Could not refresh state bridge snapshots:", error);
  }
});

const route = useRoute();

listen<AppConfigResponse>("app_config_updated", async (event) => {
  console.log(event.payload);
  await setLocale(appLocale(event.payload.data.language));
});

// Listen for theme changes from other windows
listen("theme-changed", (event) => {
  const { theme } = event.payload as { theme: string };

  // Disable transitions temporarily for instant theme switching
  const html = document.documentElement;
  html.classList.add("disable-transitions");

  // Update theme in this window's store
  mainStore.theme = theme;

  // Re-enable transitions after sufficient time for theme to fully apply
  requestAnimationFrame(() => {
    requestAnimationFrame(() => {
      html.classList.remove("disable-transitions");
    });
  });
});

void listen<McpServerConnectedPayload>("mcp-server-connected", (event) => {
  console.log(`MCP Server Connected:`, event.payload);

  mainStore.addNotification({
    title: "MCP Server Connected",
    message: `Connected to MCP server: ${event.payload.server_name} (${event.payload.tools_count} tools)`,
    buttonText: "",
    buttonFunction: () => {},
    autoClose: true,
    duration: 10000,
    type: "success",
    color: "",
  });

  // Show success notification
}).catch((error) => console.error("Could not register MCP connection listener:", error));

void listen<ContextMenuResultPayload>("context-menu-result", (event) => {
  const { success, title, message } = event.payload;
  mainStore.addNotification({
    title: success ? title : `Failed: ${title}`,
    message,
    buttonText: "",
    buttonFunction: () => {},
    autoClose: true,
    duration: success ? 4000 : 8000,
    type: success ? "success" : "error",
    color: "",
  });
}).catch((error) => console.error("Could not register context-menu result listener:", error));

void listen<{ notification: NotificationInput; windowIds?: string | string[] }>("vortideck:cross-window-notification", (event) => {
  if (currentWindowLabel !== "main" && currentWindowLabel !== "flow_editor") return;
  const targetWindows: Array<string | undefined> = event.payload.windowIds
    ? (Array.isArray(event.payload.windowIds) ? event.payload.windowIds : [event.payload.windowIds])
    : [undefined];
  if (targetWindows.includes(undefined) || targetWindows.includes(currentWindowLabel)) {
    const open = mainStore.getOpenNotificationsForWindow(currentWindowLabel);
    const isDuplicate = open.some(
      (n) => n.title === event.payload.notification.title && n.message === event.payload.notification.message,
    );
    if (!isDuplicate) {
      mainStore.addNotification(event.payload.notification, currentWindowLabel);
    }
  }
}).catch((error) => console.error("Could not register cross-window notification listener:", error));


void listen<WorkflowApprovalFlowEvent>("flow_event", async ({ payload }) => {
  if (currentWindowLabel !== "main") return;
  if (payload.type === "approvalRequested" && payload.request) {
    const { flowEditorFocused } = await approvalWindowFocus();
    if (!flowEditorFocused) addApprovalNotification(payload.request);
    await sendNativeApprovalNotification(payload.request);
  } else if (payload.type === "approvalResolved" || payload.type === "flowCancelled") {
    if (payload.wakeId) nativeApprovalNotificationWakeIds.delete(payload.wakeId);
    void syncPendingApprovalNotifications();
  }
}).catch((error) => console.error("Could not register workflow approval listener:", error));

if (currentWindowLabel === "main") {
  void onNativeNotificationAction((action) => {
    const wakeId = action.notification?.extra?.wakeId;
    if (typeof wakeId === "string") void reviewWorkflowApproval(wakeId);
  }).catch((error) => console.error("Could not register native notification listener:", error));
}

let appConfig: AppConfigResponse = {
  data: { language: "en", mcp_enabled: false },
  status: 0,
  message: "App configuration has not loaded yet",
};

const loadAppConfig = async () => {
  try {
    appConfig = await invoke<AppConfigResponse>("get_app_config");
    console.log(JSON.stringify(appConfig.data.language));
    await setLocale(appLocale(appConfig.data.language));
  } catch (error) {
    console.error("Could not load app configuration; using safe defaults:", error);
  }
};

// alert(JSON.stringify(appConfig.data));

// mainStore.addNotification({
//   title: "Update v1.0.14 available",
//   message: "Test updater",
//   buttonText: "Install update and relaunch",
//   buttonFunction: () => {
//     console.log("Test");
//   },
//   autoClose: false,
// });
// console.log("useAutoUpdate");

const currentWindow = getCurrentWebviewWindow();

// Initialize AI models store on app launch
const aiModelsStore = useAIModelsStore();
interface RecoveryRunSummary {
  runId?: string;
  flowId: string;
}

const notifyAboutRecoveryQueue = async () => {
  if (getCurrentWebviewWindow().label !== "main") return;

  try {
    const runs = await invoke<RecoveryRunSummary[]>("list_flow_recovery_runs");
    if (runs.length === 0) return;

    mainStore.addNotification(
      {
        id: 0,
        title: "Workflow recovery required",
        message: `${runs.length} interrupted workflow ${runs.length === 1 ? "run requires" : "runs require"} a safe recovery decision. Nothing will be replayed automatically.`,
        type: "warning",
        autoClose: false,
        duration: 0,
        buttonText: "Review in Flow Editor",
        buttonFunction: () => {
          void invoke("open_flow_editor", { flowId: null });
        },
        color: "bg-amber-500",
      },
      "main",
    );
  } catch (error) {
    console.error("Could not inspect the workflow recovery queue:", error);
  }
};

const initializeAIModels = async () => {
  try {
    let providerConfigs = [];

    try {
      // Use same path as settings.vue: resources/mcp/config.json in AppConfig
      const { readTextFile, BaseDirectory } = await import("@tauri-apps/plugin-fs");
      const configContent = await readTextFile("resources/mcp/config.json", {
        baseDir: BaseDirectory.AppConfig,
      });
      const config = JSON.parse(configContent) as {
        models?: { model?: AiProviderConfigEntry[] };
      };

      // Extract provider configs with API keys
      if (config?.models?.model) {
        providerConfigs = config.models.model
          .filter((m) => m.apiKey && m.provider)
          .map((m) => ({
            provider: m.provider,
            apiKey: m.apiKey,
            baseUrl: m.url, // Pass the base URL from config
            modelsUrl: m.modelsUrl, // Pass custom models URL if set
          }));
      }

      console.log(
        "🔄 Initializing AI models with",
        providerConfigs.length,
        "provider(s)"
      );
    } catch (err) {
      console.log("ℹ️ No MCP config found, skipping model initialization");
      return;
    }

    if (providerConfigs.length > 0) {
      // Main window: allow refreshing from APIs if cache is stale
      await aiModelsStore.initialize(providerConfigs, true);
      console.log("✅ AI models store initialized");
    }
  } catch (error) {
    console.error("❌ Failed to initialize AI models:", error);
  }
};

// useMcpSession(appConfig.data.mcp_enabled);
onMounted(async () => {
  await loadAppConfig();

  // Start Tauri sync for mainStore
  await mainStore.$tauri.start();

  // Approval notifications are transient projections of durable wakes. Query
  // the backend after store hydration so a restart shows only requests that
  // are still actually waiting.
  await syncPendingApprovalNotifications();

  // Start Tauri sync for AI models store (enables cross-window sync)
  await aiModelsStore.$tauri.start();
  console.log("✅ AI Models store Tauri sync enabled");

  await notifyAboutRecoveryQueue();

  // Initialize AI models in background
  initializeAIModels();

  // Note: Plugin store initialization happens AFTER backend plugins are loaded
  // For splashscreen: after trigger_plugin_loading completes (line 285)
  // For other windows: plugins should already be loaded from splashscreen
  // The store will auto-fetch on demand (dropdown, marketplace) if needed

  if (getCurrentWebviewWindow().label === "splashscreen") {
    useMcpSession(appConfig.data.mcp_enabled);
    setupWindow();
    const updateTaskId = "check-updates";
    const pluginTaskId = "load-plugins";
    let retryCount = 0;
    const maxRetries = 3;
    const retryDelay = 5000; // 5 seconds between retries

    // Promise that resolves when plugin loading is fully complete
    let resolvePluginLoading: (() => void) | null = null;
    const pluginLoadingComplete = new Promise<void>((resolve) => {
      resolvePluginLoading = resolve;
    });

    // Set up listener FIRST (await to prevent race condition with events)
    await listen("plugin-loading-progress", (event) => {
      const payload = event.payload as {
        status: string;
        message?: string;
        current?: number;
        total?: number;
      };

      if (
        payload.status === "progress" &&
        payload.current !== undefined &&
        payload.total !== undefined
      ) {
        const progress = payload.total > 0 ? (payload.current / payload.total) * 100 : 0;
        desktopUtils.updateSplashscreenTask(pluginTaskId, {
          progress,
          message: `Loaded ${payload.current}/${payload.total}`,
        });
      } else if (payload.status === "completed") {
        const progress =
          payload.current !== undefined &&
          payload.total !== undefined &&
          payload.total > 0
            ? (payload.current / payload.total) * 100
            : 100;
        desktopUtils.updateSplashscreenTask(pluginTaskId, {
          status: "completed",
          progress,
          message: payload.message || "Plugins loaded",
        });

        // Resolve the promise to signal plugin loading is done
        if (resolvePluginLoading) {
          resolvePluginLoading();
        }
      }
    });

    // Load plugins in parallel (no dependency on update check)
    const loadPlugins = async () => {
      try {
        // Add plugin loading task
        desktopUtils.addSplashscreenTask({
          id: pluginTaskId,
          name: "Loading plugins",
          status: "running",
          progress: 0,
          message: "Initializing...",
        });

        // Trigger plugin loading (this just starts the process)
        await invoke("trigger_plugin_loading");

        // Wait for the actual "completed" event from the backend (with safety fallback timeout)
        await Promise.race([
          pluginLoadingComplete,
          new Promise<void>((resolve) => setTimeout(resolve, 8000)),
        ]);

        console.log("✅ Plugins loaded successfully");

        // Fetch plugin info for global store (after plugins are loaded)
        await pluginsStore.fetchPlugins();
        console.log("✅ Plugin store initialized");
      } catch (error) {
        console.error("Failed to load plugins:", error);
        desktopUtils.updateSplashscreenTask(pluginTaskId, {
          status: "failed",
          message: "Failed to load plugins",
        });
      }
    };

    // Check for updates with retries (runs in parallel with plugin loading)
    const checkForUpdates = async () => {
      // Create update check task
      desktopUtils.addSplashscreenTask({
        id: updateTaskId,
        name: "Checking for updates",
        status: "running",
        message: "Looking for new versions",
      });

      // Retry loop for update check
      while (retryCount <= maxRetries) {
        try {
          await useInstallUpdate();

          // Success - mark as completed
          desktopUtils.updateSplashscreenTask(updateTaskId, {
            status: "completed",
            message: "Update check completed",
          });

          // Remove task after delay to let user see completion
          //setTimeout(() => {
          //  desktopUtils.removeSplashscreenTask(updateTaskId);
          //}, 1400);

          console.log("✅ Update check completed");
          return;
        } catch (error) {
          console.error(
            `Update check failed (attempt ${retryCount + 1}/${maxRetries}):`,
            error
          );

          if (retryCount < maxRetries) {
            retryCount++;
            desktopUtils.updateSplashscreenTask(updateTaskId, {
              name: "Checking for updates",
              status: "running",
              message: `Waiting for internet connection (retry ${retryCount}/${maxRetries})...`,
            });

            // Wait before retrying
            await new Promise((resolve) => setTimeout(resolve, retryDelay));
          } else {
            // Max retries reached - mark as failed but continue
            desktopUtils.updateSplashscreenTask(updateTaskId, {
              name: "Checking for updates error",
              status: "failed",
              message:
                "Couldn't find any updates, please connect to the internet and restart",
            });
            console.log("❌ Update check failed after max retries");
            return;
          }
        }
      }
    };

    // Run both tasks in parallel for faster startup
    await Promise.all([loadPlugins(), checkForUpdates()]);

    // Notify backend that frontend is ready (after both tasks complete)
    // Remove task after delay to let user see completion
    setTimeout(async () => {
      await invoke("set_complete", { task: "frontend" });
    }, 1400);
  } else {
    // alert(JSON.stringify(mainStore.getUpdate));
    await invoke("show_window_if_complete", { windowLabel: "main" });
  }

  // Global shortcut for Settings (Ctrl+, or Cmd+,)
  window.addEventListener("keydown", (e: KeyboardEvent) => {
    if ((e.ctrlKey || e.metaKey) && e.key === ",") {
      e.preventDefault();
      settingsModal.toggleSettings();
    }
  });
});

async function setupWindow() {
  const win = getCurrentWebviewWindow();
  try {
    await win.center();
  } catch {
    // Centering is best-effort when the native window is still initializing.
  }
  await win.show();
}
// import { listen } from "@tauri-apps/api/event";
// import { useMobileStore } from "@/stores/Mobile";

const connectionStore = useConnectionStore();
listen<PeerInfo>("new_peer_added", (event) => {
  console.log(`peer ${event.payload.device_type} added`);
  console.log(event.payload);
  connectionStore.addPeer(event.payload);
  console.log(connectionStore.paired);
});

listen("toggle_is_simulator", (event) => {
  const new_status = event.payload as boolean;
  desktopUtils.setIsSimulatorOpen(new_status);
});

listen<PeerInfo>("new_active_peer_added", (event) => {
  console.log(`peer ${event.payload.device_type} added`);
  console.log(event.payload);
  connectionStore.addActivePeer(event.payload);
  console.log(connectionStore.paired_active);
});

listen<PeerInfo>("active_peer_updated", (event) => {
  console.log(`peer ${event.payload.device_type} updated/reconnected`);
  console.log(event.payload);
  connectionStore.addActivePeer(event.payload); // This will update existing peer
  console.log(connectionStore.paired_active);
});

listen<PeerInfo | string>("active_peer_removed", (event) => {
  const peerLabel = typeof event.payload === "string"
    ? event.payload
    : event.payload.device_type;
  console.log(`peer ${peerLabel} removed`);
  console.log(event.payload);
  connectionStore.removeActivePeer(event.payload);
  console.log(connectionStore.paired_active);
});

// Listen for deep link navigation events
listen("navigate-to-page", (event) => {
  const pagePath = event.payload as string;
  console.log(`Deep link navigation to: ${pagePath}`);

  // Use Nuxt's navigateTo for SPA navigation
  if (pagePath) {
    navigateTo(`/${pagePath}`);
  }
});


// Context menu actions are now executed entirely in Rust via UniversalExecutionEngine.
// The shell extension sends vortideck://context-menu/item-{id} deep links,
// and Rust looks up the item, parses the stored Action JSON, and executes directly.
</script>

<style lang="scss">
/* Studio stabilization reload trigger */
</style>
