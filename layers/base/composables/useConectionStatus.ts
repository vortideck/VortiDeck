import { useConnectionStore } from "@base/stores/Connection";

export const useConnectionStatus = computed(() => {
  const connectionStore = useConnectionStore();

  return [
    connectionStore?.getStatus === "paired"
      ? "bg-themePairedBg text-themePaired"
      : "",
    connectionStore?.getStatus === "connecting"
      ? "bg-themeConnectingBg text-themeConnecting"
      : "",

    connectionStore?.getStatus === "disconnected"
      ? "bg-themeDisconnectedBg text-themeDisconnected"
      : "",
    connectionStore?.getStatus === "connected"
      ? "bg-themeConnectedBg text-themeConnected"
      : "",
  ];
});
