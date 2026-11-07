import { mount } from "@vue/test-utils";
import { computed, defineComponent } from "vue";
import { createPinia, setActivePinia } from "pinia";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useMainStore } from "@base/stores/Main";
import Notifications from "@base/components/Notifications/index.vue";
import NotificationItem from "@base/components/Notifications/item.vue";
import type { DeckWidgetIntentAckMessage } from "@base/types/deckWidgetProtocol";
import { handleDeckWidgetIntentAck } from "./useWebSocketHandlers";

vi.mock("@tauri-apps/api/event", () => ({ emit: vi.fn().mockResolvedValue(undefined) }));
vi.mock("@base/composables/useSyncedDeckWidgets", () => ({ useSyncedDeckWidgets: vi.fn() }));
vi.mock("@base/composables/useWidgetRegistry", () => ({ useWidgetRegistry: vi.fn() }));
vi.mock("@base/composables/usePortableWidgetAssets", () => ({ usePortableWidgetAssets: vi.fn() }));

const rejected = (overrides: Partial<DeckWidgetIntentAckMessage> = {}): DeckWidgetIntentAckMessage => ({
  action: "deck_widget_intent_ack", protocolVersion: 1, deckId: 1, instanceId: "button:1",
  portId: "primary", sequence: 1, interactionSessionId: "phone-session", status: "rejected",
  code: "IntegrationError", message: "OBS disconnected", ...overrides,
});

describe("mobile widget action failure feedback", () => {
  beforeEach(() => { setActivePinia(createPinia()); vi.useFakeTimers(); });
  afterEach(() => { vi.useRealTimers(); });

  it("shows the rejected reason in the shared themed notification and auto-closes it", async () => {
    const shell = defineComponent({
      components: { Notifications },
      setup: () => ({ notifications: computed(() => useMainStore().getOpenNotificationsForWindow("mobile")) }),
      template: '<Notifications :notifications="notifications" />',
    });
    const wrapper = mount(shell, { global: { components: { NotificationsItem: NotificationItem } } });
    try {
      handleDeckWidgetIntentAck(rejected());
      await wrapper.vm.$nextTick();
      expect(wrapper.text()).toContain("OBS disconnected");
      expect(wrapper.findAll(".notification-item")).toHaveLength(1);
      await vi.advanceTimersByTimeAsync(5100);
      expect(wrapper.find(".notification-item").exists()).toBe(false);
      handleDeckWidgetIntentAck(rejected());
      await wrapper.vm.$nextTick();
      expect(wrapper.find(".notification-item").exists()).toBe(false);
    } finally { wrapper.unmount(); }
  });

  it("deduplicates transport replay but lets a new interaction report its failure", () => {
    handleDeckWidgetIntentAck(rejected());
    handleDeckWidgetIntentAck(rejected());
    expect(useMainStore().openNotifications).toHaveLength(1);
    handleDeckWidgetIntentAck(rejected({ sequence: 2 }));
    expect(useMainStore().openNotifications).toHaveLength(2);
  });

  it("does not treat acknowledgement/acceptance as failure and falls back to the error code", () => {
    for (const status of ["accepted", "scheduled", "stopped", "completed"] as const) {
      handleDeckWidgetIntentAck(rejected({ status }));
    }
    expect(useMainStore().openNotifications).toHaveLength(0);
    handleDeckWidgetIntentAck(rejected({ message: " " }));
    expect(useMainStore().openNotifications[0].message).toBe("IntegrationError");
    expect(useMainStore().getOpenNotificationsForWindow("simulator")).toHaveLength(0);
  });
});
