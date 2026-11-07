export type WidgetStreamIngressEvent = { type: string; [key: string]: unknown }

const CANONICAL_STREAM_TYPES: Record<string, string> = {
  chat_message: 'chatMessage',
  stream_online: 'streamOnline',
  stream_offline: 'streamOffline',
  channel_update: 'viewerCountUpdated',
}

/**
 * Normalize the relay-facing desktop envelope to the direct overlay WebSocket
 * payload. Package widgets therefore consume one protocol in the editor and OBS.
 */
export function normalizeWidgetStreamEvent(event: WidgetStreamIngressEvent): Record<string, unknown> {
  const nested = event.data && typeof event.data === 'object'
    ? event.data as Record<string, unknown>
    : {}

  return {
    ...nested,
    accountId: event.accountId ?? event.account_id,
    platform: event.platform,
    userName: nested.userName ?? nested.user_name,
    userDisplayName: nested.userDisplayName ?? nested.user_display_name ?? nested.userName ?? nested.user_name,
    platformColor: nested.platformColor ?? nested.platform_color,
    isModerator: nested.isModerator ?? nested.is_moderator,
    isSubscriber: nested.isSubscriber ?? nested.is_subscriber,
    viewerCount: nested.viewerCount ?? nested.viewer_count,
    type: CANONICAL_STREAM_TYPES[event.type] || event.type,
  }
}
