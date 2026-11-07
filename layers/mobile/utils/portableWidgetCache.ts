import { sha256 } from "@noble/hashes/sha256";
import canonicalize from "canonicalize";
import { load } from "@tauri-apps/plugin-store";
import type { PortableWidgetAssetRef, WidgetDefinition } from "@base/types/deckWidget";
import type { PortableWidgetAssetObject } from "@base/types/deckWidgetProtocol";
import { validatePortableWidgetView } from "@base/utils/portableWidgetView";

const CACHE_FILE = "portable-widget-cache.json";
const CACHE_VERSION = 2;
const MAX_DEFINITIONS_PER_DESKTOP = 128;
const MAX_SERIALIZED_BYTES = 512 * 1024;
const MAX_ASSETS_PER_DESKTOP = 256;
const MAX_ASSET_CACHE_BYTES = 16 * 1024 * 1024;

interface CachedDefinition {
  definition: WidgetDefinition;
  storedAt: number;
}

interface DesktopDefinitionCache {
  version: 2;
  definitions: Record<string, CachedDefinition>;
  assets: Record<string, CachedAsset>;
}

interface CachedAsset {
  asset: PortableWidgetAssetRef;
  data: string;
  storedAt: number;
}

const bytesToHex = (bytes: Uint8Array) => Array.from(bytes)
  .map((byte) => byte.toString(16).padStart(2, "0"))
  .join("");

export const portableDefinitionHash = (definition: WidgetDefinition) => {
  const { contentHash: _contentHash, ...content } = definition;
  const serialized = canonicalize(content);
  return serialized ? bytesToHex(sha256(new TextEncoder().encode(serialized))) : undefined;
};

export const verifyPortableDefinition = (definition: WidgetDefinition) => {
  if (!definition.contentHash || portableDefinitionHash(definition) !== definition.contentHash) return false;
  return !definition.portableView
    || validatePortableWidgetView(definition.portableView, definition).valid;
};

const cacheKey = (desktopId: string) => `desktop:${desktopId}`;
const emptyCache = (): DesktopDefinitionCache => ({
  version: CACHE_VERSION,
  definitions: {},
  assets: {},
});

const decodeBase64 = (data: string) => {
  if (!/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(data)) return undefined;
  try {
    return Uint8Array.from(atob(data), (character) => character.charCodeAt(0));
  } catch {
    return undefined;
  }
};

export const verifyPortableAssetObject = ({ asset, data }: PortableWidgetAssetObject) => {
  if (!asset || !["image/png", "image/jpeg", "image/webp"].includes(asset.mediaType)) return false;
  if (!/^[a-f0-9]{64}$/.test(asset.sha256) || asset.byteLength <= 0 || asset.byteLength > 2 * 1024 * 1024) return false;
  const bytes = decodeBase64(data);
  if (!bytes || bytes.byteLength !== asset.byteLength
    || bytesToHex(sha256(bytes)) !== asset.sha256) return false;
  if (asset.mediaType === "image/png") {
    return [137, 80, 78, 71, 13, 10, 26, 10].every((byte, index) => bytes[index] === byte);
  }
  if (asset.mediaType === "image/jpeg") return bytes[0] === 0xff && bytes[1] === 0xd8;
  return bytes[0] === 0x52 && bytes[1] === 0x49 && bytes[2] === 0x46 && bytes[3] === 0x46
    && bytes[8] === 0x57 && bytes[9] === 0x45 && bytes[10] === 0x42 && bytes[11] === 0x50;
};

export const loadPortableDefinitions = async (desktopId: string) => {
  const store = await load(CACHE_FILE, { autoSave: false, defaults: {} });
  const cache = await store.get<DesktopDefinitionCache>(cacheKey(desktopId));
  if (!cache || cache.version !== CACHE_VERSION) return [];
  return Object.values(cache.definitions)
    .sort((left, right) => right.storedAt - left.storedAt)
    .map(({ definition }) => definition)
    .filter(verifyPortableDefinition);
};

export const cachePortableDefinitions = async (
  desktopId: string,
  definitions: WidgetDefinition[],
) => {
  const accepted = definitions.filter(verifyPortableDefinition);
  if (!accepted.length) return;
  const store = await load(CACHE_FILE, { autoSave: false, defaults: {} });
  const existing = await store.get<DesktopDefinitionCache>(cacheKey(desktopId));
  const cache = existing?.version === CACHE_VERSION ? existing : emptyCache();
  const records = cache.definitions;
  const now = Date.now();
  for (const definition of accepted) {
    for (const [hash, entry] of Object.entries(records)) {
      if (entry.definition.id === definition.id && hash !== definition.contentHash) delete records[hash];
    }
    records[definition.contentHash!] = { definition, storedAt: now };
  }
  const bounded = Object.fromEntries(Object.entries(records)
    .sort(([, left], [, right]) => right.storedAt - left.storedAt)
    .slice(0, MAX_DEFINITIONS_PER_DESKTOP));
  while (JSON.stringify(bounded).length > MAX_SERIALIZED_BYTES) {
    const oldest = Object.keys(bounded).at(-1);
    if (!oldest) break;
    delete bounded[oldest];
  }
  const reachableAssetHashes = new Set(Object.values(bounded)
    .flatMap(({ definition }) => definition.portableView?.assets ?? [])
    .map(({ sha256 }) => sha256));
  const assets = Object.fromEntries(Object.entries(cache.assets)
    .filter(([hash]) => reachableAssetHashes.has(hash)));
  await store.set(cacheKey(desktopId), { ...cache, definitions: bounded, assets });
  await store.save();
};

export const loadPortableAssets = async (desktopId: string) => {
  const store = await load(CACHE_FILE, { autoSave: false, defaults: {} });
  const cache = await store.get<DesktopDefinitionCache>(cacheKey(desktopId));
  if (!cache || cache.version !== CACHE_VERSION) return [];
  return Object.values(cache.assets)
    .sort((left, right) => right.storedAt - left.storedAt)
    .filter(verifyPortableAssetObject);
};

export const cachePortableAssets = async (
  desktopId: string,
  assets: PortableWidgetAssetObject[],
) => {
  const accepted = assets.filter(verifyPortableAssetObject);
  if (!accepted.length) return;
  const store = await load(CACHE_FILE, { autoSave: false, defaults: {} });
  const existing = await store.get<DesktopDefinitionCache>(cacheKey(desktopId));
  const cache = existing?.version === CACHE_VERSION ? existing : emptyCache();
  const now = Date.now();
  for (const object of accepted) {
    cache.assets[object.asset.sha256] = { ...object, storedAt: now };
  }
  const boundedEntries = Object.entries(cache.assets)
    .sort(([, left], [, right]) => right.storedAt - left.storedAt)
    .slice(0, MAX_ASSETS_PER_DESKTOP);
  let byteTotal = 0;
  const boundedAssets: DesktopDefinitionCache["assets"] = {};
  for (const [hash, entry] of boundedEntries) {
    if (byteTotal + entry.asset.byteLength > MAX_ASSET_CACHE_BYTES) continue;
    byteTotal += entry.asset.byteLength;
    boundedAssets[hash] = entry;
  }
  await store.set(cacheKey(desktopId), { ...cache, assets: boundedAssets });
  await store.save();
};

export const portableCacheInventory = async (desktopId: string) => {
  const [definitions, assets] = await Promise.all([
    loadPortableDefinitions(desktopId),
    loadPortableAssets(desktopId),
  ]);
  return {
    definitionHashes: definitions.flatMap(({ contentHash }) => contentHash ? [contentHash] : []),
    assetHashes: assets.map(({ asset }) => asset.sha256),
  };
};

export const purgePortableDefinitionCache = async (desktopId: string) => {
  const store = await load(CACHE_FILE, { autoSave: false, defaults: {} });
  await store.delete(cacheKey(desktopId));
  await store.save();
};

export const removePortableDefinitions = async (
  desktopId: string,
  definitionIds: string[],
) => {
  if (!definitionIds.length) return;
  const store = await load(CACHE_FILE, { autoSave: false, defaults: {} });
  const key = cacheKey(desktopId);
  const cache = await store.get<DesktopDefinitionCache>(key);
  if (!cache || cache.version !== CACHE_VERSION) return;
  const ids = new Set(definitionIds);
  const definitions = Object.fromEntries(Object.entries(cache.definitions)
    .filter(([, entry]) => !ids.has(entry.definition.id)));
  const reachableAssetHashes = new Set(Object.values(definitions)
    .flatMap(({ definition }) => definition.portableView?.assets ?? [])
    .map(({ sha256 }) => sha256));
  const assets = Object.fromEntries(Object.entries(cache.assets)
    .filter(([hash]) => reachableAssetHashes.has(hash)));
  await store.set(key, { ...cache, definitions, assets });
  await store.save();
};
