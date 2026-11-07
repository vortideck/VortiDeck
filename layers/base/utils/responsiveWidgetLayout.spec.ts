import { describe, expect, it } from "vitest";
import type { DeckWidgetInstance, WidgetDefinition } from "@base/types/deckWidget";
import { deriveResponsiveWidgetLayout } from "./responsiveWidgetLayout";

const definition: WidgetDefinition = {
  id: "test",
  name: "Test",
  category: "test",
  rendererId: "test",
  interactionType: "atomic_button",
  defaultSpan: { cols: 1, rows: 1 },
  minSpan: { cols: 1, rows: 1 },
  maxSpan: { cols: 4, rows: 4 },
  configSchema: {},
  inputs: [],
  mobileSupport: "host_schema",
};

const instance = (id: string, x: number, y: number, cols = 1, rows = 1): DeckWidgetInstance => ({
  id,
  definitionId: definition.id,
  configVersion: 1,
  config: {},
  layouts: { landscape: { x, y, cols, rows } },
  bindings: [],
  enabled: true,
});

describe("responsive widget layout", () => {
  it("never returns a partial layout when a widget has no source placement", () => {
    const orphan = instance("orphan", 0, 0);
    orphan.layouts = {};

    expect(deriveResponsiveWidgetLayout(
      [orphan],
      "landscape",
      "portrait",
      { cols: 5, rows: 3 },
      { cols: 3, rows: 5 },
      () => definition,
    )).toBeUndefined();
  });

  it("projects the latest arrangement into the other orientation without overlap", () => {
    const result = deriveResponsiveWidgetLayout(
      [instance("wide", 0, 0, 2, 1), instance("button", 4, 2)],
      "landscape",
      "portrait",
      { cols: 5, rows: 3 },
      { cols: 3, rows: 5 },
      () => definition,
    );
    expect(result?.wide).toMatchObject({ cols: 2, rows: 1 });
    expect(result?.button).toMatchObject({ x: 2, y: 4, cols: 1, rows: 1 });
  });

  it("includes widgets that previously existed in only one orientation", () => {
    const result = deriveResponsiveWidgetLayout(
      [instance("new", 2, 1)],
      "landscape",
      "portrait",
      { cols: 5, rows: 3 },
      { cols: 3, rows: 5 },
      () => definition,
    );
    expect(result?.new).toBeDefined();
  });

  it("backtracks when small widgets would otherwise fragment a completely full grid", () => {
    const widgets = [
      instance("wide-3", 0, 0, 3, 1),
      instance("one-1", 3, 0),
      instance("one-2", 4, 0),
      instance("one-3", 0, 1),
      instance("one-4", 1, 1),
      instance("one-5", 2, 1),
      instance("one-6", 3, 1),
      instance("one-7", 4, 1),
      instance("wide-2", 0, 2, 2, 1),
      instance("one-8", 2, 2),
      instance("one-9", 3, 2),
      instance("one-10", 4, 2),
    ];
    const result = deriveResponsiveWidgetLayout(
      widgets,
      "landscape",
      "portrait",
      { cols: 5, rows: 3 },
      { cols: 3, rows: 5 },
      () => definition,
    );
    expect(Object.keys(result ?? {})).toHaveLength(widgets.length);
    const occupied = new Set<string>();
    for (const placement of Object.values(result ?? {})) {
      for (let y = placement.y; y < placement.y + placement.rows; y += 1) {
        for (let x = placement.x; x < placement.x + placement.cols; x += 1) {
          const cell = `${x},${y}`;
          expect(occupied.has(cell)).toBe(false);
          occupied.add(cell);
        }
      }
    }
    expect(occupied.size).toBe(15);
  });

  it("rotates directional spans only when retaining them cannot fit", () => {
    const widgets = Array.from({ length: 5 }, (_, x) => instance(`vertical-${x}`, x, 0, 1, 3));
    const result = deriveResponsiveWidgetLayout(
      widgets,
      "landscape",
      "portrait",
      { cols: 5, rows: 3 },
      { cols: 3, rows: 5 },
      () => definition,
    );
    expect(Object.keys(result ?? {})).toHaveLength(widgets.length);
    expect(Object.values(result ?? {}).some(({ cols, rows }) => cols === 3 && rows === 1)).toBe(true);
  });

  it("preserves an existing target orientation while placing only new widgets", () => {
    const existing = instance("existing", 0, 0, 2, 1);
    existing.layouts.portrait = { x: 1, y: 3, cols: 1, rows: 2 };
    const added = instance("added", 3, 1);

    const result = deriveResponsiveWidgetLayout(
      [existing, added],
      "landscape",
      "portrait",
      { cols: 5, rows: 3 },
      { cols: 3, rows: 5 },
      () => definition,
      { preserveTargetPlacements: true },
    );

    expect(result?.existing).toEqual({ x: 1, y: 3, cols: 1, rows: 2 });
    expect(result?.added).toBeDefined();
    expect(result?.added).not.toEqual(result?.existing);
  });

  it("refuses to rewrite an invalid saved target orientation", () => {
    const first = instance("first", 0, 0);
    const second = instance("second", 1, 0);
    first.layouts.portrait = { x: 0, y: 0, cols: 1, rows: 1 };
    second.layouts.portrait = { x: 0, y: 0, cols: 1, rows: 1 };

    expect(deriveResponsiveWidgetLayout(
      [first, second],
      "landscape",
      "portrait",
      { cols: 5, rows: 3 },
      { cols: 3, rows: 5 },
      () => definition,
      { preserveTargetPlacements: true },
    )).toBeUndefined();
  });
});
