import { beforeEach, describe, expect, it, vi } from "vitest";
import { usePluginExtensionsSandboxed } from "./usePluginExtensionsSandboxed";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

describe("sandboxed plugin web-extension loader", () => {
  beforeEach(() => invoke.mockReset());

  it("requests only the selected package and combines its extension payload", async () => {
    invoke.mockResolvedValue([
      {
        plugin_id: "text-processor-plugin",
        file_type: "css",
        content: ".plugin { color: blue; }",
      },
      {
        plugin_id: "text-processor-plugin",
        file_type: "js",
        content: "window.pluginLoaded = true;",
      },
    ]);

    const html = await usePluginExtensionsSandboxed().getPluginWebExtension(
      "text-processor-plugin",
    );

    expect(invoke).toHaveBeenCalledWith("get_plugin_web_extensions", {
      pluginId: "text-processor-plugin",
    });
    expect(html).toContain("<style>.plugin { color: blue; }</style>");
    expect(html).toContain("<script>window.pluginLoaded = true;</script>");
  });

  it("fails explicitly when the requested package has no extension payload", async () => {
    invoke.mockResolvedValue([]);

    await expect(
      usePluginExtensionsSandboxed().getPluginWebExtension("missing-package"),
    ).rejects.toThrow("No web extensions found for plugin: missing-package");
  });
});
