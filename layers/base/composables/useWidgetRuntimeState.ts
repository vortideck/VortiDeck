import { shallowReactive } from "vue";
import type {
  WidgetProviderObservation,
  WidgetRequestedState,
  WidgetRuntimeError,
  WidgetRuntimeState,
} from "@base/types/deckWidget";

export interface WidgetStateSnapshot {
  protocolVersion: 1;
  instanceId: string;
  revision: number;
  status: WidgetRuntimeState["status"];
  values: Record<string, unknown>;
  providerStates?: Record<string, WidgetProviderObservation>;
  request?: WidgetRequestedState | null;
  error?: WidgetRuntimeError | null;
}

export interface WidgetStatePatch {
  protocolVersion: 1;
  instanceId: string;
  baseRevision: number;
  revision: number;
  status?: WidgetRuntimeState["status"];
  values?: Record<string, unknown>;
  remove?: string[];
  providerStates?: Record<string, WidgetProviderObservation | null>;
  request?: WidgetRequestedState | null;
  error?: WidgetRuntimeError | null;
}

const disconnectedState = (instanceId: string): WidgetRuntimeState => ({
  instanceId,
  revision: 0,
  status: "disconnected",
  values: {},
  providerStates: {},
});

export const useWidgetRuntimeState = () => {
  const states = shallowReactive(new Map<string, WidgetRuntimeState>());

  const get = (instanceId: string) => states.get(instanceId) ?? disconnectedState(instanceId);

  const applySnapshot = (snapshot: WidgetStateSnapshot) => {
    if (snapshot.protocolVersion !== 1 || snapshot.revision < 0) return false;
    states.set(snapshot.instanceId, {
      instanceId: snapshot.instanceId,
      revision: snapshot.revision,
      status: snapshot.status,
      values: { ...snapshot.values },
      providerStates: { ...(snapshot.providerStates ?? {}) },
      request: snapshot.request ?? undefined,
      error: snapshot.error ?? undefined,
    });
    return true;
  };

  const applyPatch = (patch: WidgetStatePatch) => {
    if (patch.protocolVersion !== 1 || patch.revision <= patch.baseRevision) return false;
    const current = states.get(patch.instanceId) ?? (
      patch.baseRevision === 0 ? disconnectedState(patch.instanceId) : undefined
    );
    if (!current || current.revision !== patch.baseRevision) return false;
    const values = { ...current.values, ...(patch.values ?? {}) };
    for (const key of patch.remove ?? []) delete values[key];
    const providerStates = { ...(current.providerStates ?? {}) };
    for (const [topic, observation] of Object.entries(patch.providerStates ?? {})) {
      if (observation === null) delete providerStates[topic];
      else providerStates[topic] = observation;
    }
    states.set(patch.instanceId, {
      ...current,
      revision: patch.revision,
      status: patch.status ?? current.status,
      values,
      providerStates,
      request: patch.request === null
        ? undefined
        : patch.request ?? current.request,
      error: patch.error === null
        ? undefined
        : patch.error ?? (patch.status === "error" ? current.error : undefined),
    });
    return true;
  };

  const markDisconnected = () => {
    for (const [instanceId, state] of states) {
      states.set(instanceId, {
        ...state,
        status: "disconnected",
        providerStates: Object.fromEntries(Object.entries(state.providerStates ?? {}).map(
          ([topic, observation]) => [topic, { ...observation, availability: "unavailable" as const }],
        )),
        request: undefined,
      });
    }
  };

  const publish = (
    instanceId: string,
    values: Record<string, unknown>,
    status: WidgetRuntimeState["status"] = "ready",
  ) => {
    const current = states.get(instanceId) ?? disconnectedState(instanceId);
    states.set(instanceId, {
      ...current,
      revision: current.revision + 1,
      status,
      values: { ...current.values, ...values },
      request: status === "executing" ? current.request : undefined,
      error: status === "error" ? current.error : undefined,
    });
  };

  const remove = (instanceId: string) => states.delete(instanceId);
  const clear = () => states.clear();

  return { states, get, applySnapshot, applyPatch, publish, markDisconnected, remove, clear };
};
