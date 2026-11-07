import { describe, expect, it } from "vitest";
import { permitsPluginEvent, permitsStreamEvent } from "./pluginEventScope";

describe("plugin event delivery scope", () => {
  const sensitive = { tokenLikeFixture: "fixture-not-a-real-secret" };

  it("delivers only explicitly subscribed stream event types", () => {
    const scope = { subscriptions: ["follow"] };
    expect(permitsStreamEvent(scope, { type: "follow", ...sensitive })).toBe(
      true,
    );
    expect(
      permitsStreamEvent(scope, { type: "chatMessage", ...sensitive }),
    ).toBe(false);
    expect(permitsStreamEvent(scope, sensitive)).toBe(false);
  });

  it("allows a declared same-package event without enabling cross-package impersonation", () => {
    const scope = { subscriptions: ["statusChanged"] };
    expect(
      permitsPluginEvent(scope, "alpha", {
        sourcePackageId: "alpha",
        eventType: "plugin.alpha.statusChanged",
        declaredEventType: "statusChanged",
        payload: sensitive,
      }),
    ).toBe(true);
    expect(
      permitsPluginEvent(scope, "alpha", {
        sourcePackageId: "victim",
        eventType: "plugin.victim.statusChanged",
        declaredEventType: "statusChanged",
        payload: sensitive,
      }),
    ).toBe(false);
  });

  it("supports an explicitly granted cross-package event namespace", () => {
    const scope = { subscriptions: ["plugin.beta.ready"] };
    expect(
      permitsPluginEvent(scope, "alpha", {
        sourcePackageId: "beta",
        eventType: "plugin.beta.ready",
        declaredEventType: "ready",
      }),
    ).toBe(true);
    expect(
      permitsPluginEvent(scope, "alpha", {
        sourcePackageId: "beta",
        eventType: "plugin.beta.private",
        declaredEventType: "private",
      }),
    ).toBe(false);
  });
});
