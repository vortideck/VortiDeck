import { computed, shallowReactive, type Component } from "vue";
import type { WidgetDefinition } from "@base/types/deckWidget";
import {
  legacyActionButtonDefinition,
  legacyRotaryDefinition,
} from "@base/utils/legacyDeckWidget";
import {
  packageDeckWidgetDefinitions,
  type WidgetPackageDescriptor,
} from "@base/utils/pluginDeckWidget";
import { builtInDeckWidgetDefinitions } from "@base/utils/builtInDeckWidgets";
import BuiltinFader from "@base/components/Deck/Widget/BuiltinFader.vue";
import BuiltinToggle from "@base/components/Deck/Widget/BuiltinToggle.vue";
import BuiltinMediaControls from "@base/components/Deck/Widget/BuiltinMediaControls.vue";
import BuiltinTelemetry from "@base/components/Deck/Widget/BuiltinTelemetry.vue";
import LegacyActionWidget from "@base/components/Deck/Widget/LegacyActionWidget.vue";
import LegacyRotaryWidget from "@base/components/Deck/Widget/LegacyRotaryWidget.vue";
import { validatePortableWidgetView } from "@base/utils/portableWidgetView";

interface RegisteredWidget {
  definition: WidgetDefinition;
  renderer?: Component;
  packageId?: string;
  contributionId?: string;
  sandboxHtml?: string;
  source: "builtin" | "package" | "remote";
}

const definitions = shallowReactive(new Map<string, RegisteredWidget>());
let initialized = false;

const initializeBuiltins = () => {
  if (initialized) return;
  initialized = true;
  definitions.set(legacyActionButtonDefinition.id, {
    definition: legacyActionButtonDefinition,
    renderer: LegacyActionWidget,
    source: "builtin",
  });
  definitions.set(legacyRotaryDefinition.id, {
    definition: legacyRotaryDefinition,
    renderer: LegacyRotaryWidget,
    source: "builtin",
  });
  const renderers: Record<string, Component> = {
    "builtin-fader": BuiltinFader,
    "builtin-toggle": BuiltinToggle,
    "builtin-media-controls": BuiltinMediaControls,
    "builtin-telemetry": BuiltinTelemetry,
  };
  for (const definition of builtInDeckWidgetDefinitions) {
    definitions.set(definition.id, {
      definition,
      renderer: renderers[definition.rendererId],
      source: "builtin",
    });
  }
};

export const useWidgetRegistry = () => {
  initializeBuiltins();

  const register = (definition: WidgetDefinition, renderer?: Component) => {
    if (definition.portableView
      && !validatePortableWidgetView(definition.portableView, definition).valid) return false;
    definitions.set(definition.id, { definition, renderer, source: "builtin" });
    return true;
  };

  const registerRemote = (definition: WidgetDefinition) => {
    const existing = definitions.get(definition.id);
    if (existing?.source !== "remote" && existing) return false;
    if (definition.portableView
      && !validatePortableWidgetView(definition.portableView, definition).valid) return false;
    definitions.set(definition.id, { definition, source: "remote" });
    return true;
  };

  const clearRemote = () => {
    for (const [definitionId, registration] of definitions) {
      if (registration.source === "remote") definitions.delete(definitionId);
    }
  };
  const unregisterRemote = (definitionId: string) => {
    if (definitions.get(definitionId)?.source !== "remote") return false;
    return definitions.delete(definitionId);
  };

  const unregister = (definitionId: string) => {
    if (definitionId.startsWith("vortideck.")) return false;
    return definitions.delete(definitionId);
  };

  const registerPackage = (descriptor: WidgetPackageDescriptor) => {
    const registrations = packageDeckWidgetDefinitions(descriptor);
    let registered = 0;
    for (const registration of registrations) {
      const existing = definitions.get(registration.definition.id);
      if (existing && existing.packageId !== descriptor.id) continue;
      definitions.set(registration.definition.id, { ...registration, source: "package" });
      registered += 1;
    }
    return registered;
  };

  const unregisterPackage = (packageId: string) => {
    let removed = 0;
    for (const [definitionId, registration] of definitions) {
      if (registration.packageId === packageId && definitions.delete(definitionId)) removed += 1;
    }
    return removed;
  };

  const resolve = (definitionId: string) => definitions.get(definitionId);
  const list = computed(() => Array.from(definitions.values()));

  return {
    register,
    registerRemote,
    clearRemote,
    unregisterRemote,
    unregister,
    registerPackage,
    unregisterPackage,
    resolve,
    list,
  };
};
