import type { DeckWidgetInstance, WidgetDefinition, WidgetPlacement } from "@base/types/deckWidget";
import {
  canPlaceWidget,
  placementFits,
  type WidgetGridBounds,
} from "@base/utils/widgetLayout";

export interface ResponsiveWidgetLayoutOptions {
  /** Keep valid placements already saved for the target orientation fixed and
   * derive positions only for widgets which do not have one yet. */
  preserveTargetPlacements?: boolean;
}

const projectedCoordinate = (
  value: number,
  sourceAvailable: number,
  targetAvailable: number,
) => sourceAvailable <= 0
  ? 0
  : Math.round((value / sourceAvailable) * targetAvailable);

const positionsByDistance = (
  preferredX: number,
  preferredY: number,
  maxX: number,
  maxY: number,
) => {
  const positions: Array<{ x: number; y: number }> = [];
  for (let y = 0; y <= maxY; y += 1) {
    for (let x = 0; x <= maxX; x += 1) positions.push({ x, y });
  }
  return positions.sort((first, second) => (
    Math.abs(first.x - preferredX) + Math.abs(first.y - preferredY)
    - Math.abs(second.x - preferredX) - Math.abs(second.y - preferredY)
    || first.y - second.y
    || first.x - second.x
  ));
};

export const deriveResponsiveWidgetLayout = (
  instances: DeckWidgetInstance[],
  sourceLayoutKey: string,
  targetLayoutKey: string,
  sourceBounds: WidgetGridBounds,
  targetBounds: WidgetGridBounds,
  definitionFor: (instance: DeckWidgetInstance) => WidgetDefinition,
  options: ResponsiveWidgetLayoutOptions = {},
): Record<string, WidgetPlacement> | undefined => {
  type PreparedWidget = {
    instance: DeckWidgetInstance;
    source: WidgetPlacement;
    definition: WidgetDefinition;
    cols: number;
    rows: number;
  };
  const candidates: PreparedWidget[] = [];
  const preserved: DeckWidgetInstance[] = [];
  const preservedPlacements: Record<string, WidgetPlacement> = {};
  for (const instance of instances) {
    const definition = definitionFor(instance);
    const target = instance.layouts[targetLayoutKey];
    if (options.preserveTargetPlacements && target) {
      if (
        target.cols < definition.minSpan.cols
        || target.rows < definition.minSpan.rows
        || target.cols > definition.maxSpan.cols
        || target.rows > definition.maxSpan.rows
        || !placementFits(target, targetBounds)
        || !canPlaceWidget(preserved, instance.id, targetLayoutKey, target, targetBounds)
      ) return undefined;
      const placement = { ...target };
      preservedPlacements[instance.id] = placement;
      preserved.push({
        ...instance,
        layouts: { ...instance.layouts, [targetLayoutKey]: placement },
      });
      continue;
    }
    const source = instance.layouts[sourceLayoutKey]
      ?? instance.layouts.default
      ?? instance.layouts[targetLayoutKey];
    // A responsive result must account for every deck widget. Returning a
    // partial map here would make a layout look valid while silently hiding an
    // instance in the other orientation.
    if (!source) return undefined;
    if (definition.minSpan.cols > targetBounds.cols || definition.minSpan.rows > targetBounds.rows) {
      return undefined;
    }
    const cols = Math.max(
      definition.minSpan.cols,
      Math.min(source.cols, definition.maxSpan.cols, targetBounds.cols),
    );
    const rows = Math.max(
      definition.minSpan.rows,
      Math.min(source.rows, definition.maxSpan.rows, targetBounds.rows),
    );
    candidates.push({ instance, source, definition, cols, rows });
  }
  const totalArea = preserved.reduce((sum, instance) => {
    const placement = instance.layouts[targetLayoutKey]!;
    return sum + placement.cols * placement.rows;
  }, candidates.reduce((sum, { cols, rows }) => sum + cols * rows, 0));
  if (totalArea > targetBounds.cols * targetBounds.rows) return undefined;

  // Larger spans have fewer legal placements and must be packed first. The old
  // greedy row-order algorithm could fill a row with 1x1 widgets before a later
  // 2x1 widget was considered, falsely reporting that a full 5x3/3x5 deck was
  // impossible even though a valid packing existed.
  candidates.sort((first, second) => (
    second.cols * second.rows - first.cols * first.rows
    || Math.max(second.cols, second.rows) - Math.max(first.cols, first.rows)
    || first.source.y - second.source.y
    || first.source.x - second.source.x
    || first.instance.id.localeCompare(second.instance.id)
  ));

  const solve = (allowRotatedSpans: boolean) => {
    const projected: DeckWidgetInstance[] = [...preserved];
    const placements: Record<string, WidgetPlacement> = Object.fromEntries(
      Object.entries(preservedPlacements).map(([id, placement]) => [id, { ...placement }]),
    );
    const failedStates = new Set<string>();

    const visit = (index: number): boolean => {
      if (index === candidates.length) return true;
      const state = `${index}:${projected
        .map(({ layouts }) => {
          const placement = layouts[targetLayoutKey];
          return placement ? `${placement.x},${placement.y},${placement.cols},${placement.rows}` : "";
        })
        .sort()
        .join("|")}`;
      if (failedStates.has(state)) return false;

      const entry = candidates[index]!;
      const spanOptions = [{ cols: entry.cols, rows: entry.rows }];
      const rotated = { cols: entry.rows, rows: entry.cols };
      if (
        allowRotatedSpans
        && (rotated.cols !== entry.cols || rotated.rows !== entry.rows)
        && rotated.cols >= entry.definition.minSpan.cols
        && rotated.rows >= entry.definition.minSpan.rows
        && rotated.cols <= entry.definition.maxSpan.cols
        && rotated.rows <= entry.definition.maxSpan.rows
        && rotated.cols <= targetBounds.cols
        && rotated.rows <= targetBounds.rows
      ) spanOptions.push(rotated);

      for (const span of spanOptions) {
        const preferredX = projectedCoordinate(
          entry.source.x,
          Math.max(0, sourceBounds.cols - entry.source.cols),
          Math.max(0, targetBounds.cols - span.cols),
        );
        const preferredY = projectedCoordinate(
          entry.source.y,
          Math.max(0, sourceBounds.rows - entry.source.rows),
          Math.max(0, targetBounds.rows - span.rows),
        );
        for (const position of positionsByDistance(
          preferredX,
          preferredY,
          targetBounds.cols - span.cols,
          targetBounds.rows - span.rows,
        )) {
          const placement = { ...position, ...span };
          if (!canPlaceWidget(
            projected,
            entry.instance.id,
            targetLayoutKey,
            placement,
            targetBounds,
          )) continue;
          placements[entry.instance.id] = placement;
          projected.push({
            ...entry.instance,
            layouts: { ...entry.instance.layouts, [targetLayoutKey]: placement },
          });
          if (visit(index + 1)) return true;
          projected.pop();
          delete placements[entry.instance.id];
        }
      }
      failedStates.add(state);
      return false;
    };

    return visit(0) ? placements : undefined;
  };

  // Preserve widget dimensions whenever possible. Rotation is only a fallback
  // for a transposed grid that cannot contain the original directional spans.
  return solve(false) ?? solve(true);
};
