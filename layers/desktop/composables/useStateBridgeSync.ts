import { invoke } from "@tauri-apps/api/core";
import { useConnectionStore } from "../../base/stores/Connection";
import type { PeerInfo } from "../../base/types/Peer";
import { useDesktopUtils } from "../stores/desktopUtils";

interface VolatileStateSnapshot {
  activePeers: PeerInfo[];
  obsPluginActive: boolean;
}

let refreshInFlight: Promise<VolatileStateSnapshot> | null = null;

export function useStateBridgeSync() {
  const connectionStore = useConnectionStore();
  const desktopUtils = useDesktopUtils();

  const refreshVolatileState = () => {
    if (refreshInFlight) return refreshInFlight;

    refreshInFlight = Promise.all([
      invoke<PeerInfo[]>("retrieve_active_peers"),
      invoke<boolean>("get_obs_plugin_status"),
    ])
      .then(([activePeers, obsPluginActive]) => {
        connectionStore.setPairedActive(activePeers);
        desktopUtils.setIsObsPluginActive(obsPluginActive);
        return { activePeers, obsPluginActive };
      })
      .finally(() => {
        refreshInFlight = null;
      });

    return refreshInFlight;
  };

  return { refreshVolatileState };
}
