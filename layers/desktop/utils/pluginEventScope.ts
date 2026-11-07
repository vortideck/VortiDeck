export interface PluginEventScope {
  subscriptions: string[];
}

export interface ScopedPluginEvent {
  sourcePackageId?: string;
  eventType?: string;
  declaredEventType?: string;
  payload?: unknown;
}

export function permitsStreamEvent(
  scope: PluginEventScope,
  payload: unknown,
): boolean {
  if (!payload || typeof payload !== "object") return false;
  const eventType = (payload as Record<string, unknown>).type;
  return (
    typeof eventType === "string" && scope.subscriptions.includes(eventType)
  );
}

export function permitsPluginEvent(
  scope: PluginEventScope,
  sessionPackageId: string,
  event: ScopedPluginEvent,
): boolean {
  if (typeof event.eventType !== "string") return false;
  if (scope.subscriptions.includes(event.eventType)) return true;
  return (
    event.sourcePackageId === sessionPackageId &&
    typeof event.declaredEventType === "string" &&
    scope.subscriptions.includes(event.declaredEventType)
  );
}
