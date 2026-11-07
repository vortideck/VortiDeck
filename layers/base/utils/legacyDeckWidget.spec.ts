import { describe, expect, it } from "vitest";
import type { Button } from "@base/types/deck";
import {
  legacyButtonToWidget,
  reconcileLegacyWidgetInstance,
  legacyExecutionPolicy,
  legacyPositionToPlacement,
  legacyWidgetPlacementForLayout,
  isVacantLegacyButton,
} from "@base/utils/legacyDeckWidget";

const button = (overrides: Partial<Button> = {}): Button => ({
  id: 42,
  deck_id: 3,
  label: "Test",
  icon: null,
  background: null,
  action: '{"SystemControl":"system.volume.set"}',
  position: 7,
  mode: "click/press",
  execution_mode: "run_once",
  execution_config: null,
  icon_updated_at: null,
  icon_fit: "cover",
  ...overrides,
});

describe("legacy deck widget adapter", () => {
  it("converts one-based legacy positions into grid coordinates", () => {
    expect(legacyPositionToPlacement(7, 3)).toEqual({
      x: 0,
      y: 2,
      cols: 1,
      rows: 1,
    });
  });

  it("preserves legacy button identity and action binding", () => {
    const widget = legacyButtonToWidget(button(), 3);
    expect(widget.id).toBe("button:42");
    expect(widget.legacyButtonId).toBe(42);
    expect(widget.definitionId).toBe("vortideck.action-button");
    expect(widget.bindings[0]?.action).toBe(button().action);
  });

  it("creates explicit portrait and landscape placements when both grids are known", () => {
    const widget = legacyButtonToWidget(button(), { portrait: 3, landscape: 5 });
    expect(widget.layouts).toEqual({
      default: { x: 0, y: 2, cols: 1, rows: 1 },
      portrait: { x: 0, y: 2, cols: 1, rows: 1 },
      landscape: { x: 1, y: 1, cols: 1, rows: 1 },
    });
  });

  it("repairs an old default-only migrated placement for the active orientation", () => {
    const widget = legacyButtonToWidget(button(), 3);
    expect(legacyWidgetPlacementForLayout(
      widget,
      "landscape",
      { portrait: 3, landscape: 5 },
    )).toEqual({ x: 1, y: 1, cols: 1, rows: 1 });
  });

  it("maps rotary buttons to continuous value bindings", () => {
    const widget = legacyButtonToWidget(button({ mode: "rotary" }), 3);
    expect(widget.definitionId).toBe("vortideck.rotary");
    expect(widget.bindings[0]?.portId).toBe("value");
    expect(widget.bindings[0]?.policy).toEqual({
      type: "continuous",
      throttleMs: 50,
      commitOnRelease: true,
    });
  });

  it("maps repeat configuration without losing timing", () => {
    expect(legacyExecutionPolicy(button({
      execution_mode: "repeat_while_pressing",
      execution_config: JSON.stringify({
        hold_threshold_ms: 450,
        repeat_interval_ms: 80,
      }),
    }))).toEqual({
      type: "repeat",
      holdMs: 450,
      intervalMs: 80,
    });
  });

  it("distinguishes vacant V1 slots from configured controls", () => {
    expect(isVacantLegacyButton(button({
      label: "",
      action: null,
      mode: null,
    }))).toBe(true);
    expect(isVacantLegacyButton(button({ label: "Volume", action: null }))).toBe(false);
    expect(isVacantLegacyButton(button({ label: "", action: null, mode: "rotary" }))).toBe(false);
  });

  it("reconciles a mode change without resetting orientation layouts", () => {
    const action = legacyButtonToWidget(button(), { portrait: 3, landscape: 5 });
    action.layouts.portrait = { x: 0, y: 1, cols: 3, rows: 1 };
    action.layouts.landscape = { x: 2, y: 0, cols: 1, rows: 2 };

    const rotary = reconcileLegacyWidgetInstance(action, button({ mode: "rotary" }));

    expect(rotary.definitionId).toBe("vortideck.rotary");
    expect(rotary.layouts).toEqual(action.layouts);
    expect(rotary.bindings[0]).toMatchObject({
      portId: "value",
      policy: { type: "continuous", throttleMs: 50, commitOnRelease: true },
    });

    const convertedBack = reconcileLegacyWidgetInstance(rotary, button({
      mode: "click/press",
      execution_mode: "repeat_while_pressing",
      execution_config: JSON.stringify({ hold_threshold_ms: 400, repeat_interval_ms: 75 }),
    }));
    expect(convertedBack.definitionId).toBe("vortideck.action-button");
    expect(convertedBack.layouts).toEqual(action.layouts);
    expect(convertedBack.bindings[0]).toMatchObject({
      portId: "primary",
      policy: { type: "repeat", holdMs: 400, intervalMs: 75 },
    });
  });
});
