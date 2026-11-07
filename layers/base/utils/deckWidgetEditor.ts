import type {
  DeckWidgetInstance,
  WidgetBinding,
  WidgetDefinition,
  WidgetPlacement,
} from "@base/types/deckWidget";
import {
  canPlaceWidget,
  createWidgetHistory,
  findFirstAvailablePlacement,
  moveWidgetsWithReflow,
  placeWidget,
  placeWidgetWithReflow,
  type WidgetGridBounds,
  type WidgetPlacementFailure,
} from "@base/utils/widgetLayout";
import { validateWidgetConfig } from "@base/utils/widgetConfigSchema";

export type DeckWidgetEditorResult =
  | { ok: true }
  | { ok: false; reason: WidgetPlacementFailure | "invalid_span" | "invalid_config" | "duplicate_item" };

const createId = () => globalThis.crypto?.randomUUID?.()
  ?? `${Date.now()}-${Math.random().toString(16).slice(2)}`;

export const createDeckWidgetEditor = (
  initialInstances: DeckWidgetInstance[],
  initialRevision: number,
  historyLimit = 50,
) => {
  const history = createWidgetHistory(initialInstances, historyLimit);
  let revision = initialRevision;
  let dirty = false;

  const commit = (instances: DeckWidgetInstance[]) => {
    history.push(instances);
    dirty = true;
  };

  const add = (
    definition: WidgetDefinition,
    layoutKey: string,
    bounds: WidgetGridBounds,
    config: Record<string, unknown> = {},
    id = `${definition.id}:${createId()}`,
    preferredPlacement?: WidgetPlacement,
  ): DeckWidgetEditorResult => {
    const current = history.current();
    if (current.some((instance) => instance.id === id)) return { ok: false, reason: "duplicate_item" };
    const placement = preferredPlacement
      ? { ...preferredPlacement }
      : findFirstAvailablePlacement(
        current,
        id,
        layoutKey,
        definition.defaultSpan,
        bounds,
      );
    if (!placement) return { ok: false, reason: "collision" };
    if (placement.cols < definition.minSpan.cols || placement.rows < definition.minSpan.rows
      || placement.cols > definition.maxSpan.cols || placement.rows > definition.maxSpan.rows) {
      return { ok: false, reason: "invalid_span" };
    }
    if (!canPlaceWidget(current, id, layoutKey, placement, bounds)) {
      return { ok: false, reason: "collision" };
    }
    const validatedConfig = validateWidgetConfig(definition.configSchema, config);
    if (!validatedConfig.valid) return { ok: false, reason: "invalid_config" };
    commit([...current, {
      id,
      definitionId: definition.id,
      configVersion: 1,
      config: validatedConfig.value,
      layouts: { [layoutKey]: placement },
      bindings: [],
      enabled: true,
    }]);
    return { ok: true };
  };

  const insert = (
    instance: DeckWidgetInstance,
    definition: WidgetDefinition,
    layoutKey: string,
    placement: WidgetPlacement,
    bounds: WidgetGridBounds,
  ): DeckWidgetEditorResult => {
    const current = history.current();
    if (current.some(({ id }) => id === instance.id)) {
      return { ok: false, reason: "duplicate_item" };
    }
    if (placement.cols < definition.minSpan.cols || placement.rows < definition.minSpan.rows
      || placement.cols > definition.maxSpan.cols || placement.rows > definition.maxSpan.rows) {
      return { ok: false, reason: "invalid_span" };
    }
    if (!canPlaceWidget(current, instance.id, layoutKey, placement, bounds)) {
      return { ok: false, reason: "collision" };
    }
    const validatedConfig = validateWidgetConfig(definition.configSchema, instance.config);
    if (!validatedConfig.valid) return { ok: false, reason: "invalid_config" };
    commit([...current, {
      ...instance,
      config: validatedConfig.value,
      layouts: { [layoutKey]: { ...placement } },
    }]);
    return { ok: true };
  };

  const remove = (instanceId: string): DeckWidgetEditorResult => {
    const current = history.current();
    if (!current.some((instance) => instance.id === instanceId)) {
      return { ok: false, reason: "missing_item" };
    }
    commit(current.filter((instance) => instance.id !== instanceId));
    return { ok: true };
  };

  const removeMany = (instanceIds: Iterable<string>): DeckWidgetEditorResult => {
    const ids = new Set(instanceIds);
    const current = history.current();
    if (!current.some((instance) => ids.has(instance.id))) {
      return { ok: false, reason: "missing_item" };
    }
    commit(current.filter((instance) => !ids.has(instance.id)));
    return { ok: true };
  };

  const place = (
    instanceId: string,
    definition: WidgetDefinition,
    layoutKey: string,
    placement: WidgetPlacement,
    bounds: WidgetGridBounds,
  ): DeckWidgetEditorResult => {
    if (placement.cols < definition.minSpan.cols || placement.rows < definition.minSpan.rows
      || placement.cols > definition.maxSpan.cols || placement.rows > definition.maxSpan.rows) {
      return { ok: false, reason: "invalid_span" };
    }
    const result = placeWidget(history.current(), instanceId, layoutKey, placement, bounds);
    if (!result.ok) return { ok: false, reason: result.reason };
    commit(result.instances);
    return { ok: true };
  };

  const placeWithReflow = (
    instanceId: string,
    definition: WidgetDefinition,
    layoutKey: string,
    placement: WidgetPlacement,
    bounds: WidgetGridBounds,
  ): DeckWidgetEditorResult => {
    if (placement.cols < definition.minSpan.cols || placement.rows < definition.minSpan.rows
      || placement.cols > definition.maxSpan.cols || placement.rows > definition.maxSpan.rows) {
      return { ok: false, reason: "invalid_span" };
    }
    const result = placeWidgetWithReflow(history.current(), instanceId, layoutKey, placement, bounds);
    if (!result.ok) return { ok: false, reason: result.reason };
    commit(result.instances);
    return { ok: true };
  };

  const moveMany = (
    instanceIds: Iterable<string>,
    layoutKey: string,
    delta: { x: number; y: number },
    bounds: WidgetGridBounds,
  ): DeckWidgetEditorResult => {
    const result = moveWidgetsWithReflow(history.current(), instanceIds, layoutKey, delta, bounds);
    if (!result.ok) return { ok: false, reason: result.reason };
    commit(result.instances);
    return { ok: true };
  };

  const replaceLayout = (
    layoutKey: string,
    placements: Record<string, WidgetPlacement>,
    bounds: WidgetGridBounds,
    recordHistory = true,
  ): DeckWidgetEditorResult => {
    const current = history.current();
    if (Object.keys(placements).some((instanceId) => (
      !current.some((instance) => instance.id === instanceId)
    ))) return { ok: false, reason: "missing_item" };
    const next = current.map((instance) => {
      const placement = placements[instance.id];
      return placement
        ? { ...instance, layouts: { ...instance.layouts, [layoutKey]: { ...placement } } }
        : instance;
    });
    const valid = Object.entries(placements).every(([instanceId, placement]) => (
      canPlaceWidget(next, instanceId, layoutKey, placement, bounds)
    ));
    if (!valid) return { ok: false, reason: "collision" };
    if (recordHistory) commit(next);
    else {
      history.replaceCurrent(next);
      dirty = true;
    }
    return { ok: true };
  };

  const swap = (
    firstInstanceId: string,
    secondInstanceId: string,
    layoutKey: string,
    bounds: WidgetGridBounds,
  ): DeckWidgetEditorResult => {
    const current = history.current();
    const first = current.find(({ id }) => id === firstInstanceId);
    const second = current.find(({ id }) => id === secondInstanceId);
    if (!first || !second) return { ok: false, reason: "missing_item" };
    const firstPlacement = first.layouts[layoutKey];
    const secondPlacement = second.layouts[layoutKey];
    if (!firstPlacement || !secondPlacement) return { ok: false, reason: "missing_item" };

    const otherInstances = current.filter(({ id }) => (
      id !== firstInstanceId && id !== secondInstanceId
    ));
    if (!canPlaceWidget(otherInstances, firstInstanceId, layoutKey, secondPlacement, bounds)
      || !canPlaceWidget(otherInstances, secondInstanceId, layoutKey, firstPlacement, bounds)) {
      return { ok: false, reason: "collision" };
    }

    commit(current.map((instance) => {
      if (instance.id === firstInstanceId) {
        return { ...instance, layouts: { ...instance.layouts, [layoutKey]: { ...secondPlacement } } };
      }
      if (instance.id === secondInstanceId) {
        return { ...instance, layouts: { ...instance.layouts, [layoutKey]: { ...firstPlacement } } };
      }
      return instance;
    }));
    return { ok: true };
  };

  const configure = (
    instanceId: string,
    definition: WidgetDefinition,
    config: Record<string, unknown>,
  ) => {
    const current = history.current();
    const instance = current.find((candidate) => candidate.id === instanceId);
    if (!instance) return false;
    const validated = validateWidgetConfig(definition.configSchema, {
      ...instance.config,
      ...config,
    });
    if (!validated.valid) return false;
    commit(current.map((instance) => instance.id === instanceId
      ? { ...instance, config: validated.value }
      : instance));
    return true;
  };

  const bind = (instanceId: string, binding: WidgetBinding) => {
    const current = history.current();
    const instance = current.find((candidate) => candidate.id === instanceId);
    if (!instance) return false;
    commit(current.map((candidate) => candidate.id === instanceId
      ? {
          ...candidate,
          bindings: [
            ...candidate.bindings.filter((existing) => existing.portId !== binding.portId),
            binding,
          ],
        }
      : candidate));
    return true;
  };

  const unbind = (instanceId: string, portId: string) => {
    const current = history.current();
    const instance = current.find((candidate) => candidate.id === instanceId);
    if (!instance || !instance.bindings.some((binding) => binding.portId === portId)) return false;
    commit(current.map((candidate) => candidate.id === instanceId
      ? { ...candidate, bindings: candidate.bindings.filter((binding) => binding.portId !== portId) }
      : candidate));
    return true;
  };

  return {
    instances: () => history.current(),
    revision: () => revision,
    isDirty: () => dirty,
    add,
    insert,
    remove,
    removeMany,
    place,
    placeWithReflow,
    moveMany,
    replaceLayout,
    swap,
    configure,
    bind,
    unbind,
    undo: () => {
      const before = history.current();
      const after = history.undo();
      if (after !== before) dirty = true;
      return after;
    },
    redo: () => {
      const before = history.current();
      const after = history.redo();
      if (after !== before) dirty = true;
      return after;
    },
    canUndo: history.canUndo,
    canRedo: history.canRedo,
    markSaved: (savedRevision: number, _canonicalInstances = history.current()) => {
      revision = savedRevision;
      dirty = false;
    },
    reset: (instances: DeckWidgetInstance[], nextRevision: number) => {
      revision = nextRevision;
      history.reset(instances);
      dirty = false;
    },
  };
};
