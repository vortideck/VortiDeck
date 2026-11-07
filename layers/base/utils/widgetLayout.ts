import type { DeckWidgetInstance, WidgetPlacement } from "@base/types/deckWidget";

export interface WidgetGridBounds {
  cols: number;
  rows: number;
}

export type WidgetPlacementFailure = "out_of_bounds" | "collision" | "missing_item";

export type WidgetPlacementResult =
  | { ok: true; instances: DeckWidgetInstance[] }
  | { ok: false; reason: WidgetPlacementFailure; instances: DeckWidgetInstance[] };

export const placementsOverlap = (left: WidgetPlacement, right: WidgetPlacement) => (
  left.x < right.x + right.cols
  && left.x + left.cols > right.x
  && left.y < right.y + right.rows
  && left.y + left.rows > right.y
);

export const placementFits = (placement: WidgetPlacement, bounds: WidgetGridBounds) => (
  placement.x >= 0
  && placement.y >= 0
  && placement.cols > 0
  && placement.rows > 0
  && placement.x + placement.cols <= bounds.cols
  && placement.y + placement.rows <= bounds.rows
);

/** Returns the first one-based row-major cell in a widget footprint which is
 * outside the user's physical deck-cell allowance. */
export const firstRestrictedCellPosition = (
  placement: WidgetPlacement,
  bounds: WidgetGridBounds,
  allowedCells: number,
): number | undefined => {
  const limit = Math.max(0, Math.floor(allowedCells));
  for (let y = placement.y; y < placement.y + placement.rows; y += 1) {
    for (let x = placement.x; x < placement.x + placement.cols; x += 1) {
      const position = y * bounds.cols + x + 1;
      if (position > limit) return position;
    }
  }
  return undefined;
};

export const placementExceedsCellLimit = (
  placement: WidgetPlacement,
  bounds: WidgetGridBounds,
  allowedCells: number,
) => firstRestrictedCellPosition(placement, bounds, allowedCells) !== undefined;

export const canPlaceWidget = (
  instances: DeckWidgetInstance[],
  instanceId: string,
  layoutKey: string,
  placement: WidgetPlacement,
  bounds: WidgetGridBounds,
) => placementFits(placement, bounds) && !instances.some((candidate) => {
  if (candidate.id === instanceId || !candidate.enabled) return false;
  const candidatePlacement = candidate.layouts[layoutKey];
  return candidatePlacement ? placementsOverlap(placement, candidatePlacement) : false;
});

export const placeWidget = (
  instances: DeckWidgetInstance[],
  instanceId: string,
  layoutKey: string,
  placement: WidgetPlacement,
  bounds: WidgetGridBounds,
): WidgetPlacementResult => {
  const index = instances.findIndex((instance) => instance.id === instanceId);
  if (index < 0) return { ok: false, reason: "missing_item", instances };
  if (!placementFits(placement, bounds)) {
    return { ok: false, reason: "out_of_bounds", instances };
  }
  if (!canPlaceWidget(instances, instanceId, layoutKey, placement, bounds)) {
    return { ok: false, reason: "collision", instances };
  }
  return {
    ok: true,
    instances: instances.map((instance, candidateIndex) => candidateIndex === index
      ? { ...instance, layouts: { ...instance.layouts, [layoutKey]: { ...placement } } }
      : instance),
  };
};

/** Move a selected set as one rigid group without changing relative geometry. */
export const moveWidgetsBy = (
  instances: DeckWidgetInstance[],
  instanceIds: Iterable<string>,
  layoutKey: string,
  delta: { x: number; y: number },
  bounds: WidgetGridBounds,
): WidgetPlacementResult => {
  const ids = new Set(instanceIds);
  if (!ids.size) return { ok: false, reason: "missing_item", instances };
  const selected = instances.filter(({ id }) => ids.has(id));
  if (selected.length !== ids.size || selected.some(({ layouts }) => !layouts[layoutKey])) {
    return { ok: false, reason: "missing_item", instances };
  }
  const next = instances.map((instance) => {
    if (!ids.has(instance.id)) return instance;
    const placement = instance.layouts[layoutKey]!;
    return {
      ...instance,
      layouts: {
        ...instance.layouts,
        [layoutKey]: {
          ...placement,
          x: placement.x + delta.x,
          y: placement.y + delta.y,
        },
      },
    };
  });
  const valid = next
    .filter(({ id }) => ids.has(id))
    .every((instance) => canPlaceWidget(
      next,
      instance.id,
      layoutKey,
      instance.layouts[layoutKey]!,
      bounds,
    ));
  return valid
    ? { ok: true, instances: next }
    : { ok: false, reason: "collision", instances };
};

/** Moves a selection as one rigid group and repacks any unselected widgets
 * covered by the target footprint. The operation is atomic. */
export const moveWidgetsWithReflow = (
  instances: DeckWidgetInstance[],
  instanceIds: Iterable<string>,
  layoutKey: string,
  delta: { x: number; y: number },
  bounds: WidgetGridBounds,
): WidgetPlacementResult => {
  const ids = new Set(instanceIds);
  if (!ids.size) return { ok: false, reason: "missing_item", instances };
  const selected = instances.filter(({ id }) => ids.has(id));
  if (selected.length !== ids.size || selected.some(({ layouts }) => !layouts[layoutKey])) {
    return { ok: false, reason: "missing_item", instances };
  }
  const targetPlacements = new Map(selected.map((instance) => {
    const placement = instance.layouts[layoutKey]!;
    return [instance.id, {
      ...placement,
      x: placement.x + delta.x,
      y: placement.y + delta.y,
    }];
  }));
  if ([...targetPlacements.values()].some((placement) => !placementFits(placement, bounds))) {
    return { ok: false, reason: "out_of_bounds", instances };
  }

  const targets = [...targetPlacements.values()];
  const displaced = instances.filter((candidate) => {
    if (ids.has(candidate.id) || !candidate.enabled) return false;
    const placement = candidate.layouts[layoutKey];
    return placement ? targets.some((target) => placementsOverlap(target, placement)) : false;
  });
  if (!displaced.length) return moveWidgetsBy(instances, ids, layoutKey, delta, bounds);

  const displacedIds = new Set(displaced.map(({ id }) => id));
  const placed = instances
    .filter((candidate) => !ids.has(candidate.id) && !displacedIds.has(candidate.id))
    .concat(selected.map((instance) => ({
      ...instance,
      layouts: { ...instance.layouts, [layoutKey]: targetPlacements.get(instance.id)! },
    })));
  const nextPlacements = new Map(targetPlacements);
  const sourcePlacements = selected.map(({ layouts }) => layouts[layoutKey]!);
  const ordered = [...displaced].sort((first, second) => {
    const firstPlacement = first.layouts[layoutKey]!;
    const secondPlacement = second.layouts[layoutKey]!;
    return secondPlacement.cols * secondPlacement.rows - firstPlacement.cols * firstPlacement.rows
      || firstPlacement.y - secondPlacement.y
      || firstPlacement.x - secondPlacement.x;
  });

  const isInsideVacatedArea = (placement: WidgetPlacement) => {
    for (let y = placement.y; y < placement.y + placement.rows; y += 1) {
      for (let x = placement.x; x < placement.x + placement.cols; x += 1) {
        if (!sourcePlacements.some((source) => (
          x >= source.x && x < source.x + source.cols
          && y >= source.y && y < source.y + source.rows
        ))) return false;
      }
    }
    return true;
  };

  const candidatePositions = (candidate: DeckWidgetInstance) => {
    const current = candidate.layouts[layoutKey]!;
    const positions: WidgetPlacement[] = [];
    for (let y = 0; y <= bounds.rows - current.rows; y += 1) {
      for (let x = 0; x <= bounds.cols - current.cols; x += 1) {
        positions.push({ x, y, cols: current.cols, rows: current.rows });
      }
    }
    return positions.sort((first, second) => (
      Number(isInsideVacatedArea(second)) - Number(isInsideVacatedArea(first))
      || Math.abs(first.x - current.x) + Math.abs(first.y - current.y)
        - Math.abs(second.x - current.x) - Math.abs(second.y - current.y)
      || first.y - second.y
      || first.x - second.x
    ));
  };

  const failedStates = new Set<string>();
  const visit = (index: number): boolean => {
    if (index === ordered.length) return true;
    const occupancy = placed
      .map(({ layouts }) => {
        const placement = layouts[layoutKey];
        return placement ? `${placement.x},${placement.y},${placement.cols},${placement.rows}` : "";
      })
      .sort()
      .join("|");
    const state = `${index}:${occupancy}`;
    if (failedStates.has(state)) return false;
    const candidate = ordered[index]!;
    for (const target of candidatePositions(candidate)) {
      if (!canPlaceWidget(placed, candidate.id, layoutKey, target, bounds)) continue;
      placed.push({ ...candidate, layouts: { ...candidate.layouts, [layoutKey]: target } });
      nextPlacements.set(candidate.id, target);
      if (visit(index + 1)) return true;
      nextPlacements.delete(candidate.id);
      placed.pop();
    }
    failedStates.add(state);
    return false;
  };

  if (!visit(0)) return { ok: false, reason: "collision", instances };
  return {
    ok: true,
    instances: instances.map((instance) => {
      const placement = nextPlacements.get(instance.id);
      return placement
        ? { ...instance, layouts: { ...instance.layouts, [layoutKey]: { ...placement } } }
        : instance;
    }),
  };
};

/** Moves a widget and atomically repacks every widget covered by its target.
 * Displaced widgets prefer the moving widget's vacated footprint, preserving
 * their relative order, and then the closest remaining grid cells. */
export const placeWidgetWithReflow = (
  instances: DeckWidgetInstance[],
  instanceId: string,
  layoutKey: string,
  placement: WidgetPlacement,
  bounds: WidgetGridBounds,
): WidgetPlacementResult => {
  const moving = instances.find((instance) => instance.id === instanceId);
  const source = moving?.layouts[layoutKey];
  if (!moving || !source) return { ok: false, reason: "missing_item", instances };
  if (!placementFits(placement, bounds)) {
    return { ok: false, reason: "out_of_bounds", instances };
  }
  const displaced = instances.filter((candidate) => {
    if (candidate.id === instanceId || !candidate.enabled) return false;
    const candidatePlacement = candidate.layouts[layoutKey];
    return candidatePlacement ? placementsOverlap(placement, candidatePlacement) : false;
  });
  if (!displaced.length) return placeWidget(instances, instanceId, layoutKey, placement, bounds);

  const displacedIds = new Set(displaced.map(({ id }) => id));
  const placed = instances
    .filter((candidate) => candidate.id !== instanceId && !displacedIds.has(candidate.id))
    .concat({ ...moving, layouts: { ...moving.layouts, [layoutKey]: { ...placement } } });
  const nextPlacements = new Map<string, WidgetPlacement>([[instanceId, placement]]);
  const ordered = [...displaced].sort((first, second) => {
    const firstPlacement = first.layouts[layoutKey]!;
    const secondPlacement = second.layouts[layoutKey]!;
    return secondPlacement.cols * secondPlacement.rows - firstPlacement.cols * firstPlacement.rows
      || firstPlacement.y - secondPlacement.y
      || firstPlacement.x - secondPlacement.x;
  });

  const candidatePositions = (candidate: DeckWidgetInstance) => {
    const current = candidate.layouts[layoutKey]!;
    const relativeX = current.x - placement.x;
    const relativeY = current.y - placement.y;
    const preferredX = source.x + relativeX;
    const preferredY = source.y + relativeY;
    const positions: WidgetPlacement[] = [];
    for (let y = 0; y <= bounds.rows - current.rows; y += 1) {
      for (let x = 0; x <= bounds.cols - current.cols; x += 1) {
        positions.push({ x, y, cols: current.cols, rows: current.rows });
      }
    }
    return positions.sort((first, second) => {
      const firstInVacated = first.x >= source.x
        && first.y >= source.y
        && first.x + first.cols <= source.x + source.cols
        && first.y + first.rows <= source.y + source.rows;
      const secondInVacated = second.x >= source.x
        && second.y >= source.y
        && second.x + second.cols <= source.x + source.cols
        && second.y + second.rows <= source.y + source.rows;
      return Number(secondInVacated) - Number(firstInVacated)
        || Math.abs(first.x - preferredX) + Math.abs(first.y - preferredY)
          - Math.abs(second.x - preferredX) - Math.abs(second.y - preferredY)
        || first.y - second.y
        || first.x - second.x;
    });
  };

  const failedStates = new Set<string>();
  const visit = (index: number): boolean => {
    if (index === ordered.length) return true;
    const occupancy = placed
      .map(({ layouts }) => {
        const current = layouts[layoutKey];
        return current ? `${current.x},${current.y},${current.cols},${current.rows}` : "";
      })
      .sort()
      .join("|");
    const state = `${index}:${occupancy}`;
    if (failedStates.has(state)) return false;
    const candidate = ordered[index]!;
    for (const target of candidatePositions(candidate)) {
      if (!canPlaceWidget(placed, candidate.id, layoutKey, target, bounds)) continue;
      placed.push({ ...candidate, layouts: { ...candidate.layouts, [layoutKey]: target } });
      nextPlacements.set(candidate.id, target);
      if (visit(index + 1)) return true;
      nextPlacements.delete(candidate.id);
      placed.pop();
    }
    failedStates.add(state);
    return false;
  };

  if (!visit(0)) return { ok: false, reason: "collision", instances };
  return {
    ok: true,
    instances: instances.map((instance) => {
      const next = nextPlacements.get(instance.id);
      return next
        ? { ...instance, layouts: { ...instance.layouts, [layoutKey]: { ...next } } }
        : instance;
    }),
  };
};

export const findFirstAvailablePlacement = (
  instances: DeckWidgetInstance[],
  instanceId: string,
  layoutKey: string,
  span: { cols: number; rows: number },
  bounds: WidgetGridBounds,
): WidgetPlacement | undefined => {
  for (let y = 0; y <= bounds.rows - span.rows; y += 1) {
    for (let x = 0; x <= bounds.cols - span.cols; x += 1) {
      const placement = { x, y, ...span };
      if (canPlaceWidget(instances, instanceId, layoutKey, placement, bounds)) return placement;
    }
  }
  return undefined;
};

export const createWidgetHistory = <T>(initial: T, limit = 50) => {
  let past: T[] = [];
  let current = initial;
  let future: T[] = [];

  return {
    current: () => current,
    canUndo: () => past.length > 0,
    canRedo: () => future.length > 0,
    push(next: T) {
      if (Object.is(next, current)) return current;
      past = [...past, current].slice(-Math.max(1, limit));
      current = next;
      future = [];
      return current;
    },
    replaceCurrent(next: T) {
      current = next;
      future = [];
      return current;
    },
    undo() {
      const previous = past.at(-1);
      if (previous === undefined) return current;
      past = past.slice(0, -1);
      future = [current, ...future];
      current = previous;
      return current;
    },
    redo() {
      const next = future[0];
      if (next === undefined) return current;
      future = future.slice(1);
      past = [...past, current].slice(-Math.max(1, limit));
      current = next;
      return current;
    },
    reset(next: T) {
      past = [];
      current = next;
      future = [];
      return current;
    },
  };
};
