import { describe, expect, it, vi } from "vitest";
import type { DeckWidgetInstance, WidgetIntent } from "@base/types/deckWidget";
import { useWidgetInteractionRouter } from "@base/composables/useWidgetInteractionRouter";

const instance = (
  policy: DeckWidgetInstance["bindings"][number]["policy"],
): DeckWidgetInstance => ({
  id: "button:1",
  definitionId: "vortideck.action-button",
  configVersion: 1,
  config: {},
  layouts: { default: { x: 0, y: 0, cols: 1, rows: 1 } },
  bindings: [{ id: "binding:1", portId: "primary", action: "action", policy }],
  legacyButtonId: 1,
  enabled: true,
});

const intent = (phase: WidgetIntent["phase"], sequence = 1): WidgetIntent => ({
  instanceId: "button:1",
  portId: "primary",
  legacyButtonId: 1,
  kind: "trigger",
  phase,
  sequence,
  timestamp: 1_000,
  inputSource: "mouse",
});

describe("widget interaction router", () => {
  it("dispatches committed run-once inputs", async () => {
    const widget = instance({ type: "run_once" });
    const dispatch = vi.fn();
    const router = useWidgetInteractionRouter({
      getInstance: () => widget,
      dispatch,
    });

    expect(await router.route(intent("commit"))).toBe(true);
    expect(dispatch).toHaveBeenCalledWith(
      expect.objectContaining({
        type: "execute",
        instance: widget,
        commit: true,
      }),
      expect.anything(),
    );
  });

  it("maps repeat start and end into lifecycle operations", async () => {
    const widget = instance({ type: "repeat", holdMs: 300, intervalMs: 100 });
    const dispatch = vi.fn();
    const router = useWidgetInteractionRouter({
      getInstance: () => widget,
      dispatch,
    });

    await router.route(intent("start", 1));
    await router.route(intent("end", 2));
    expect(dispatch.mock.calls.map(([operation]) => operation.type)).toEqual([
      "repeat_start",
      "repeat_stop",
    ]);
  });

  it("rejects stale intent sequences", async () => {
    const widget = instance({ type: "run_once" });
    const dispatch = vi.fn();
    const router = useWidgetInteractionRouter({
      getInstance: () => widget,
      dispatch,
    });

    await router.route(intent("commit", 2));
    expect(await router.route(intent("commit", 1))).toBe(false);
    expect(dispatch).toHaveBeenCalledTimes(1);
  });

  it("does not let one mounted client suppress another", async () => {
    const widget = instance({ type: "run_once" });
    const dispatch = vi.fn();
    const first = useWidgetInteractionRouter({
      getInstance: () => widget,
      dispatch,
      interactionSessionId: "client-a-mount-1",
    });
    const second = useWidgetInteractionRouter({
      getInstance: () => widget,
      dispatch,
      interactionSessionId: "client-b-mount-1",
    });

    expect(await first.route(intent("commit", 1))).toBe(true);
    expect(await second.route(intent("commit", 1))).toBe(true);
    expect(
      dispatch.mock.calls.map(([, routed]) => routed.interactionSessionId),
    ).toEqual(["client-a-mount-1", "client-b-mount-1"]);
  });
});
