import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import PreparationModal from "./PreparationModal.vue";

const rehearsal = {
  mode: "read_only" as const,
  externalEffects: false as const,
  flowRevision: "1234567890abcdef1234567890abcdef",
  generatedAt: "2026-09-05T13:00:00.000Z",
  ready: false,
  graph: { isValid: true, errors: [], warnings: [] },
  blockers: ["Switch scene: Binding is not installed"],
  warnings: [],
  executableNodes: [],
  actionHealth: [{
    nodeId: "2",
    nodeName: "Switch scene",
    stableActionId: "missing",
    preflight: {
      integrationId: "",
      actionId: "missing",
      status: "missing_target" as const,
      issues: [{
        code: "missing_saved_action",
        message: "Binding is not installed",
        recoverable: true,
      }],
    },
  }],
};

describe("PreparationModal", () => {
  it("states the rehearsal effect boundary and explains a blocker from preflight evidence", () => {
    const wrapper = mount(PreparationModal, {
      props: { show: true, mode: "rehearsal", report: rehearsal },
    });

    expect(wrapper.text()).toContain("No provider is called and no external effect is performed");
    expect(wrapper.text()).toContain("Switch scene: Binding is not installed");
    expect(wrapper.text()).not.toContain("Run real flow");
  });

  it("shows import isolation and emits confirmation without activating the recipe", async () => {
    const wrapper = mount(PreparationModal, {
      props: {
        show: true,
        mode: "import",
        importPreview: {
          envelope: {
            format: "vortideck.flow-recipe",
            version: 1,
            recipe: {
              id: "break-scene", version: "1.0.0", name: "Break scene",
              createdAt: "2026-09-05T13:00:00.000Z",
            },
            flow: {} as never,
            dependencies: [],
            review: { executableNodes: [], permissionsResetOnImport: true },
          },
          flow: {} as never,
          sourceFlowId: "recipe-flow",
          importedFlowId: "flow-new",
          missingBindings: [],
          executableNodes: [],
          permissionsReset: true,
        },
      },
    });

    expect(wrapper.text()).toContain("Fresh inactive flow");
    expect(wrapper.text()).toContain("Reset to private");
    expect(wrapper.text()).toContain("heuristic screen, not proof");
    const confirm = wrapper.findAll("button")
      .find((button) => button.text().trim() === "Import inactive flow");
    expect(confirm).toBeDefined();
    await confirm!.trigger("click");
    expect(wrapper.emitted("confirmImport")).toHaveLength(1);
  });
});
