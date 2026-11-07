export const useFlowUtils = () => {
  // Helper functions for AI model display
  const getAiModelName = (modelId: string, availableModels: any[] = []) => {
    const model = availableModels.find((m) => m.id === modelId);
    return model ? model.name : modelId;
  };

  const getAiModelShortName = (modelId: string, availableModels: any[] = []) => {
    const model = availableModels.find((m) => m.id === modelId);
    if (!model) return modelId;

    // Create short names for common models
    switch (modelId) {
      case "gpt-3.5-turbo":
        return "GPT-3.5";
      case "gpt-4":
        return "GPT-4";
      case "gpt-4-turbo":
        return "GPT-4T";
      default:
        return model.name.length > 8
          ? model.name.substring(0, 8) + "..."
          : model.name;
    }
  };

  // Format log details
  const formatLogDetails = (details: any): string => {
    if (typeof details === "string") return details;
    return JSON.stringify(details, null, 2);
  };

  // Format result output
  const formatResultOutput = (output: any): string => {
    if (typeof output === "string") return output;
    return JSON.stringify(output, null, 2);
  };

  // Format timestamp
  const formatTimestamp = (timestamp: string): string => {
    const date = new Date(timestamp);
    return date.toLocaleTimeString([], {
      hour12: false,
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  };

  // Format category name for display - supports all dynamic categories
  const formatCategoryName = (category: string | number, customDisplayName?: string): string => {
    // If a custom display name is provided, use it
    if (customDisplayName) {
      return customDisplayName;
    }
    
    const categoryStr = category.toString().toUpperCase();
    
    // Map known categories to display names
    const categoryMap: Record<string, string> = {
      TRIGGER: "Start Triggers",
      SYSTEM: "System Tools", 
      INTEGRATION: "Integrations",
      STREAM: "Streaming Tools",
      AUDIO: "Audio Tools",
      UTILITY: "Utilities",
      UTILITIES: "Utilities", // Handle both singular/plural
      MACRO: "Macros & Automation",
      AI: "AI Tools",
      CONTROL: "Control Flow",
      PLUGINS: "Plugins",

      DATA: "Data Processing", 
      DATA_TRANSFORMATION: "Data Processing",
      WEB: "Web Tools",
      MEDIA: "Media Tools",
      DATABASE: "Database Tools",
      SECURITY: "Security Tools",
      CUSTOM: "Custom Tools",
      OTHER: "Other Tools"
    };

    // Return mapped name or format the category string nicely
    return categoryMap[categoryStr] || 
           categoryStr.split('_')
                     .map(word => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
                     .join(' ');
  };

  // Generic search filtering utility
  const createSearchFilter = (
    sourceData: any,
    searchTerm: string,
    searchFields: string[] = ["label", "name"]
  ) => {
    const normalizeArray = (items: any[]) =>
      (Array.isArray(items) ? items : []).filter(
        (item) => item && typeof item === "object"
      );

    if (!searchTerm.trim()) {
      if (Array.isArray(sourceData)) {
        return normalizeArray(sourceData);
      }

      const normalizedGrouped: Record<string, any[]> = {};
      Object.entries(sourceData || {}).forEach(([category, items]) => {
        const safeItems = normalizeArray(items as any[]);
        if (safeItems.length > 0) normalizedGrouped[category] = safeItems;
      });
      return normalizedGrouped;
    }
    const term = searchTerm.toLowerCase();

    if (Array.isArray(sourceData)) {
      return normalizeArray(sourceData).filter(
        (item) => {
          const description = typeof item.description === "string" ? item.description : "";
          return (
            searchFields.some(
              (field) =>
                typeof item[field] === "string" &&
                item[field].toLowerCase().includes(term)
            ) ||
            description.toLowerCase().includes(term)
          );
        }
      );
    }

    // For grouped tools (object with categories)
    const filtered: Record<string, any[]> = {};
    Object.entries(sourceData || {}).forEach(([category, items]) => {
      const filteredItems = normalizeArray(items as any[]).filter(
        (item) => {
          const description = typeof item.description === "string" ? item.description : "";
          return (
            searchFields.some(
              (field) =>
                typeof item[field] === "string" &&
                item[field].toLowerCase().includes(term)
            ) ||
            description.toLowerCase().includes(term)
          );
        }
      );
      if (filteredItems.length > 0) {
        filtered[category] = filteredItems;
      }
    });
    return filtered;
  };

  // Group items by category - still used by NodeLibrary for dynamic categorization
  const groupItemsByCategory = (itemsList: any[], categoryField: string = 'category', nameField: string = 'name'): Record<string, any[]> => {
    const grouped: Record<string, any[]> = {};

    itemsList.forEach((item) => {
      if (!item || typeof item !== "object") return;
      const category = item[categoryField]?.toString().toUpperCase() || "OTHER";

      if (!grouped[category]) {
        grouped[category] = [];
      }
      grouped[category].push(item);
    });

    return grouped;
  };

  return {
    formatCategoryName,
    groupItemsByCategory,
    createSearchFilter,
    getAiModelName,
    getAiModelShortName,
    formatTimestamp,
    formatLogDetails,
    formatResultOutput,
  };
};
