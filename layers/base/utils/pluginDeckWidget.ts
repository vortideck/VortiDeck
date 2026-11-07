import type {
  WidgetDefinition,
  WidgetGridSpan,
  WidgetInputPort,
  WidgetInteractionType,
  PortableWidgetView,
} from "@base/types/deckWidget";
import { validatePortableWidgetView } from "@base/utils/portableWidgetView";

export interface PackageDeckSurfaceContribution {
  definitionId: string;
  rendererId: string;
  interactionType: WidgetInteractionType;
  defaultSpan: WidgetGridSpan;
  minSpan: WidgetGridSpan;
  maxSpan: WidgetGridSpan;
  configSchema?: Record<string, unknown>;
  stateSchema?: Record<string, unknown>;
  inputs?: WidgetInputPort[];
  stateSubscriptions?: string[];
  mobileSupport?: WidgetDefinition["mobileSupport"];
  portableView?: PortableWidgetView;
  publicConfigPaths?: string[];
  publicStatePaths?: string[];
}

export interface PackageWidgetContribution {
  type: "widget";
  id?: string;
  name?: string;
  description?: string;
  content?: string;
  deckSurface?: PackageDeckSurfaceContribution;
}

export interface WidgetPackageDescriptor {
  id: string;
  name: string;
  trust: "builtin" | "verified" | "community" | string;
  is_granted: boolean;
  needs_reconsent: boolean;
  contributions: Array<PackageWidgetContribution | { type: string }>;
}

export interface PackageWidgetRegistration {
  definition: WidgetDefinition;
  packageId: string;
  contributionId: string;
  sandboxHtml?: string;
}

const validSpan = (span: WidgetGridSpan) => (
  Number.isInteger(span.cols) && span.cols > 0
  && Number.isInteger(span.rows) && span.rows > 0
);

export const packageDeckWidgetDefinitions = (
  descriptor: WidgetPackageDescriptor,
): PackageWidgetRegistration[] => {
  if (!descriptor.is_granted || descriptor.needs_reconsent) return [];

  return descriptor.contributions.flatMap((contribution) => {
    if (contribution.type !== "widget") return [];
    const widget = contribution as PackageWidgetContribution;
    const surface = widget.deckSurface;
    if (!surface || !widget.id) return [];
    if (!validSpan(surface.defaultSpan) || !validSpan(surface.minSpan) || !validSpan(surface.maxSpan)) {
      return [];
    }
    if (surface.minSpan.cols > surface.defaultSpan.cols
      || surface.minSpan.rows > surface.defaultSpan.rows
      || surface.defaultSpan.cols > surface.maxSpan.cols
      || surface.defaultSpan.rows > surface.maxSpan.rows) return [];
    if (descriptor.trust !== "builtin"
      && surface.rendererId !== "sandboxed-content"
      && surface.rendererId !== "portable-schema") return [];
    if (descriptor.trust !== "builtin"
      && (!surface.definitionId.startsWith(`${descriptor.id}.`)
        || surface.definitionId.startsWith("vortideck."))) return [];

    const definition: WidgetDefinition = {
      id: surface.definitionId,
      name: widget.name ?? widget.id,
      category: "plugins",
      rendererId: surface.rendererId,
      interactionType: surface.interactionType,
      defaultSpan: surface.defaultSpan,
      minSpan: surface.minSpan,
      maxSpan: surface.maxSpan,
      configSchema: surface.configSchema ?? {},
      stateSchema: surface.stateSchema,
      inputs: surface.inputs ?? [],
      stateSubscriptions: surface.stateSubscriptions ?? [],
      mobileSupport: surface.portableView ? "host_schema" : surface.mobileSupport ?? "fallback_only",
      portableView: surface.portableView,
    };
    if (surface.portableView && !validatePortableWidgetView(surface.portableView, definition).valid) {
      return [];
    }
    if (surface.rendererId === "portable-schema" && !surface.portableView) return [];

    return [{
      packageId: descriptor.id,
      contributionId: widget.id,
      sandboxHtml: surface.rendererId === "sandboxed-content" ? widget.content : undefined,
      definition,
    }];
  });
};
