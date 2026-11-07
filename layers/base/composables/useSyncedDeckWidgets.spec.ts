import { beforeEach, describe, expect, it, vi } from "vitest";

const remoteDefinitions = new Map<string, unknown>();
vi.mock("@base/composables/useWidgetRegistry", () => ({
  useWidgetRegistry: () => ({
    clearRemote: () => remoteDefinitions.clear(),
    registerRemote: (definition: { id: string }) => remoteDefinitions.set(definition.id, definition),
    unregisterRemote: (definitionId: string) => remoteDefinitions.delete(definitionId),
    resolve: (definitionId: string) => {
      const definition = remoteDefinitions.get(definitionId);
      return definition ? { definition } : undefined;
    },
  }),
}));
import { useSyncedDeckWidgets } from "@base/composables/useSyncedDeckWidgets";
import type {
  DeckWidgetDeltaMessage,
  DeckWidgetSnapshotMessage,
} from "@base/types/deckWidgetProtocol";

const instance = {
  id: "widget:1",
  definitionId: "test.control",
  configVersion: 1,
  config: {},
  layouts: { default: { x: 0, y: 0, cols: 1, rows: 1 } },
  availablePorts: ["toggle"],
  enabled: true,
};

const definition = {
  id: "test.control",
  name: "Test control",
  category: "test",
  rendererId: "unavailable",
  interactionType: "atomic_button" as const,
  defaultSpan: { cols: 1, rows: 1 },
  minSpan: { cols: 1, rows: 1 },
  maxSpan: { cols: 1, rows: 1 },
  configSchema: {},
  inputs: [],
  stateSubscriptions: ["device"],
  mobileSupport: "host_schema" as const,
};

describe("synced deck widget lifecycle", () => {
  const synced = useSyncedDeckWidgets();

  beforeEach(() => synced.reset());

  it("drops stale instance state when a widget is rebound", () => {
    const snapshot: DeckWidgetSnapshotMessage = {
      action: "deck_widget_snapshot",
      protocolVersion: 1,
      deckId: 7,
      revision: 3,
      layoutKey: "default",
      definitions: [definition],
      instances: [instance],
      state: [{
        protocolVersion: 1,
        instanceId: instance.id,
        revision: 4,
        status: "ready",
        values: { enabled: true },
      }],
    };
    expect(synced.applySnapshot(snapshot)).toBe(true);
    expect(synced.runtime.get(instance.id).values.enabled).toBe(true);

    const delta: DeckWidgetDeltaMessage = {
      action: "deck_widget_delta",
      protocolVersion: 1,
      deckId: 7,
      baseRevision: 3,
      revision: 4,
      upsert: [{ ...instance, configVersion: 2 }],
      remove: [],
      state: [],
    };
    expect(synced.applyDelta(delta)).toBe(true);
    expect(synced.runtime.get(instance.id)).toMatchObject({
      revision: 0,
      status: "disconnected",
      values: {},
    });
  });
});
