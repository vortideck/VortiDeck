import { describe, expect, it } from "vitest";
import { validateWidgetConfig } from "@base/utils/widgetConfigSchema";

describe("widget config schema", () => {
  it("applies defaults and validates required values and ranges", () => {
    expect(validateWidgetConfig({
      type: "object",
      required: ["label"],
      properties: {
        label: { type: "string", default: "Volume" },
        max: { type: "number", minimum: 1, maximum: 100, default: 100 },
      },
    }, {})).toEqual({
      valid: true,
      value: { label: "Volume", max: 100 },
      errors: [],
    });
    expect(validateWidgetConfig({
      type: "object",
      properties: { max: { type: "number", maximum: 100 } },
    }, { max: 101 }).valid).toBe(false);
  });
});
