import { describe, expect, it } from "vitest";
import type { WidgetDefinition } from "@base/types/deckWidget";
import {
  portableDefinitionHash,
  verifyPortableAssetObject,
  verifyPortableDefinition,
} from "./portableWidgetCache";

const definition = (): WidgetDefinition => ({
  id: "community.player.control",
  name: "Player",
  category: "plugins",
  rendererId: "portable-schema",
  interactionType: "atomic_button",
  defaultSpan: { cols: 1, rows: 1 },
  minSpan: { cols: 1, rows: 1 },
  maxSpan: { cols: 2, rows: 2 },
  configSchema: {},
  inputs: [{ id: "play", kind: "trigger", supportedPolicies: ["run_once"] }],
  mobileSupport: "host_schema",
  portableView: {
    schemaVersion: 1,
    root: {
      type: "press-zone",
      gestures: [{ gesture: "press", portId: "play", delivery: "commit" }],
      children: [{ type: "text", value: { literal: "Play" } }],
    },
  },
});

describe("portable widget asset integrity", () => {
  const asset = {
    id: "icon.png",
    sha256: "4c4b6a3be1314ab86138bef4314dde022e600960d8689a2c8f8631802d20dab6",
    mediaType: "image/png" as const,
    byteLength: 8,
    purpose: "icon" as const,
  };

  it("accepts bytes matching the declared hash, length, type, and PNG signature", () => {
    expect(verifyPortableAssetObject({ asset, data: "iVBORw0KGgo=" })).toBe(true);
  });

  it("rejects a type-confused or modified asset", () => {
    expect(verifyPortableAssetObject({
      asset: { ...asset, mediaType: "image/jpeg" },
      data: "iVBORw0KGgo=",
    })).toBe(false);
    expect(verifyPortableAssetObject({ asset, data: "YWJj" })).toBe(false);
  });
});

describe("portable widget definition integrity", () => {
  it("accepts the exact canonical definition hash", () => {
    const value = definition();
    value.contentHash = portableDefinitionHash(value);
    expect(verifyPortableDefinition(value)).toBe(true);
  });

  it("rejects a cached definition modified after hashing", () => {
    const value = definition();
    value.contentHash = portableDefinitionHash(value);
    value.name = "Tampered";
    expect(verifyPortableDefinition(value)).toBe(false);
  });
});
