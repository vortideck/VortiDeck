import { beforeEach, describe, expect, it, vi } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { useMainStore } from "@base/stores/Main";
import * as tauriEvent from "@tauri-apps/api/event";

vi.mock("@tauri-apps/api/event", () => ({
  emit: vi.fn().mockResolvedValue(undefined),
}));

describe("MainStore Notifications & Cross-Window Bridge", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
  });

  it("adds notification to notifications and openNotifications capped at 3", () => {
    const store = useMainStore();

    store.addNotification({ title: "Notif 1", message: "Msg 1", type: "info" });
    store.addNotification({ title: "Notif 2", message: "Msg 2", type: "info" });
    store.addNotification({ title: "Notif 3", message: "Msg 3", type: "info" });
    expect(store.openNotifications.length).toBe(3);
    expect(store.openNotifications[0]!.title).toBe("Notif 1");

    store.addNotification({ title: "Notif 4", message: "Msg 4", type: "info" });
    expect(store.openNotifications.length).toBe(3);
    expect(store.openNotifications[0]!.title).toBe("Notif 2");
    expect(store.openNotifications[2]!.title).toBe("Notif 4");
    expect(store.notifications.length).toBe(4);
  });

  it("filters notifications by targeted window ID", () => {
    const store = useMainStore();

    store.addNotification({ title: "Main Only", message: "For main window", type: "info" }, "main");
    store.addNotification({ title: "Flow Only", message: "For flow editor", type: "info" }, "flow_editor");

    const mainNotifications = store.getOpenNotificationsForWindow("main");
    expect(mainNotifications.some((n) => n.title === "Main Only")).toBe(true);
    expect(mainNotifications.some((n) => n.title === "Flow Only")).toBe(false);

    const flowNotifications = store.getOpenNotificationsForWindow("flow_editor");
    expect(flowNotifications.some((n) => n.title === "Flow Only")).toBe(true);
    expect(flowNotifications.some((n) => n.title === "Main Only")).toBe(false);
  });

  it("broadcasts cross-window notifications via Tauri event bridge", () => {
    const store = useMainStore();
    const buttonCallback = vi.fn();

    store.addNotification(
      {
        title: "Action Added",
        message: "Action assigned to button",
        type: "success",
        buttonText: "Undo",
        buttonFunction: buttonCallback,
      },
      ["main", "simulator"],
    );

    expect(tauriEvent.emit).toHaveBeenCalledWith(
      "vortideck:cross-window-notification",
      expect.objectContaining({
        windowIds: ["main", "simulator"],
        notification: expect.objectContaining({
          title: "Action Added",
          message: "Action assigned to button",
          type: "success",
          buttonFunction: undefined, // stripped for cross-window JSON serialization
        }),
      }),
    );
  });

  it("closes and removes notifications cleanly", () => {
    const store = useMainStore();
    store.addNotification({ title: "To Close", message: "msg", type: "info" });
    const id = store.openNotifications[0]!.id;

    store.closeNotification(id);
    expect(store.openNotifications.length).toBe(0);
    expect(store.notifications.length).toBe(1);

    store.removeNotification(id);
    expect(store.notifications.length).toBe(0);
  });
});
