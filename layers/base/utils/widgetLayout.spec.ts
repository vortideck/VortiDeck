import { describe, expect, it } from "vitest";
import type { DeckWidgetInstance, WidgetPlacement } from "@base/types/deckWidget";
import {
  canPlaceWidget,
  createWidgetHistory,
  firstRestrictedCellPosition,
  findFirstAvailablePlacement,
  moveWidgetsBy,
  moveWidgetsWithReflow,
  placeWidget,
  placeWidgetWithReflow,
  placementsOverlap,
  placementExceedsCellLimit,
} from "@base/utils/widgetLayout";

const widget = (id: string, placement: WidgetPlacement): DeckWidgetInstance => ({
  id,
  definitionId: "test.widget",
  configVersion: 1,
  config: {},
  layouts: { default: placement },
  bindings: [],
  enabled: true,
});

describe("widget layout engine", () => {
  it("moves a selected group rigidly and rejects collisions", () => {
    const instances = [
      widget("one", { x: 0, y: 0, cols: 1, rows: 1 }),
      widget("two", { x: 1, y: 0, cols: 1, rows: 1 }),
      widget("blocker", { x: 3, y: 0, cols: 1, rows: 1 }),
    ];
    const moved = moveWidgetsBy(instances, ["one", "two"], "default", { x: 0, y: 1 }, { cols: 4, rows: 3 });
    expect(moved.ok).toBe(true);
    if (moved.ok) {
      expect(moved.instances.find(({ id }) => id === "one")?.layouts.default).toMatchObject({ x: 0, y: 1 });
      expect(moved.instances.find(({ id }) => id === "two")?.layouts.default).toMatchObject({ x: 1, y: 1 });
    }
    expect(moveWidgetsBy(instances, ["one", "two"], "default", { x: 2, y: 0 }, { cols: 4, rows: 3 }).ok)
      .toBe(false);
  });

  it("moves a selected group and repacks widgets covered by the group", () => {
    const instances = [
      widget("one", { x: 0, y: 0, cols: 1, rows: 1 }),
      widget("two", { x: 1, y: 0, cols: 1, rows: 1 }),
      widget("blocker-one", { x: 0, y: 1, cols: 1, rows: 1 }),
      widget("blocker-two", { x: 1, y: 1, cols: 1, rows: 1 }),
    ];
    const moved = moveWidgetsWithReflow(
      instances,
      ["one", "two"],
      "default",
      { x: 0, y: 1 },
      { cols: 2, rows: 2 },
    );
    expect(moved.ok).toBe(true);
    if (!moved.ok) return;
    expect(moved.instances.map(({ layouts }) => layouts.default)).toEqual([
      { x: 0, y: 1, cols: 1, rows: 1 },
      { x: 1, y: 1, cols: 1, rows: 1 },
      { x: 0, y: 0, cols: 1, rows: 1 },
      { x: 1, y: 0, cols: 1, rows: 1 },
    ]);
  });

  it("detects overlap for multi-span rectangles", () => {
    expect(placementsOverlap(
      { x: 0, y: 0, cols: 2, rows: 2 },
      { x: 1, y: 1, cols: 2, rows: 1 },
    )).toBe(true);
    expect(placementsOverlap(
      { x: 0, y: 0, cols: 2, rows: 2 },
      { x: 2, y: 0, cols: 1, rows: 2 },
    )).toBe(false);
  });

  it("rejects collisions and preserves the original document", () => {
    const instances = [
      widget("one", { x: 0, y: 0, cols: 2, rows: 1 }),
      widget("two", { x: 2, y: 0, cols: 1, rows: 1 }),
    ];
    const result = placeWidget(
      instances,
      "two",
      "default",
      { x: 1, y: 0, cols: 1, rows: 1 },
      { cols: 3, rows: 2 },
    );
    expect(result).toEqual({ ok: false, reason: "collision", instances });
    expect(instances[1]?.layouts.default).toEqual({ x: 2, y: 0, cols: 1, rows: 1 });
  });

  it("rejects placements outside the grid", () => {
    const instances = [widget("one", { x: 0, y: 0, cols: 1, rows: 1 })];
    expect(placeWidget(
      instances,
      "one",
      "default",
      { x: 2, y: 0, cols: 2, rows: 1 },
      { cols: 3, rows: 2 },
    )).toMatchObject({ ok: false, reason: "out_of_bounds" });
  });

  it("finds the first row-major opening for a requested span", () => {
    const instances = [
      widget("one", { x: 0, y: 0, cols: 2, rows: 1 }),
      widget("two", { x: 2, y: 0, cols: 1, rows: 2 }),
    ];
    expect(findFirstAvailablePlacement(
      instances,
      "new",
      "default",
      { cols: 2, rows: 1 },
      { cols: 3, rows: 3 },
    )).toEqual({ x: 0, y: 1, cols: 2, rows: 1 });
    expect(canPlaceWidget(
      instances,
      "new",
      "default",
      { x: 0, y: 1, cols: 2, rows: 1 },
      { cols: 3, rows: 3 },
    )).toBe(true);
  });

  it("applies a cell allowance to the complete footprint of resized widgets", () => {
    const bounds = { cols: 5, rows: 3 };
    expect(placementExceedsCellLimit(
      { x: 0, y: 0, cols: 3, rows: 1 },
      bounds,
      3,
    )).toBe(false);
    expect(firstRestrictedCellPosition(
      { x: 2, y: 0, cols: 2, rows: 1 },
      bounds,
      3,
    )).toBe(4);
    expect(placementExceedsCellLimit(
      { x: 0, y: 1, cols: 1, rows: 2 },
      bounds,
      6,
    )).toBe(true);
  });

  it("uses the visible orientation's column count for row-major limits", () => {
    const placement = { x: 2, y: 1, cols: 1, rows: 1 };
    expect(firstRestrictedCellPosition(placement, { cols: 3, rows: 5 }, 5)).toBe(6);
    expect(firstRestrictedCellPosition(placement, { cols: 5, rows: 3 }, 8)).toBeUndefined();
  });

  it("moves covered 1x1 widgets into a large widget's vacated footprint", () => {
    const instances = [
      widget("wide", { x: 0, y: 0, cols: 3, rows: 1 }),
      widget("one", { x: 0, y: 1, cols: 1, rows: 1 }),
      widget("two", { x: 1, y: 1, cols: 1, rows: 1 }),
      widget("three", { x: 2, y: 1, cols: 1, rows: 1 }),
    ];
    const result = placeWidgetWithReflow(
      instances,
      "wide",
      "default",
      { x: 0, y: 1, cols: 3, rows: 1 },
      { cols: 3, rows: 2 },
    );
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.instances.find(({ id }) => id === "wide")?.layouts.default)
      .toEqual({ x: 0, y: 1, cols: 3, rows: 1 });
    expect(result.instances.filter(({ id }) => id !== "wide").map(({ layouts }) => layouts.default))
      .toEqual([
        { x: 0, y: 0, cols: 1, rows: 1 },
        { x: 1, y: 0, cols: 1, rows: 1 },
        { x: 2, y: 0, cols: 1, rows: 1 },
      ]);
  });

  it("rejects a displacement atomically when the covered widgets cannot be repacked", () => {
    const instances = [
      widget("wide", { x: 0, y: 0, cols: 2, rows: 1 }),
      widget("blocked", { x: 0, y: 1, cols: 3, rows: 1 }),
      widget("fixed", { x: 2, y: 0, cols: 1, rows: 1 }),
    ];
    const result = placeWidgetWithReflow(
      instances,
      "wide",
      "default",
      { x: 0, y: 1, cols: 2, rows: 1 },
      { cols: 3, rows: 2 },
    );
    expect(result).toEqual({ ok: false, reason: "collision", instances });
  });

  it("supports bounded undo and redo without mutating snapshots", () => {
    const history = createWidgetHistory(["a"]);
    history.push(["a", "b"]);
    history.push(["a", "b", "c"]);
    expect(history.undo()).toEqual(["a", "b"]);
    expect(history.undo()).toEqual(["a"]);
    expect(history.redo()).toEqual(["a", "b"]);
    history.push(["replacement"]);
    expect(history.canRedo()).toBe(false);
  });
});
