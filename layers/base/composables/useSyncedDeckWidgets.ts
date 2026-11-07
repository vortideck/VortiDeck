import { ref, shallowRef } from "vue";
import { useWidgetRuntimeState } from "@base/composables/useWidgetRuntimeState";
import { useWidgetRegistry } from "@base/composables/useWidgetRegistry";
import type { DeckWidgetInstance } from "@base/types/deckWidget";
import {
  applyDeckWidgetDelta,
  remoteWidgetToDeckInstance,
  type DeckWidgetDeltaMessage,
  type DeckWidgetSnapshotMessage,
  type DeckWidgetStateMessage,
} from "@base/types/deckWidgetProtocol";

const deckId = ref<number | null>(null);
const revision = ref(0);
const layoutKey = ref("default");
const instances = shallowRef<DeckWidgetInstance[]>([]);
const needsSnapshot = ref(false);
const runtime = useWidgetRuntimeState();
const registry = useWidgetRegistry();

export const useSyncedDeckWidgets = () => {
  const applySnapshot = (message: DeckWidgetSnapshotMessage) => {
    if (message.protocolVersion !== 1 || message.revision < 0) return false;
    deckId.value = message.deckId;
    revision.value = message.revision;
    layoutKey.value = message.layoutKey;
    registry.clearRemote();
    for (const definition of message.definitions ?? []) {
      registry.registerRemote(definition);
    }
    instances.value = message.instances.map(remoteWidgetToDeckInstance);
    runtime.clear();
    for (const snapshot of message.state) runtime.applySnapshot(snapshot);
    needsSnapshot.value = false;
    return true;
  };

  const applyDelta = (message: DeckWidgetDeltaMessage) => {
    if (deckId.value !== message.deckId) {
      needsSnapshot.value = true;
      return false;
    }
    const result = applyDeckWidgetDelta(instances.value, revision.value, message);
    if (!result) {
      needsSnapshot.value = true;
      return false;
    }
    revision.value = result.revision;
    instances.value = result.instances;
    for (const instanceId of message.remove) runtime.remove(instanceId);
    for (const instance of message.upsert) runtime.remove(instance.id);
    for (const definitionId of message.removeDefinitions ?? []) registry.unregisterRemote(definitionId);
    for (const definition of message.definitions ?? []) registry.registerRemote(definition);
    for (const patch of message.state) {
      if (!runtime.applyPatch(patch)) needsSnapshot.value = true;
    }
    return !needsSnapshot.value;
  };

  const reset = () => {
    deckId.value = null;
    revision.value = 0;
    instances.value = [];
    needsSnapshot.value = false;
    runtime.clear();
  };

  const applyState = (message: DeckWidgetStateMessage) => {
    if (message.protocolVersion !== 1 || deckId.value !== message.deckId) return false;
    let applied = true;
    for (const patch of message.patches) {
      if (!runtime.applyPatch(patch)) applied = false;
    }
    return applied;
  };

  const setLayoutKey = (key: "portrait" | "landscape" | "default") => {
    layoutKey.value = key;
  };

  return {
    deckId,
    revision,
    layoutKey,
    instances,
    needsSnapshot,
    runtime,
    applySnapshot,
    applyDelta,
    applyState,
    setLayoutKey,
    reset,
  };
};
