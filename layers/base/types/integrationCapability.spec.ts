import { describe, expect, it } from "vitest";
import type { CapabilityDefinition } from "./integrationCapability";
import { validateCapabilityValues, visibleCapabilityInputs } from "./integrationCapability";

const capability: CapabilityDefinition = {
  integrationId: "obs",
  capabilityId: "scene_source",
  version: 1,
  kind: "action",
  presentation: { name: "Scene source", description: "", category: "OBS" },
  allowAdditionalInputs: false,
  contract: {},
  outputs: [],
  inputs: [{
    id: "scene",
    label: "Scene",
    description: "",
    valueType: "string",
    required: true,
    widget: "select",
    options: [{
      label: "Main",
      value: "Main",
      parameters: [{
        id: "source",
        label: "Source",
        description: "",
        valueType: "string",
        required: true,
        widget: "select",
        options: [{ label: "Camera", value: "Camera" }],
      }],
    }],
  }],
};

describe("integration capability forms", () => {
  it("reveals only parameters belonging to the selected parent option", () => {
    expect(visibleCapabilityInputs(capability.inputs, {}).map((input) => input.id)).toEqual(["scene"]);
    expect(visibleCapabilityInputs(capability.inputs, { scene: "Main" }).map((input) => input.id)).toEqual(["scene", "source"]);
  });

  it("requires visible nested parameters", () => {
    expect(validateCapabilityValues(capability, { scene: "Main" })).toBe(false);
    expect(validateCapabilityValues(capability, { scene: "Main", source: "Camera" })).toBe(true);
  });
});
