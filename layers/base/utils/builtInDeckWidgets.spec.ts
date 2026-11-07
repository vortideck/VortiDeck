import { describe, expect, it } from "vitest";
import { createDeckWidgetEditor } from "./deckWidgetEditor";
import { builtInDeckWidgetDefinitions } from "./builtInDeckWidgets";

describe("built-in deck widget sizing", () => {
  it("adds every built-in as one standard square button slot", () => {
    for (const definition of builtInDeckWidgetDefinitions) {
      expect(definition.defaultSpan, definition.id).toEqual({ cols: 1, rows: 1 });
      expect(definition.minSpan.cols, definition.id).toBe(definition.minSpan.rows);
      expect(definition.maxSpan.cols, definition.id).toBe(definition.maxSpan.rows);
    }
  });

  it("allows every built-in to be resized into rectangular spans", () => {
    for (const definition of builtInDeckWidgetDefinitions) {
      const editor = createDeckWidgetEditor([], 0);
      expect(editor.add(definition, "landscape", { cols: 4, rows: 4 }, {}, definition.id))
        .toEqual({ ok: true });
      expect(editor.place(
        definition.id,
        definition,
        "landscape",
        { x: 0, y: 0, cols: 1, rows: 2 },
        { cols: 4, rows: 4 },
      ), definition.id).toEqual({ ok: true });
    }
  });
});
