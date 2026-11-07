export const useSettingsModal = () => {
  const isOpen = useState<boolean>("settings_modal_open", () => false);
  const activeTab = useState<string>("settings_modal_active_tab", () => "appearance");

  const openSettings = (tab?: string) => {
    if (tab) {
      activeTab.value = tab;
    }
    isOpen.value = true;
  };

  const closeSettings = () => {
    isOpen.value = false;
  };

  const toggleSettings = () => {
    isOpen.value = !isOpen.value;
  };

  return {
    isOpen,
    activeTab,
    openSettings,
    closeSettings,
    toggleSettings,
  };
};
