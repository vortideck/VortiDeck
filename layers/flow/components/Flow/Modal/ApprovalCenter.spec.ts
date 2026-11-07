import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";

type EventHandler = (event: { payload: any }) => void;

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  handlers: new Map<string, EventHandler>(),
  wakes: [] as any[],
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (event: string, handler: EventHandler) => {
    mocks.handlers.set(event, handler);
    return vi.fn();
  }),
}));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ isFocused: () => Promise.resolve(true) }),
}));

import ApprovalCenter from "./ApprovalCenter.vue";
import { useMainStore } from "../../../../base/stores/Main";

const waitingApproval = {
  id: "wake-approval-1",
  runId: "run-1",
  nodeId: "approval-node",
  kind: "approval",
  status: "waiting",
  dueAt: null,
  payload: {
    title: "Publish scene change?",
    message: "Confirm before changing the live scene.",
    severity: "high",
    requestedChannels: ["desktop", "mobile"],
    decisionPolicy: { allowedChannels: ["desktop", "mobile_direct"] },
    requestSnapshot: { scene: "Interview" },
  },
};

describe("Workflow approval center", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mocks.handlers.clear();
    mocks.wakes = [waitingApproval];
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "list_workflow_wakes") return Promise.resolve(mocks.wakes);
      if (command === "resolve_workflow_approval") {
        mocks.wakes = [];
        return Promise.resolve({ outcome: "applied" });
      }
      return Promise.resolve(null);
    });
  });

  it("rehydrates a durable request without forcing a dialog and resolves it on review", async () => {
    const wrapper = mount(ApprovalCenter);
    await flushPromises();

    expect(wrapper.text()).toContain("1 approval waiting");
    expect(wrapper.text()).not.toContain("Publish scene change?");
    await wrapper.get("button").trigger("click");
    await flushPromises();
    expect(wrapper.text()).toContain("Publish scene change?");
    expect(wrapper.text()).toContain("workflow is paused");

    const approve = wrapper.findAll("button").find((candidate) => candidate.text() === "Approve");
    expect(approve).toBeDefined();
    await approve!.trigger("click");
    await flushPromises();

    expect(mocks.invoke).toHaveBeenCalledWith("resolve_workflow_approval", {
      wakeId: "wake-approval-1",
      decision: "approved",
      comment: null,
    });
    expect(wrapper.text()).not.toContain("Publish scene change?");
  });

  it("projects one live notification and removes it on cancellation", async () => {
    mocks.invoke.mockResolvedValueOnce([]);
    const wrapper = mount(ApprovalCenter);
    await flushPromises();

    mocks.handlers.get("flow_event")?.({
      payload: { type: "approvalRequested", runId: "run-1", request: waitingApproval },
    });
    await flushPromises();
    const mainStore = useMainStore();
    expect(mainStore.getOpenNotificationsForWindow("flow_editor")).toHaveLength(1);
    expect(mainStore.getOpenNotificationsForWindow("flow_editor")[0]?.key)
      .toBe("workflow-approval:wake-approval-1");
    expect(wrapper.text()).not.toContain("1 approval waiting");

    const notificationId = mainStore.getOpenNotificationsForWindow("flow_editor")[0]!.id;
    mainStore.closeNotification(notificationId);
    await flushPromises();
    expect(wrapper.text()).toContain("1 approval waiting");

    mocks.wakes = [];
    mocks.handlers.get("flow_event")?.({
      payload: { type: "flowCancelled", runId: "run-1" },
    });
    await flushPromises();
    expect(mainStore.getOpenNotificationsForWindow("flow_editor")).toHaveLength(0);
    expect(wrapper.text()).not.toContain("1 approval waiting");
  });
});
