import { describe, expect, it } from "vitest";
import {
  packageDeckWidgetDefinitions,
  type PackageWidgetContribution,
  type WidgetPackageDescriptor,
} from "@base/utils/pluginDeckWidget";

const packageDescriptor = (overrides: Partial<WidgetPackageDescriptor> = {}): WidgetPackageDescriptor => ({
  id: "community.mixer",
  name: "Mixer",
  trust: "community",
  is_granted: true,
  needs_reconsent: false,
  contributions: [{
    type: "widget",
    id: "fader",
    name: "Fader",
    content: "<button>mix</button>",
    deckSurface: {
      definitionId: "community.mixer.fader",
      rendererId: "sandboxed-content",
      interactionType: "continuous_value",
      defaultSpan: { cols: 1, rows: 2 },
      minSpan: { cols: 1, rows: 1 },
      maxSpan: { cols: 2, rows: 4 },
      inputs: [{
        id: "value",
        kind: "value",
        valueType: "number",
        min: 0,
        max: 100,
        supportedDelivery: ["live_and_commit"],
      }],
    },
  }],
  ...overrides,
});

describe("plugin deck widget definitions", () => {
  it("registers granted package contributions with provenance", () => {
    const registrations = packageDeckWidgetDefinitions(packageDescriptor());
    expect(registrations).toHaveLength(1);
    expect(registrations[0]).toMatchObject({
      packageId: "community.mixer",
      contributionId: "fader",
      sandboxHtml: "<button>mix</button>",
      definition: {
        id: "community.mixer.fader",
        defaultSpan: { cols: 1, rows: 2 },
      },
    });
  });

  it("hides ungranted and re-consent-required packages", () => {
    expect(packageDeckWidgetDefinitions(packageDescriptor({ is_granted: false }))).toEqual([]);
    expect(packageDeckWidgetDefinitions(packageDescriptor({ needs_reconsent: true }))).toEqual([]);
  });

  it("rejects host renderers and namespace spoofing from community packages", () => {
    const hostRenderer = packageDescriptor();
    const contribution = hostRenderer.contributions[0] as PackageWidgetContribution;
    if (contribution.deckSurface) contribution.deckSurface.rendererId = "legacy-action-button";
    expect(packageDeckWidgetDefinitions(hostRenderer)).toEqual([]);

    const spoofed = packageDescriptor();
    const spoofedContribution = spoofed.contributions[0] as PackageWidgetContribution;
    if (spoofedContribution.deckSurface) {
      spoofedContribution.deckSurface.definitionId = "vortideck.action-button";
    }
    expect(packageDeckWidgetDefinitions(spoofed)).toEqual([]);
  });

  it("registers a validated portable companion view but rejects executable fields", () => {
    const portable = packageDescriptor();
    const contribution = portable.contributions[0] as PackageWidgetContribution;
    contribution.deckSurface!.portableView = {
      schemaVersion: 1,
      root: {
        type: "slider",
        value: { state: "value", fallback: 0 },
        gestures: [{ gesture: "drag", portId: "value", delivery: "preview_commit" }],
      },
    };
    const registration = packageDeckWidgetDefinitions(portable)[0];
    expect(registration.definition.mobileSupport).toBe("host_schema");
    expect(registration.definition.portableView?.root.type).toBe("slider");

    (contribution.deckSurface!.portableView!.root as any).html = "<script>alert(1)</script>";
    expect(packageDeckWidgetDefinitions(portable)).toEqual([]);
  });
});
