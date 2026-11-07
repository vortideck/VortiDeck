import type {
  DeckWidgetInstance,
  WidgetBinding,
  WidgetIntent,
} from "@base/types/deckWidget";

export type WidgetDispatchOperation =
  | {
      type: "execute";
      instance: DeckWidgetInstance;
      binding: WidgetBinding;
      value?: unknown;
      commit: boolean;
    }
  | {
      type: "repeat_start" | "repeat_stop";
      instance: DeckWidgetInstance;
      binding: WidgetBinding;
    };

export interface WidgetInteractionRouterOptions {
  getInstance: (instanceId: string) => DeckWidgetInstance | undefined;
  dispatch: (
    operation: WidgetDispatchOperation,
    intent: WidgetIntent,
  ) => void | Promise<void>;
  canInteract?: (instance: DeckWidgetInstance) => boolean;
  interactionSessionId?: string;
}

export const createWidgetInteractionSessionId = (): string =>
  globalThis.crypto?.randomUUID?.() ??
  `widget-${Date.now()}-${Math.random().toString(16).slice(2)}`;

const transformValue = (binding: WidgetBinding, input: unknown): unknown => {
  if (typeof input !== "number" || !binding.transform) return input;
  const transform = binding.transform;
  const inputRange = transform.inputMax - transform.inputMin;
  if (inputRange === 0) return transform.outputMin;
  const normalized = Math.min(
    1,
    Math.max(0, (input - transform.inputMin) / inputRange),
  );
  const curved =
    transform.curve === "logarithmic"
      ? Math.log10(1 + 9 * normalized)
      : normalized;
  return (
    transform.outputMin + curved * (transform.outputMax - transform.outputMin)
  );
};

export const useWidgetInteractionRouter = (
  options: WidgetInteractionRouterOptions,
) => {
  const interactionSessionId =
    options.interactionSessionId ?? createWidgetInteractionSessionId();
  const lastSequence = new Map<string, number>();
  const lastPreviewDispatch = new Map<string, number>();

  const route = async (intent: WidgetIntent): Promise<boolean> => {
    intent = { ...intent, interactionSessionId };
    const instance = options.getInstance(intent.instanceId);
    if (
      !instance ||
      !instance.enabled ||
      options.canInteract?.(instance) === false
    )
      return false;

    const binding = instance.bindings.find(
      (candidate) => candidate.portId === intent.portId,
    );
    if (!binding || binding.action == null) return false;

    // Every input source owns its sequence. A keyboard activation must not be
    // rejected merely because the pointer renderer has already emitted events.
    const sequenceKey = `${intent.instanceId}:${intent.portId}:${intent.inputSource}`;
    const previousSequence = lastSequence.get(sequenceKey) ?? -1;
    if (intent.sequence <= previousSequence) return false;
    lastSequence.set(sequenceKey, intent.sequence);

    if (binding.policy.type === "repeat") {
      if (intent.phase === "start") {
        await options.dispatch(
          { type: "repeat_start", instance, binding },
          intent,
        );
        return true;
      }
      if (intent.phase === "end" || intent.phase === "cancel") {
        await options.dispatch(
          { type: "repeat_stop", instance, binding },
          intent,
        );
        return true;
      }
      return false;
    }

    if (binding.policy.type === "continuous") {
      if (intent.phase === "preview") {
        const now = intent.timestamp;
        const previous = lastPreviewDispatch.get(sequenceKey) ?? 0;
        if (now - previous < binding.policy.throttleMs) return false;
        lastPreviewDispatch.set(sequenceKey, now);
      } else if (intent.phase !== "commit") {
        return false;
      }
      await options.dispatch(
        {
          type: "execute",
          instance,
          binding,
          value: transformValue(binding, intent.value),
          commit: intent.phase === "commit",
        },
        intent,
      );
      return true;
    }

    if (intent.phase !== "commit") return false;
    await options.dispatch(
      {
        type: "execute",
        instance,
        binding,
        value: transformValue(binding, intent.value),
        commit: true,
      },
      intent,
    );
    return true;
  };

  const reset = (instanceId?: string) => {
    if (!instanceId) {
      lastSequence.clear();
      lastPreviewDispatch.clear();
      return;
    }
    for (const key of lastSequence.keys()) {
      if (key.startsWith(`${instanceId}:`)) lastSequence.delete(key);
    }
    for (const key of lastPreviewDispatch.keys()) {
      if (key.startsWith(`${instanceId}:`)) lastPreviewDispatch.delete(key);
    }
  };

  return { route, reset, interactionSessionId };
};
