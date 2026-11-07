import { describe, expect, it } from "vitest";
import { resolveContextMenuPlacement } from "./contextMenuPosition";

describe("resolveContextMenuPlacement", () => {
  it("places a submenu to the right when it fits", () => {
    expect(resolveContextMenuPlacement({
      anchor: { top: 100, right: 220, bottom: 130, left: 100 },
      menuWidth: 176,
      menuHeight: 200,
      viewportWidth: 800,
      viewportHeight: 600,
    })).toMatchObject({ left: 226, top: 96, side: "right" });
  });

  it("flips left at the right edge", () => {
    expect(resolveContextMenuPlacement({
      anchor: { top: 100, right: 790, bottom: 130, left: 650 },
      menuWidth: 176,
      menuHeight: 200,
      viewportWidth: 800,
      viewportHeight: 600,
    })).toMatchObject({ left: 468, side: "left" });
  });

  it("shifts a tall submenu into the visible viewport", () => {
    const placement = resolveContextMenuPlacement({
      anchor: { top: 550, right: 300, bottom: 580, left: 200 },
      menuWidth: 176,
      menuHeight: 900,
      viewportWidth: 800,
      viewportHeight: 600,
    });

    expect(placement.top).toBe(8);
    expect(placement.maxHeight).toBe(584);
  });
});
