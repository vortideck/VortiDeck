import { describe, expect, it } from "vitest";
import { toActionReferenceKind } from "./useHandleDragAction";

describe("toActionReferenceKind", () => {
  it.each([
    "macro",
    "program",
    "flow",
    "system_control",
    "http_request",
    "focus_window",
  ] as const)("keeps local action kind %s", (kind) => {
    expect(toActionReferenceKind(kind)).toBe(kind);
  });

  it.each(["obs", "discord", "third-party-provider", "saved_action"])(
    "maps provider action type %s to a saved action reference",
    (providerType) => {
      expect(toActionReferenceKind(providerType)).toBe("saved_action");
    },
  );
});
