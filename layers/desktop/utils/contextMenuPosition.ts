export type ContextMenuSide = "left" | "right";

export interface ContextMenuRect {
  top: number;
  right: number;
  bottom: number;
  left: number;
}

export interface ContextMenuPlacement {
  left: number;
  top: number;
  maxHeight: number;
  side: ContextMenuSide;
}

interface ContextMenuPlacementOptions {
  anchor: ContextMenuRect;
  menuWidth: number;
  menuHeight: number;
  viewportWidth: number;
  viewportHeight: number;
  preferredSide?: ContextMenuSide;
  gap?: number;
  viewportPadding?: number;
}

const clamp = (value: number, minimum: number, maximum: number) => (
  Math.min(Math.max(value, minimum), Math.max(minimum, maximum))
);

/**
 * Places a side menu in the visible webview viewport. The preferred side is
 * used when it fits; otherwise the roomier side wins and the menu is shifted
 * so it never relies on a clipping ancestor for visibility.
 */
export const resolveContextMenuPlacement = ({
  anchor,
  menuWidth,
  menuHeight,
  viewportWidth,
  viewportHeight,
  preferredSide = "right",
  gap = 6,
  viewportPadding = 8,
}: ContextMenuPlacementOptions): ContextMenuPlacement => {
  const usableWidth = Math.max(0, viewportWidth - viewportPadding * 2);
  const width = Math.min(menuWidth, usableWidth);
  const rightSpace = viewportWidth - viewportPadding - anchor.right - gap;
  const leftSpace = anchor.left - viewportPadding - gap;
  const preferredFits = preferredSide === "right"
    ? rightSpace >= width
    : leftSpace >= width;
  const alternateFits = preferredSide === "right"
    ? leftSpace >= width
    : rightSpace >= width;

  let side = preferredSide;
  if (!preferredFits && alternateFits) {
    side = preferredSide === "right" ? "left" : "right";
  } else if (!preferredFits && !alternateFits) {
    side = rightSpace >= leftSpace ? "right" : "left";
  }

  const desiredLeft = side === "right"
    ? anchor.right + gap
    : anchor.left - width - gap;
  const maxHeight = Math.max(0, viewportHeight - viewportPadding * 2);
  const visibleHeight = Math.min(menuHeight, maxHeight);

  return {
    left: clamp(desiredLeft, viewportPadding, viewportWidth - width - viewportPadding),
    top: clamp(anchor.top - 4, viewportPadding, viewportHeight - visibleHeight - viewportPadding),
    maxHeight,
    side,
  };
};
