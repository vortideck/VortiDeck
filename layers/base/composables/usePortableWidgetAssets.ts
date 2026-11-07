import type { PortableWidgetAssetObject } from "@base/types/deckWidgetProtocol";
import type { WidgetDefinition } from "@base/types/deckWidget";
import { shallowReactive } from "vue";

const assetUrls = shallowReactive(new Map<string, string>());

export const usePortableWidgetAssets = () => {
  const register = (objects: PortableWidgetAssetObject[]) => {
    for (const { asset, data } of objects) {
      assetUrls.set(asset.sha256, `data:${asset.mediaType};base64,${data}`);
    }
  };

  const clear = () => assetUrls.clear();

  const resolve = (definition: WidgetDefinition, assetId: string) => {
    const reference = definition.portableView?.assets?.find(({ id }) => id === assetId);
    return reference ? assetUrls.get(reference.sha256) : undefined;
  };

  return { register, clear, resolve };
};
