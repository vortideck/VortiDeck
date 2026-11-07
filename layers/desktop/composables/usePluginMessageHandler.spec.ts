import { defineComponent } from "vue";
import { mount } from "@vue/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => vi.fn()),
}));

import {
  handlePluginRequestEvent,
  invalidatePluginFrames,
  registerPluginFrame,
  usePluginMessageHandler,
} from "./usePluginMessageHandler";

let nextRequestId = 100;
const cleanups: Array<() => Promise<void>> = [];

function pluginEvent(
  source: Window,
  data: Record<string, unknown>,
  origin = "null",
): MessageEvent {
  const event = new MessageEvent("message", { data, origin });
  Object.defineProperty(event, "source", { value: source });
  return event;
}

function fakeFrame() {
  return { postMessage: vi.fn() } as unknown as Window;
}

async function bindFrame(
  frame: Window,
  packageId = "example.plugin",
  sessionId = "session-a",
) {
  const invokeCommand = vi.fn(async (command: string) => {
    if (command === "register_plugin_frame_session") {
      return { sessionId, packageId, expiresAtMs: Date.now() + 60_000 };
    }
    return true;
  });
  const registration = await registerPluginFrame(
    packageId,
    frame,
    invokeCommand,
  );
  cleanups.push(registration.unregister);
  return registration;
}

afterEach(async () => {
  while (cleanups.length) await cleanups.pop()?.();
});

describe("source-bound plugin frame bridge", () => {
  it("rejects an unregistered opaque frame before invoking native code", async () => {
    const invokeCommand = vi.fn();
    const accepted = await handlePluginRequestEvent(
      pluginEvent(fakeFrame(), {
        source: "vortideck-plugin",
        sessionId: "invented",
        pluginId: "victim.plugin",
        id: nextRequestId++,
        type: "callPlugin",
        payload: { functionName: "execute" },
      }),
      invokeCommand,
    );

    expect(accepted).toBe(false);
    expect(invokeCommand).not.toHaveBeenCalled();
  });

  it("rejects a cross-package claim and a wrong session", async () => {
    const frame = fakeFrame();
    await bindFrame(frame);
    const invokeCommand = vi.fn();
    const base = {
      source: "vortideck-plugin",
      id: nextRequestId++,
      type: "callPlugin",
      payload: { functionName: "execute" },
    };

    expect(
      await handlePluginRequestEvent(
        pluginEvent(frame, {
          ...base,
          sessionId: "session-a",
          pluginId: "victim.plugin",
        }),
        invokeCommand,
      ),
    ).toBe(false);
    expect(
      await handlePluginRequestEvent(
        pluginEvent(frame, {
          ...base,
          id: nextRequestId++,
          sessionId: "wrong-session",
          pluginId: "example.plugin",
        }),
        invokeCommand,
      ),
    ).toBe(false);
    expect(invokeCommand).not.toHaveBeenCalled();
  });

  it("uses the registered session and returns native capability denial", async () => {
    const frame = fakeFrame();
    await bindFrame(frame);
    const invokeCommand = vi.fn(
      async (command: string, args?: Record<string, unknown>) => {
        expect(command).toBe("plugin_run_flow");
        expect(args).toMatchObject({
          sessionId: "session-a",
          flowName: "protected-flow",
        });
        expect(args).not.toHaveProperty("pluginId");
        throw new Error("Permission denied: flows");
      },
    );

    expect(
      await handlePluginRequestEvent(
        pluginEvent(frame, {
          source: "vortideck-plugin",
          sessionId: "session-a",
          pluginId: "example.plugin",
          id: nextRequestId++,
          type: "runFlow",
          payload: { flowName: "protected-flow", input: {} },
        }),
        invokeCommand,
      ),
    ).toBe(true);
    expect(frame.postMessage).toHaveBeenCalledWith(
      expect.objectContaining({
        source: "vortideck-host",
        sessionId: "session-a",
        pluginId: "example.plugin",
        error: "Permission denied: flows",
      }),
      "*",
    );
  });

  it("allows a legitimate frame to call only its registered package session", async () => {
    const frame = fakeFrame();
    await bindFrame(frame);
    const invokeCommand = vi.fn(async () => JSON.stringify({ output: "ok" }));

    expect(
      await handlePluginRequestEvent(
        pluginEvent(frame, {
          source: "vortideck-plugin",
          sessionId: "session-a",
          pluginId: "example.plugin",
          id: nextRequestId++,
          type: "callPlugin",
          payload: { functionName: "execute", data: { value: 1 } },
        }),
        invokeCommand,
      ),
    ).toBe(true);
    expect(invokeCommand).toHaveBeenCalledWith(
      "plugin_call_wasm",
      expect.objectContaining({
        sessionId: "session-a",
        function: "execute",
      }),
    );
    expect(frame.postMessage).toHaveBeenCalledWith(
      expect.objectContaining({
        result: { output: "ok" },
      }),
      "*",
    );
  });

  it("rejects the old frame immediately after teardown", async () => {
    const frame = fakeFrame();
    const registration = await bindFrame(frame);
    await registration.unregister();
    const invokeCommand = vi.fn();

    expect(
      await handlePluginRequestEvent(
        pluginEvent(frame, {
          source: "vortideck-plugin",
          sessionId: "session-a",
          pluginId: "example.plugin",
          id: nextRequestId++,
          type: "callPlugin",
          payload: { functionName: "execute" },
        }),
        invokeCommand,
      ),
    ).toBe(false);
    expect(invokeCommand).not.toHaveBeenCalled();
  });

  it("invalidates only frames owned by the changed package", async () => {
    const revoked = fakeFrame();
    const remaining = fakeFrame();
    const onRevoked = vi.fn();
    const revokedRegistration = await registerPluginFrame(
      "revoked.plugin",
      revoked,
      vi.fn(async () => ({
        sessionId: "revoked-session",
        packageId: "revoked.plugin",
        expiresAtMs: Date.now() + 60_000,
      })),
      onRevoked,
    );
    cleanups.push(revokedRegistration.unregister);
    await bindFrame(remaining, "remaining.plugin", "remaining-session");

    expect(invalidatePluginFrames("revoked.plugin")).toBe(1);
    expect(onRevoked).toHaveBeenCalledOnce();
    expect(revoked.postMessage).toHaveBeenCalledWith(
      expect.objectContaining({
        type: "sessionRevoked",
        sessionId: "revoked-session",
      }),
      "*",
    );
    expect(invalidatePluginFrames("remaining.plugin")).toBe(1);
  });

  it("installs one browser listener for multiple composable consumers", () => {
    const add = vi.spyOn(window, "addEventListener");
    const remove = vi.spyOn(window, "removeEventListener");
    const Consumer = defineComponent({
      setup() {
        usePluginMessageHandler();
        return () => null;
      },
    });

    const first = mount(Consumer);
    const second = mount(Consumer);
    expect(add.mock.calls.filter(([type]) => type === "message")).toHaveLength(
      1,
    );
    first.unmount();
    expect(
      remove.mock.calls.filter(([type]) => type === "message"),
    ).toHaveLength(0);
    second.unmount();
    expect(
      remove.mock.calls.filter(([type]) => type === "message"),
    ).toHaveLength(1);
  });
});
