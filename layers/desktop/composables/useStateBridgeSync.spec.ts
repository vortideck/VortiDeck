import { beforeEach, describe, expect, it, vi } from "vitest";
import { useStateBridgeSync } from "./useStateBridgeSync";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  setPairedActive: vi.fn(),
  setIsObsPluginActive: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("../../base/stores/Connection", () => ({
  useConnectionStore: () => ({ setPairedActive: mocks.setPairedActive }),
}));
vi.mock("../stores/desktopUtils", () => ({
  useDesktopUtils: () => ({ setIsObsPluginActive: mocks.setIsObsPluginActive }),
}));

describe("state bridge snapshot synchronization", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("replaces volatile UI state with authoritative peer and OBS snapshots", async () => {
    const activePeers = [{
      client_id: "mobile-1",
      device_type: "mobile",
      device: "phone",
    }];
    mocks.invoke.mockImplementation((command: string) => {
      if (command === "retrieve_active_peers") return Promise.resolve(activePeers);
      if (command === "get_obs_plugin_status") return Promise.resolve(true);
      return Promise.reject(new Error(`unexpected command: ${command}`));
    });

    const { refreshVolatileState } = useStateBridgeSync();
    const firstRefresh = refreshVolatileState();
    const coalescedRefresh = refreshVolatileState();

    expect(coalescedRefresh).toBe(firstRefresh);
    await firstRefresh;

    expect(mocks.invoke).toHaveBeenCalledTimes(2);
    expect(mocks.setPairedActive).toHaveBeenCalledWith(activePeers);
    expect(mocks.setIsObsPluginActive).toHaveBeenCalledWith(true);
  });
});
