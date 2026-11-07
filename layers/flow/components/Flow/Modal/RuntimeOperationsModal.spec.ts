import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import RuntimeOperationsModal from "./RuntimeOperationsModal.vue";

const recoveryRun = {
  runId: "run-recovery-1",
  flowId: "flow-1",
  flowDefinition: { name: "Broadcast recovery" },
  nodeResults: {
    overlay: { nodeId: "overlay", status: "Running" },
  },
};

const runtimeInspection = {
  recoveryRuns: 1,
  pendingOverlayCommands: [],
  sideEffects: [],
  deadLetters: [],
  metrics: {},
};

const configureInspection = () => {
  invoke.mockImplementation((command: string) => {
    switch (command) {
      case "inspect_workflow_runtime":
        return Promise.resolve(runtimeInspection);
      case "list_flow_recovery_runs":
        return Promise.resolve([recoveryRun]);
      case "list_workflow_wakes":
        return Promise.resolve([]);
      case "get_metrics_access_token":
        return Promise.resolve("metrics-token");
      case "get_server_info_new":
        return Promise.resolve({ ip: "127.0.0.1", port: 9001 });
      default:
        return Promise.resolve(undefined);
    }
  });
};

const button = (wrapper: ReturnType<typeof mount>, label: string) => {
  const match = wrapper
    .findAll("button")
    .find((candidate) => candidate.text().trim() === label);
  expect(match, `button '${label}'`).toBeDefined();
  return match!;
};

describe("RuntimeOperationsModal recovery decisions", () => {
  beforeEach(() => {
    configureInspection();
  });

  it("requires confirmation and invokes the safe-resume command", async () => {
    const wrapper = mount(RuntimeOperationsModal, { props: { show: true } });
    await flushPromises();

    expect(wrapper.text()).toContain("Broadcast recovery");
    await button(wrapper, "Review safe resume").trigger("click");
    expect(wrapper.text()).toContain("runtime rejects an unsafe checkpoint");

    await button(wrapper, "Confirm safe resume").trigger("click");
    await flushPromises();

    expect(invoke).toHaveBeenCalledWith("resume_flow_recovery_run", {
      runId: "run-recovery-1",
    });
  });

  it("fails closed with an operator reason only after confirmation", async () => {
    const wrapper = mount(RuntimeOperationsModal, { props: { show: true } });
    await flushPromises();

    await button(wrapper, "Fail safely").trigger("click");
    expect(wrapper.text()).toContain("without repeating its external side effects");

    await button(wrapper, "Confirm fail safely").trigger("click");
    await flushPromises();

    expect(invoke).toHaveBeenCalledWith("fail_flow_recovery_run", {
      runId: "run-recovery-1",
      reason: "Operator selected fail safely from runtime diagnostics",
    });
  });

  it("keeps the run visible when safe resume is rejected", async () => {
    invoke.mockImplementation((command: string) => {
      if (command === "resume_flow_recovery_run") {
        return Promise.reject(new Error("checkpoint requires manual review"));
      }
      if (command === "inspect_workflow_runtime") return Promise.resolve(runtimeInspection);
      if (command === "list_flow_recovery_runs") return Promise.resolve([recoveryRun]);
      if (command === "list_workflow_wakes") return Promise.resolve([]);
      return Promise.resolve(null);
    });
    const wrapper = mount(RuntimeOperationsModal, { props: { show: true } });
    await flushPromises();

    await button(wrapper, "Review safe resume").trigger("click");
    await button(wrapper, "Confirm safe resume").trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("Safe resume was rejected");
    expect(wrapper.text()).toContain("Broadcast recovery");
  });
});
