import { describe, expect, it } from "vitest";
import { buildNodeCatalog, canonicalNodeCategory, validateNodeCatalog } from "./nodeCatalog";

describe("node catalog", () => {
  it("merges historical category aliases into one visible category", () => {
    const catalog = buildNodeCatalog([
      { name: "delay", label: "Delay", category: "CONTROL", subcategory: "Timing" },
      { name: "retry", label: "Retry", category: "CONTROL_FLOW", subcategory: "Error Handling" },
    ]);

    expect(canonicalNodeCategory("CONTROL_FLOW")).toBe("CONTROL");
    expect(catalog).toHaveLength(1);
    expect(catalog[0]?.label).toBe("Control Flow");
    expect(catalog[0]?.groups.map((group) => group.label)).toEqual(["Error Handling", "Timing"]);
  });

  it("reports duplicate IDs and conflicting category labels", () => {
    expect(validateNodeCatalog([
      { name: "same", label: "One", category: "ACTION", category_display_name: "Actions" },
      { name: "same", label: "Two", category: "ACTION", category_display_name: "Operations" },
    ])).toEqual([
      "Duplicate node ID: same",
      "Category ACTION has conflicting labels: Actions, Operations",
    ]);
  });
});
