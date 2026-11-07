import { describe, expect, it } from "vitest";
import type { WidgetDefinition } from "@base/types/deckWidget";
import { createDeckWidgetEditor } from "@base/utils/deckWidgetEditor";

const definition: WidgetDefinition = {
  id: "test.panel",
  name: "Panel",
  category: "test",
  rendererId: "test",
  interactionType: "compound_zones",
  defaultSpan: { cols: 2, rows: 1 },
  minSpan: { cols: 1, rows: 1 },
  maxSpan: { cols: 3, rows: 2 },
  configSchema: {},
  inputs: [{ id: "primary", kind: "trigger", supportedPolicies: ["run_once"] }],
  mobileSupport: "host_schema",
};

describe("deck widget editor", () => {
  it("adds, configures, binds and saves a widget document", () => {
    const editor = createDeckWidgetEditor([], 2);
    expect(editor.add(definition, "portrait", { cols: 4, rows: 4 }, { label: "One" }, "one"))
      .toEqual({ ok: true });
    expect(editor.configure("one", definition, { label: "Updated" })).toBe(true);
    expect(editor.bind("one", {
      id: "binding:one",
      portId: "primary",
      action: "action",
      policy: { type: "run_once" },
    })).toBe(true);
    expect(editor.instances()[0]).toMatchObject({
      id: "one",
      config: { label: "Updated" },
      layouts: { portrait: { x: 0, y: 0, cols: 2, rows: 1 } },
    });
    expect(editor.isDirty()).toBe(true);
    editor.markSaved(3);
    expect(editor.revision()).toBe(3);
    expect(editor.isDirty()).toBe(false);
  });

  it("adds at an explicitly selected empty cell", () => {
    const editor = createDeckWidgetEditor([], 0);
    expect(editor.add(
      definition,
      "portrait",
      { cols: 4, rows: 4 },
      {},
      "one",
      { x: 2, y: 1, cols: 2, rows: 1 },
    )).toEqual({ ok: true });
    expect(editor.instances()[0]?.layouts.portrait)
      .toEqual({ x: 2, y: 1, cols: 2, rows: 1 });
  });

  it("restores an existing configured instance at a selected cell", () => {
    const editor = createDeckWidgetEditor([], 0);
    const restored = {
      id: "button:42",
      definitionId: definition.id,
      configVersion: 1,
      config: { label: "Restored" },
      layouts: { landscape: { x: 3, y: 3, cols: 1, rows: 1 } },
      bindings: [],
      legacyButtonId: 42,
      enabled: true,
    };
    expect(editor.insert(
      restored,
      definition,
      "portrait",
      { x: 1, y: 2, cols: 1, rows: 1 },
      { cols: 4, rows: 4 },
    )).toEqual({ ok: true });
    expect(editor.instances()[0]).toMatchObject({
      id: "button:42",
      legacyButtonId: 42,
      config: { label: "Restored" },
      layouts: { portrait: { x: 1, y: 2, cols: 1, rows: 1 } },
    });
  });

  it("enforces definition spans and supports undo", () => {
    const editor = createDeckWidgetEditor([], 0);
    editor.add(definition, "default", { cols: 4, rows: 4 }, {}, "one");
    expect(editor.place(
      "one",
      definition,
      "default",
      { x: 0, y: 0, cols: 4, rows: 1 },
      { cols: 4, rows: 4 },
    )).toEqual({ ok: false, reason: "invalid_span" });
    editor.remove("one");
    expect(editor.instances()).toEqual([]);
    expect(editor.undo()).toHaveLength(1);
  });

  it("swaps occupied cells as one history operation", () => {
    const editor = createDeckWidgetEditor([], 0);
    editor.add(definition, "landscape", { cols: 4, rows: 2 }, {}, "one");
    editor.add(definition, "landscape", { cols: 4, rows: 2 }, {}, "two");
    const before = editor.instances().map(({ id, layouts }) => ({ id, placement: layouts.landscape }));
    expect(editor.swap("one", "two", "landscape", { cols: 4, rows: 2 })).toEqual({ ok: true });
    expect(editor.instances()[0]?.layouts.landscape).toEqual(before[1]?.placement);
    expect(editor.instances()[1]?.layouts.landscape).toEqual(before[0]?.placement);
    editor.undo();
    expect(editor.instances().map(({ id, layouts }) => ({ id, placement: layouts.landscape })))
      .toEqual(before);
  });

  it("removes a group as one undoable edit", () => {
    const editor = createDeckWidgetEditor([], 0);
    editor.add(definition, "landscape", { cols: 6, rows: 2 }, {}, "one");
    editor.add(definition, "landscape", { cols: 6, rows: 2 }, {}, "two");
    editor.add(definition, "landscape", { cols: 6, rows: 2 }, {}, "three");

    expect(editor.removeMany(["one", "three"])).toEqual({ ok: true });
    expect(editor.instances().map(({ id }) => id)).toEqual(["two"]);
    editor.undo();
    expect(editor.instances().map(({ id }) => id)).toEqual(["one", "two", "three"]);
  });

  it("moves a selected group as one undoable edit", () => {
    const editor = createDeckWidgetEditor([], 0);
    editor.add(definition, "landscape", { cols: 6, rows: 4 }, {}, "one");
    editor.add(definition, "landscape", { cols: 6, rows: 4 }, {}, "two");
    const before = editor.instances().map(({ layouts }) => layouts.landscape!);
    expect(editor.moveMany(["one", "two"], "landscape", { x: 0, y: 1 }, { cols: 6, rows: 4 }))
      .toEqual({ ok: true });
    expect(editor.instances().map(({ layouts }) => layouts.landscape))
      .toEqual(before.map((placement) => ({ ...placement, y: placement.y + 1 })));
    editor.undo();
    expect(editor.instances().map(({ layouts }) => layouts.landscape)).toEqual(before);
  });

  it("removes a widget and all of its orientation placements together", () => {
    const editor = createDeckWidgetEditor([], 0);
    editor.add(definition, "landscape", { cols: 4, rows: 4 }, {}, "one");
    editor.replaceLayout("portrait", {
      one: { x: 0, y: 1, cols: 1, rows: 2 },
    }, { cols: 3, rows: 5 });

    expect(editor.instances()[0]?.layouts).toMatchObject({
      landscape: { x: 0, y: 0, cols: 2, rows: 1 },
      portrait: { x: 0, y: 1, cols: 1, rows: 2 },
    });
    expect(editor.remove("one")).toEqual({ ok: true });
    expect(editor.instances()).toEqual([]);
  });

  it("replaces an orientation layout as one undoable transaction", () => {
    const editor = createDeckWidgetEditor([], 0);
    editor.add(definition, "landscape", { cols: 6, rows: 2 }, {}, "one");
    editor.add(definition, "landscape", { cols: 6, rows: 2 }, {}, "two");
    expect(editor.replaceLayout("portrait", {
      one: { x: 0, y: 0, cols: 2, rows: 1 },
      two: { x: 0, y: 1, cols: 2, rows: 1 },
    }, { cols: 2, rows: 6 })).toEqual({ ok: true });
    expect(editor.instances().every(({ layouts }) => layouts.portrait != null)).toBe(true);
    editor.undo();
    expect(editor.instances().every(({ layouts }) => layouts.portrait == null)).toBe(true);
  });

  it("can fold a derived orientation into the originating edit history entry", () => {
    const editor = createDeckWidgetEditor([], 0);
    editor.add(definition, "landscape", { cols: 4, rows: 4 }, {}, "one");
    expect(editor.replaceLayout("portrait", {
      one: { x: 0, y: 0, cols: 2, rows: 1 },
    }, { cols: 4, rows: 4 }, false)).toEqual({ ok: true });
    editor.undo();
    expect(editor.instances()).toEqual([]);
  });

  it("edits one orientation without changing the other", () => {
    const editor = createDeckWidgetEditor([], 0);
    editor.add(definition, "landscape", { cols: 4, rows: 4 }, {}, "one");
    editor.replaceLayout("portrait", {
      one: { x: 0, y: 1, cols: 1, rows: 2 },
    }, { cols: 3, rows: 5 });
    const portraitBefore = editor.instances()[0]?.layouts.portrait;

    expect(editor.place(
      "one",
      definition,
      "landscape",
      { x: 1, y: 2, cols: 3, rows: 1 },
      { cols: 4, rows: 4 },
    )).toEqual({ ok: true });

    expect(editor.instances()[0]?.layouts.landscape)
      .toEqual({ x: 1, y: 2, cols: 3, rows: 1 });
    expect(editor.instances()[0]?.layouts.portrait).toEqual(portraitBefore);
  });
});
