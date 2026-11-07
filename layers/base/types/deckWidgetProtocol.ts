import type {
  DeckWidgetInstance,
  RemoteDeckWidgetInstance,
  PortableWidgetAssetRef,
  WidgetDefinition,
  WidgetIntent,
} from "@base/types/deckWidget";
import type {
  WidgetStatePatch,
  WidgetStateSnapshot,
} from "@base/composables/useWidgetRuntimeState";

export interface DeckWidgetSnapshotMessage {
  action: "deck_widget_snapshot";
  protocolVersion: 1;
  deckId: number;
  revision: number;
  layoutKey: string;
  definitions?: WidgetDefinition[];
  instances: RemoteDeckWidgetInstance[];
  state: WidgetStateSnapshot[];
}

export interface DeckWidgetDeltaMessage {
  action: "deck_widget_delta";
  protocolVersion: 1;
  deckId: number;
  baseRevision: number;
  revision: number;
  upsert: RemoteDeckWidgetInstance[];
  remove: string[];
  state: WidgetStatePatch[];
  definitions?: WidgetDefinition[];
  removeDefinitions?: string[];
}

export interface DeckWidgetIntentMessage {
  action: "deck_widget_intent";
  protocolVersion: 1;
  deckId: number;
  documentRevision: number;
  intent: WidgetIntent;
  correlationId: string;
}

export interface DeckWidgetStateMessage {
  action: "deck_widget_state";
  protocolVersion: 1;
  deckId: number;
  patches: WidgetStatePatch[];
}

export interface DeckWidgetIntentAckMessage {
  action: "deck_widget_intent_ack";
  protocolVersion: 1;
  deckId: number;
  instanceId: string;
  portId: string;
  sequence: number;
  interactionSessionId?: string;
  correlationId?: string;
  status: "accepted" | "scheduled" | "stopped" | "completed" | "rejected";
  code?: string;
  message?: string;
}

export interface DeckWidgetDefinitionsMessage {
  action: "deck_widget_definitions";
  protocolVersion: 1;
  definitions: WidgetDefinition[];
  removeDefinitions: string[];
}

export interface PortableWidgetAssetObject {
  asset: PortableWidgetAssetRef;
  /** Standard base64 without a data-URL prefix. */
  data: string;
}

export interface DeckWidgetCacheInventoryMessage {
  action: "deck_widget_cache_inventory";
  protocolVersion: 2;
  schemaVersions: number[];
  deckId: number;
  definitionHashes: string[];
  assetHashes: string[];
}

export interface DeckWidgetObjectsMessage {
  action: "deck_widget_objects";
  protocolVersion: 2;
  deckId: number;
  definitions: WidgetDefinition[];
  assets: PortableWidgetAssetObject[];
}

export interface DeckWidgetManifestMessage {
  action: "deck_widget_manifest";
  protocolVersion: 2;
  schemaVersions: number[];
  deckId: number;
  deckRevision: number;
  definitions: Array<{ definitionId: string; contentHash: string }>;
  assets: PortableWidgetAssetRef[];
}

export type DeckWidgetProtocolMessage =
  | DeckWidgetSnapshotMessage
  | DeckWidgetDeltaMessage
  | DeckWidgetIntentMessage
  | DeckWidgetStateMessage
  | DeckWidgetIntentAckMessage
  | DeckWidgetDefinitionsMessage
  | DeckWidgetCacheInventoryMessage
  | DeckWidgetObjectsMessage
  | DeckWidgetManifestMessage;

export const remoteWidgetToDeckInstance = (
  instance: RemoteDeckWidgetInstance,
): DeckWidgetInstance => ({
  ...instance,
  bindings: [],
});

export const applyDeckWidgetDelta = (
  instances: DeckWidgetInstance[],
  currentRevision: number,
  delta: DeckWidgetDeltaMessage,
) => {
  if (
    delta.protocolVersion !== 1 ||
    delta.baseRevision !== currentRevision ||
    delta.revision <= currentRevision
  )
    return undefined;
  const next = new Map(instances.map((instance) => [instance.id, instance]));
  for (const instanceId of delta.remove) next.delete(instanceId);
  for (const instance of delta.upsert)
    next.set(instance.id, remoteWidgetToDeckInstance(instance));
  return { revision: delta.revision, instances: Array.from(next.values()) };
};
