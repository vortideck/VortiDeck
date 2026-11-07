import { describe, expect, it } from "vitest";
import type { DeckWidgetInstance } from "@base/types/deckWidget";
import { applyDeckWidgetDelta } from "@base/types/deckWidgetProtocol";

const widget = (id: string): DeckWidgetInstance => ({
  id,
  definitionId: "test",
  configVersion: 1,
  config: {},
  layouts: { default: { x: 0, y: 0, cols: 1, rows: 1 } },
  bindings: [],
  enabled: true,
});
const remoteWidget = (id: string) => ({
  ...widget(id),
  availablePorts: [],
});

describe("deck widget sync protocol", () => {
  it("applies an ordered upsert/remove delta", () => {
    const result = applyDeckWidgetDelta([widget("old"), widget("remove")], 7, {
      action: "deck_widget_delta",
      protocolVersion: 1,
      deckId: 1,
      baseRevision: 7,
      revision: 8,
      upsert: [remoteWidget("old"), remoteWidget("new")],
      remove: ["remove"],
      state: [],
    });
    expect(result?.revision).toBe(8);
    expect(result?.instances.map(({ id }) => id)).toEqual(["old", "new"]);
  });

  it("rejects a revision gap", () => {
    expect(applyDeckWidgetDelta([], 7, {
      action: "deck_widget_delta",
      protocolVersion: 1,
      deckId: 1,
      baseRevision: 6,
      revision: 8,
      upsert: [],
      remove: [],
      state: [],
    })).toBeUndefined();
  });
});
