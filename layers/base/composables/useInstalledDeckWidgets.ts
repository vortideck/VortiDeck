import { invoke } from "@tauri-apps/api/core";
import { useWidgetRegistry } from "@base/composables/useWidgetRegistry";
import type { WidgetPackageDescriptor } from "@base/utils/pluginDeckWidget";

export const useInstalledDeckWidgets = () => {
  const registry = useWidgetRegistry();
  const loadedPackageIds = new Set<string>();

  const refresh = async () => {
    const packages = await invoke<WidgetPackageDescriptor[]>("list_packages");
    for (const packageId of loadedPackageIds) registry.unregisterPackage(packageId);
    loadedPackageIds.clear();
    let definitions = 0;
    for (const descriptor of packages) {
      const count = registry.registerPackage(descriptor);
      if (count > 0) loadedPackageIds.add(descriptor.id);
      definitions += count;
    }
    return definitions;
  };

  return { refresh };
};
