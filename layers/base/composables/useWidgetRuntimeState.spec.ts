import { describe, expect, it } from "vitest";
import { useWidgetRuntimeState } from "@base/composables/useWidgetRuntimeState";

describe("widget runtime state", () => {
  it("applies snapshots and ordered patches", () => {
    const runtime = useWidgetRuntimeState();
    expect(runtime.applySnapshot({
      protocolVersion: 1,
      instanceId: "meter:1",
      revision: 3,
      status: "ready",
      values: { level: 10, peak: 20 },
    })).toBe(true);
    expect(runtime.applyPatch({
      protocolVersion: 1,
      instanceId: "meter:1",
      baseRevision: 3,
      revision: 4,
      values: { level: 15 },
      remove: ["peak"],
    })).toBe(true);
    expect(runtime.get("meter:1")).toMatchObject({ revision: 4, values: { level: 15 } });
  });

  it("rejects revision gaps so callers can request a snapshot", () => {
    const runtime = useWidgetRuntimeState();
    runtime.applySnapshot({
      protocolVersion: 1,
      instanceId: "meter:1",
      revision: 3,
      status: "ready",
      values: {},
    });
    expect(runtime.applyPatch({
      protocolVersion: 1,
      instanceId: "meter:1",
      baseRevision: 4,
      revision: 5,
      values: { level: 20 },
    })).toBe(false);
    expect(runtime.get("meter:1").revision).toBe(3);
  });

  it("publishes host state with monotonic local revisions", () => {
    const runtime = useWidgetRuntimeState();
    runtime.publish("fader:1", { value: 40 });
    runtime.publish("fader:1", { muted: false });
    expect(runtime.get("fader:1")).toMatchObject({
      revision: 2,
      status: "ready",
      values: { value: 40, muted: false },
    });
  });

  it("boots from a revision-zero patch and preserves snapshot errors", () => {
    const runtime = useWidgetRuntimeState();
    expect(runtime.applyPatch({
      protocolVersion: 1,
      instanceId: "button:1",
      baseRevision: 0,
      revision: 1,
      status: "executing",
      values: {},
    })).toBe(true);
    expect(runtime.applySnapshot({
      protocolVersion: 1,
      instanceId: "button:1",
      revision: 2,
      status: "error",
      values: {},
      error: { code: "execution_failed", message: "Action failed" },
    })).toBe(true);
    expect(runtime.get("button:1").error).toEqual({
      code: "execution_failed",
      message: "Action failed",
    });
  });

  it("keeps requested state separate from authoritative provider observations", () => {
    const runtime = useWidgetRuntimeState();
    runtime.applySnapshot({
      protocolVersion: 1,
      instanceId: "volume:1",
      revision: 1,
      status: "ready",
      values: { value: 25, muted: false },
      providerStates: {
        "system.volume": {
          revision: 7,
          availability: "available",
          observedAt: 100,
          values: { value: 25, muted: false },
        },
      },
    });
    runtime.applyPatch({
      protocolVersion: 1,
      instanceId: "volume:1",
      baseRevision: 1,
      revision: 2,
      status: "executing",
      request: { requestId: "request-1", requestedAt: 110, values: { value: 80 } },
    });
    expect(runtime.get("volume:1")).toMatchObject({
      status: "executing",
      values: { value: 25, muted: false },
      request: { values: { value: 80 } },
    });
    runtime.applyPatch({
      protocolVersion: 1,
      instanceId: "volume:1",
      baseRevision: 2,
      revision: 3,
      status: "error",
      request: null,
      error: { code: "device_rejected", message: "Device rejected the change" },
    });
    expect(runtime.get("volume:1")).toMatchObject({
      status: "error",
      values: { value: 25, muted: false },
      request: undefined,
    });
  });

  it("preserves the last observation but marks its provider unavailable", () => {
    const runtime = useWidgetRuntimeState();
    runtime.applySnapshot({
      protocolVersion: 1,
      instanceId: "device:1",
      revision: 1,
      status: "ready",
      values: { enabled: true },
      providerStates: {
        device: {
          revision: 1,
          availability: "available",
          observedAt: 100,
          values: { enabled: true },
        },
      },
    });
    runtime.applyPatch({
      protocolVersion: 1,
      instanceId: "device:1",
      baseRevision: 1,
      revision: 2,
      status: "disconnected",
      values: {},
      providerStates: {
        device: {
          revision: 2,
          availability: "unavailable",
          observedAt: 120,
          values: { enabled: true },
        },
      },
      request: null,
    });
    expect(runtime.get("device:1")).toMatchObject({
      status: "disconnected",
      values: { enabled: true },
      providerStates: { device: { availability: "unavailable", revision: 2 } },
    });
  });

  it("converges two controls and a remote client on provider truth", () => {
    const first = useWidgetRuntimeState();
    const second = useWidgetRuntimeState();
    const remote = useWidgetRuntimeState();
    const seed = (runtime: ReturnType<typeof useWidgetRuntimeState>, instanceId: string) =>
      runtime.applySnapshot({
        protocolVersion: 1,
        instanceId,
        revision: 1,
        status: "ready",
        values: { muted: false },
        providerStates: {
          device: {
            revision: 1,
            availability: "available",
            observedAt: 10,
            values: { muted: false },
          },
        },
      });
    seed(first, "button:1");
    seed(second, "button:2");
    seed(remote, "button:1");

    first.applyPatch({
      protocolVersion: 1,
      instanceId: "button:1",
      baseRevision: 1,
      revision: 2,
      status: "executing",
      request: { requestId: "mute", requestedAt: 20, values: { muted: true } },
    });
    expect(first.get("button:1").values.muted).toBe(false);
    expect(second.get("button:2").values.muted).toBe(false);
    expect(remote.get("button:1").values.muted).toBe(false);

    first.applyPatch({
      protocolVersion: 1,
      instanceId: "button:1",
      baseRevision: 2,
      revision: 3,
      status: "error",
      request: null,
      error: { code: "rejected", message: "Rejected" },
    });
    for (const [runtime, instanceId, baseRevision] of [
      [first, "button:1", 3],
      [second, "button:2", 1],
      [remote, "button:1", 1],
    ] as const) {
      runtime.applyPatch({
        protocolVersion: 1,
        instanceId,
        baseRevision,
        revision: baseRevision + 1,
        status: "disconnected",
        values: {},
        providerStates: {
          device: {
            revision: 2,
            availability: "unavailable",
            observedAt: 30,
            values: { muted: false },
          },
        },
        request: null,
        error: null,
      });
      runtime.applyPatch({
        protocolVersion: 1,
        instanceId,
        baseRevision: baseRevision + 1,
        revision: baseRevision + 2,
        status: "ready",
        values: { muted: true },
        providerStates: {
          device: {
            revision: 3,
            availability: "available",
            observedAt: 40,
            values: { muted: true },
          },
        },
        request: null,
        error: null,
      });
      expect(runtime.get(instanceId)).toMatchObject({
        status: "ready",
        values: { muted: true },
        request: undefined,
        providerStates: { device: { revision: 3, availability: "available" } },
      });
    }
  });
});
