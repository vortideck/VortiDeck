import { describe, expect, it } from "vitest";
import type { WidgetDefinition } from "@base/types/deckWidget";
import {
  resolvePortableWidgetValue,
  validatePortableWidgetView,
} from "@base/utils/portableWidgetView";

const definition: Pick<WidgetDefinition, "inputs"> = {
  inputs: [{ id: "primary", kind: "trigger", supportedPolicies: ["run_once"] }],
};

describe("portable widget view", () => {
  it("accepts a bounded declarative control tree", () => {
    const result = validatePortableWidgetView({
      schemaVersion: 1,
      root: {
        type: "press-zone",
        padding: 12,
        gestures: [{ gesture: "press", portId: "primary", delivery: "commit" }],
        children: [
          { type: "icon", icon: "play" },
          { type: "text", value: { state: "track.title", fallback: "Nothing playing" } },
        ],
      },
    }, definition);
    expect(result).toEqual({ valid: true, errors: [] });
  });

  it("rejects executable fields, unknown ports, and unsafe depth", () => {
    let root: Record<string, unknown> = { type: "text", value: { literal: "leaf" } };
    for (let index = 0; index < 9; index += 1) root = { type: "panel", children: [root] };
    const result = validatePortableWidgetView({
      schemaVersion: 1,
      root: {
        type: "press-zone",
        html: "<script>alert(1)</script>",
        gestures: [{ gesture: "press", portId: "missing" }],
        children: [root],
      },
    }, definition);
    expect(result.valid).toBe(false);
    expect(result.errors.some((error) => error.includes("unsupported fields"))).toBe(true);
    expect(result.errors.some((error) => error.includes("declared input"))).toBe(true);
    expect(result.errors.some((error) => error.includes("maximum depth"))).toBe(true);
  });

  it("rejects oversized or malformed asset declarations", () => {
    const result = validatePortableWidgetView({
      schemaVersion: 1,
      root: { type: "image", source: { asset: "cover" } },
      assets: [{
        id: "cover",
        sha256: "bad",
        mediaType: "text/html",
        byteLength: 10_000_000,
        purpose: "artwork",
      }],
    }, definition);
    expect(result.valid).toBe(false);
    expect(result.errors).toEqual(expect.arrayContaining([
      "assets[0].sha256 is invalid",
      "assets[0].mediaType is unsupported",
      "assets[0].byteLength is out of range",
    ]));
  });

  it("rejects image nodes that reference an undeclared asset", () => {
    const result = validatePortableWidgetView({
      schemaVersion: 1,
      root: { type: "image", source: { asset: "missing.png" } },
      assets: [],
    }, definition);
    expect(result.errors).toContain("view references undeclared asset 'missing.png'");
  });

  it("resolves only literal, config, and state bindings", () => {
    expect(resolvePortableWidgetValue({ config: "label" }, { label: "Mixer" }, {})).toBe("Mixer");
    expect(resolvePortableWidgetValue(
      { state: "track.title", fallback: "Idle" }, {}, { track: { title: "Song" } },
    )).toBe("Song");
    expect(resolvePortableWidgetValue({ state: "missing", fallback: "Idle" }, {}, {})).toBe("Idle");
  });
});
