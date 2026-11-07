import { listen } from "@tauri-apps/api/event";

export const useNetworkStatus = () => {
  const isOnline = useState<boolean>("is_network_online", () => true);

  if (import.meta.client) {
    listen<boolean>("internet_status", (event) => {
      isOnline.value = Boolean(event.payload);
    }).catch(() => {});
  }

  return {
    isOnline,
  };
};
