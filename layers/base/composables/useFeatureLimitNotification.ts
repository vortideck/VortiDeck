import { useMainStore } from "@base/stores/Main";

export const useFeatureLimitNotification = () => {
  const mainStore = useMainStore();

  const showFeatureLimitNotification = async (error: any, featureContext?: string) => {
    const errorMessage = error?.toString() || "An error occurred. Please try again.";
    
    // Detect if this is a feature limit error
    const isStorageLimitError = errorMessage.includes("Storage limit reached") || 
                               errorMessage.includes("storage limit") ||
                               errorMessage.includes("limit reached") ||
                               errorMessage.includes("Upgrade to Premium");
    
    const isUsageLimitError = errorMessage.includes("Usage limit") || 
                             errorMessage.includes("limit exceeded");
    
    const isFeatureLimitError = isStorageLimitError || isUsageLimitError;
    
    // Generate contextual messaging based on feature
    const getFeatureMessage = (context?: string) => {
      if (!isFeatureLimitError) return errorMessage;
      
      const baseMessage = "Upgrade to Premium for unlimited access and unlock all features!";
      
      switch (context?.toLowerCase()) {
        case 'macro':
        case 'macro_recording':
          return "You've reached your macro storage limit. " + baseMessage;
        case 'plugin':
        case 'plugin_count':
          return "You've reached your plugin limit. " + baseMessage;
        case 'deck':
        case 'saved_decks':
          return "You've reached your saved deck limit. " + baseMessage;
        case 'theme':
        case 'premium_themes':
          return "You've reached your theme limit. " + baseMessage;
        default:
          return "You've reached a usage limit. " + baseMessage;
      }
    };
    
    const openPlansPage = async () => {
      try {
        const { open } = await import('@tauri-apps/plugin-shell');
        await open('https://vortideck.com/plans');
      } catch (openError) {
        console.error("Failed to open plans page:", openError);
        // Fallback notification if browser fails to open
        mainStore.addNotification({
          title: "Visit VortiDeck Plans",
          message: "Please visit vortideck.com/plans to view our premium plans",
          type: "info",
          autoClose: true,
          duration: 5000,
          buttonText: "",
          buttonFunction: () => {},
        });
      }
    };

    // Show the appropriate notification
    mainStore.addNotification({
      title: isFeatureLimitError ? "Feature Limit Reached" : "Error",
      message: getFeatureMessage(featureContext),
      type: isFeatureLimitError ? "warning" : "error",
      autoClose: true,
      duration: isFeatureLimitError ? 12000 : 8000,
      buttonText: isFeatureLimitError ? "View Plans" : "",
      buttonFunction: isFeatureLimitError ? openPlansPage : () => {},
    });
  };

  const showSuccessNotification = (message: string, title: string = "Success") => {
    mainStore.addNotification({
      title,
      message,
      type: "success",
      autoClose: true,
      duration: 5000,
      buttonText: "",
      buttonFunction: () => {},
    });
  };

  return {
    showFeatureLimitNotification,
    showSuccessNotification,
  };
};
