/**
 * Source-bound API gateway for sandboxed plugin frames.
 *
 * A frame never chooses its authority. The trusted host page asks Rust for a
 * session, binds that session to the exact WindowProxy, and Rust resolves the
 * package principal again for every privileged command.
 */

import { onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface IssuedPluginFrameSession {
  sessionId: string;
  packageId: string;
  expiresAtMs: number;
}

interface PluginRequest {
  source: "vortideck-plugin";
  sessionId: string;
  pluginId?: string;
  id: number;
  type: string;
  payload?: unknown;
}

interface PluginResponse {
  source: "vortideck-host";
  sessionId: string;
  pluginId: string;
  id: number;
  result?: unknown;
  error?: string;
}

interface FrameRegistration {
  sessionId: string;
  packageId: string;
  onRevoked?: () => void;
}

type InvokeCommand = <T>(
  command: string,
  args?: Record<string, unknown>,
) => Promise<T>;

const frameRegistrations = new Map<Window, FrameRegistration>();
const handledRequests = new Set<string>();
const listenerOwners = new Set<symbol>();
const isListening = ref(false);
const requestCount = ref(0);
let authorityUnlisten: UnlistenFn | null = null;
let authorityListenGeneration = 0;

interface PackageAuthorityChanged {
  packageId: string;
}

export function invalidatePluginFrames(packageId: string): number {
  let invalidated = 0;
  for (const [sourceWindow, registration] of frameRegistrations) {
    if (registration.packageId !== packageId) continue;
    frameRegistrations.delete(sourceWindow);
    invalidated += 1;
    try {
      sourceWindow.postMessage(
        {
          source: "vortideck-host",
          sessionId: registration.sessionId,
          pluginId: registration.packageId,
          type: "sessionRevoked",
          error: "Plugin frame authority changed; reload is required",
        },
        "*",
      );
    } finally {
      registration.onRevoked?.();
    }
  }
  return invalidated;
}

function isAllowedFrameOrigin(origin: string): boolean {
  return origin === "null" || origin === window.location.origin;
}

function requestPayload(payload: unknown): Record<string, unknown> {
  if (payload === undefined) return {};
  if (!payload || typeof payload !== "object" || Array.isArray(payload)) {
    throw new Error("Plugin API payload must be an object");
  }
  return payload as Record<string, unknown>;
}

async function processRequest(
  sessionId: string,
  type: string,
  rawPayload: unknown,
  invokeCommand: InvokeCommand,
): Promise<unknown> {
  const payload = requestPayload(rawPayload);

  switch (type) {
    case "callPlugin": {
      const functionName = payload.functionName;
      if (typeof functionName !== "string" || !functionName)
        throw new Error("Invalid function name");
      const result = await invokeCommand<string>("plugin_call_wasm", {
        sessionId,
        function: functionName,
        data: JSON.stringify(payload.data ?? {}),
      });
      return JSON.parse(result);
    }
    case "runFlow": {
      const flowName = payload.flowName;
      if (typeof flowName !== "string" || !flowName)
        throw new Error("Invalid flow name");
      const result = await invokeCommand<string>("plugin_run_flow", {
        sessionId,
        flowName,
        input: JSON.stringify(payload.input ?? {}),
      });
      return JSON.parse(result);
    }
    case "storageGet": {
      const key = payload.key;
      if (typeof key !== "string" || !key)
        throw new Error("Invalid storage key");
      const result = await invokeCommand<string | null>("plugin_storage_get", {
        sessionId,
        key,
      });
      return result === null ? null : JSON.parse(result);
    }
    case "storageSet": {
      const key = payload.key;
      if (typeof key !== "string" || !key)
        throw new Error("Invalid storage key");
      await invokeCommand("plugin_storage_set", {
        sessionId,
        key,
        value: JSON.stringify(payload.value),
      });
      return null;
    }
    default:
      throw new Error(`Unknown plugin API: ${type}`);
  }
}

function sendResponse(
  sourceWindow: Window,
  response: PluginResponse,
  origin: string,
) {
  sourceWindow.postMessage(response, origin === "null" ? "*" : origin);
}

/** Exported for the isolated security harness. */
export async function handlePluginRequestEvent(
  event: MessageEvent,
  invokeCommand: InvokeCommand = invoke,
): Promise<boolean> {
  const data = event.data as Partial<PluginRequest> | null;
  if (
    data?.source !== "vortideck-plugin" ||
    typeof data.sessionId !== "string" ||
    !Number.isSafeInteger(data.id) ||
    Number(data.id) <= 0 ||
    typeof data.type !== "string" ||
    !data.type
  ) {
    return false;
  }

  const sourceWindow = event.source as Window | null;
  if (!sourceWindow || !isAllowedFrameOrigin(event.origin)) return false;

  const registration = frameRegistrations.get(sourceWindow);
  if (!registration || registration.sessionId !== data.sessionId) return false;
  if (data.pluginId !== undefined && data.pluginId !== registration.packageId)
    return false;

  const requestKey = `${registration.sessionId}:${data.id}`;
  if (handledRequests.has(requestKey)) return false;
  handledRequests.add(requestKey);
  if (handledRequests.size > 1000) {
    Array.from(handledRequests)
      .slice(0, 500)
      .forEach((key) => handledRequests.delete(key));
  }

  requestCount.value += 1;
  let response: PluginResponse;
  try {
    const result = await processRequest(
      registration.sessionId,
      data.type,
      data.payload,
      invokeCommand,
    );
    response = {
      source: "vortideck-host",
      sessionId: registration.sessionId,
      pluginId: registration.packageId,
      id: Number(data.id),
      result,
    };
  } catch (error) {
    response = {
      source: "vortideck-host",
      sessionId: registration.sessionId,
      pluginId: registration.packageId,
      id: Number(data.id),
      error: error instanceof Error ? error.message : String(error),
    };
  }
  sendResponse(sourceWindow, response, event.origin);
  return true;
}

export async function registerPluginFrame(
  packageId: string,
  sourceWindow: Window,
  invokeCommand: InvokeCommand = invoke,
  onRevoked?: () => void,
): Promise<{
  session: IssuedPluginFrameSession;
  unregister: () => Promise<void>;
}> {
  if (frameRegistrations.has(sourceWindow)) {
    throw new Error("Plugin frame is already registered");
  }
  const session = await invokeCommand<IssuedPluginFrameSession>(
    "register_plugin_frame_session",
    {
      packageId,
    },
  );
  frameRegistrations.set(sourceWindow, {
    sessionId: session.sessionId,
    packageId: session.packageId,
    onRevoked,
  });

  let active = true;
  return {
    session,
    unregister: async () => {
      if (!active) return;
      active = false;
      const current = frameRegistrations.get(sourceWindow);
      if (current?.sessionId === session.sessionId)
        frameRegistrations.delete(sourceWindow);
      try {
        await invokeCommand("revoke_plugin_frame_session", {
          sessionId: session.sessionId,
        });
      } catch (error) {
        console.warn(
          "[Plugin Handler] Native frame-session revocation failed:",
          error,
        );
      }
    },
  };
}

function onMessage(event: MessageEvent) {
  void handlePluginRequestEvent(event).catch((error) => {
    console.error("[Plugin Handler] Unexpected request failure:", error);
  });
}

function syncAuthorityListener(shouldListen: boolean) {
  const generation = ++authorityListenGeneration;
  if (!shouldListen) {
    authorityUnlisten?.();
    authorityUnlisten = null;
    return;
  }
  if (authorityUnlisten) return;
  void listen<PackageAuthorityChanged>("package-authority-changed", (event) => {
    if (typeof event.payload?.packageId === "string") {
      invalidatePluginFrames(event.payload.packageId);
    }
  })
    .then((unlisten) => {
      if (
        generation !== authorityListenGeneration ||
        listenerOwners.size === 0
      ) {
        unlisten();
      } else {
        authorityUnlisten = unlisten;
      }
    })
    .catch((error) => {
      console.error(
        "[Plugin Handler] Failed to subscribe to authority changes:",
        error,
      );
    });
}

function syncListener() {
  const shouldListen = listenerOwners.size > 0;
  if (shouldListen === isListening.value) return;
  if (shouldListen) window.addEventListener("message", onMessage);
  else window.removeEventListener("message", onMessage);
  syncAuthorityListener(shouldListen);
  isListening.value = shouldListen;
}

export function usePluginMessageHandler() {
  const owner = Symbol("plugin-message-handler-owner");
  const startListening = () => {
    listenerOwners.add(owner);
    syncListener();
  };
  const stopListening = () => {
    listenerOwners.delete(owner);
    syncListener();
  };

  onMounted(startListening);
  onUnmounted(stopListening);

  return { isListening, requestCount, startListening, stopListening };
}
