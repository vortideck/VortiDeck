/**
 * Rust-based streaming composable
 *
 * This replaces the legacy TypeScript WebSocket implementation with calls
 * to the Rust backend. All WebSocket connection, message storage, and
 * session management is now handled in Rust for persistence across page reloads.
 */

import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useAuthStore } from '@/stores/Auth';

// ============================================================================
// Type Definitions
// ============================================================================

export interface UnifiedChatMessage {
  id: string;
  account_id: number;
  platform: 'twitch' | 'youtube' | 'kick' | 'tiktok' | 'discord';
  user_id: string;
  user_name: string;
  user_display_name: string;
  message: string;
  timestamp: string;
  badges: string[];
  is_moderator: boolean;
  is_subscriber: boolean;
  is_vip: boolean;
  platform_color: string;
  emotes?: Emote[];
  mentions?: string[];
}

export interface Emote {
  id: string;
  name: string;
  url: string;
}

export interface StreamData {
  stream_id: string;
  title: string;
  started_at: string;
  viewer_count?: number;
  game_name?: string;
  stream_type?: string;
}

export interface ActiveStream {
  account_id: number;
  platform: string;
  stream_id: string;
  title: string;
  started_at: string;
  game_name?: string;
  viewer_count?: number;
  stream_type?: string;
}

export interface ChannelUpdateData {
  title?: string;
  viewer_count?: number;
  game_name?: string;
}

export type CloudEvent =
  | { type: 'connection_established'; streamer_id: string; display_name: string; timestamp: string; active_streams?: ActiveStream[] }
  | { type: 'chat_message'; account_id: number; platform: string; data: UnifiedChatMessage }
  | { type: 'stream_online'; account_id: number; platform: string; data: StreamData }
  | { type: 'stream_offline'; account_id: number; platform: string }
  | { type: 'channel_update'; account_id: number; platform: string; data: ChannelUpdateData }
  | { type: 'subscription'; account_id: number; platform: string; data: any }
  | { type: 'raid'; account_id: number; platform: string; data: any }
  | { type: 'follow'; account_id: number; platform: string; data: any }
  | { type: 'bits'; account_id: number; platform: string; data: any }
  | { type: 'platform_status'; platform: string; status: string; message?: string }
  | { type: 'send_chat_result'; success: boolean; platforms_sent?: string[]; errors?: any[] }
  | { type: 'error'; message: string; status?: string };

export function useStreamingRust() {
  const config = useRuntimeConfig();

  // Connection state
  const isConnected = ref(false);
  const connectionStatus = ref<'connecting' | 'connected' | 'disconnected' | 'error'>('disconnected');
  const lastError = ref<string | null>(null);

  // Event listeners
  let eventUnlisten: UnlistenFn | null = null;
  const messageCallbacks = new Set<(message: UnifiedChatMessage) => void>();
  const eventCallbacks = new Set<(event: CloudEvent) => void>();
  const streamStatusCallbacks = new Set<(event: CloudEvent) => void>();
  const sessionUpdateCallbacks = new Set<() => void>();

  // Set up event listener immediately (backend may already be connected via auto-connect)
  // This is called once when the composable is first created
  if (!eventUnlisten) {
    listen<CloudEvent>('streaming:event', (event) => {
      console.log('🎧 [LISTEN] Received event from Rust:', event.payload);
      handleCloudEvent(event.payload);
    }).then(unlisten => {
      eventUnlisten = unlisten;
      console.log('✅ [RUST] Event listener registered for streaming:event');
    }).catch(err => {
      console.error('❌ [RUST] Failed to register event listener:', err);
    });
  }

  /**
   * Connect to streaming service (Rust backend)
   */
  async function connect() {
    if (isConnected.value) {
      console.log('✅ Already connected to streaming service');
      return;
    }

    connectionStatus.value = 'connecting';
    lastError.value = null;

    try {
      // Build WebSocket URL
      const wsUrl = String(config.public.WS_URL ?? "");
      const streamingWsUrl = wsUrl.replace(/\/+$/, "") + "/ws/streaming/";

      // Get access token (you may need to get this from auth store)
      const authStore = useAuthStore();
      const accessToken = authStore.getAccessToken?.token || null;

      // Connect via Rust backend
      console.log('📞 [RUST] Calling invoke streaming_connect with:', { url: wsUrl, hasToken: !!accessToken });
      const result = await invoke('streaming_connect', {
        url: wsUrl,
        accessToken,
      });
      console.log('📞 [RUST] invoke streaming_connect returned:', result);

      isConnected.value = true;
      connectionStatus.value = 'connected';
      console.log('✅ Connected to streaming service (Rust backend)');
    } catch (error) {
      console.error('❌ Failed to connect:', error);
      connectionStatus.value = 'error';
      lastError.value = error instanceof Error ? error.message : String(error);
      isConnected.value = false;
    }
  }

  /**
   * Disconnect from streaming service
   */
  async function disconnect() {
    try {
      // Stop listening to events
      if (eventUnlisten) {
        eventUnlisten();
        eventUnlisten = null;
      }

      // Disconnect Rust WebSocket
      await invoke('streaming_disconnect');

      isConnected.value = false;
      connectionStatus.value = 'disconnected';
      console.log('✅ Disconnected from streaming service');
    } catch (error) {
      console.error('❌ Error disconnecting:', error);
    }
  }

  /**
   * Send chat message to platforms
   */
  type StreamingInvocationResult = {
    invocationId: string;
    status: "applied" | "transport_accepted" | "partial" | "failed" | "timed_out" | "uncertain" | "skipped";
    outputs: Record<string, unknown>;
    error?: { message?: string } | null;
  };

  const invokeStreamingAction = async (
    command: string,
    args: Record<string, unknown>,
  ): Promise<StreamingInvocationResult> => {
    const result = await invoke<StreamingInvocationResult>(command, args);
    if (["failed", "timed_out", "uncertain"].includes(result.status)) {
      throw new Error(result.error?.message || `Streaming action ended as ${result.status}`);
    }
    return result;
  };

  async function sendChatMessage(accountIds: (string | number)[], message: string, replyTo?: string) {
    try {
      // Convert account IDs to numbers
      const numericIds = accountIds.map(id => typeof id === 'string' ? parseInt(id) : id);

      await invokeStreamingAction('streaming_send_chat_message', {
        accountIds: numericIds,
        message,
        replyTo: replyTo || null,
      });
    } catch (error) {
      console.error('❌ Failed to send chat message:', error);
      throw error;
    }
  }

  /**
   * Ban user on platform
   */
  async function banUser(platform: string, userId: string, reason?: string) {
    try {
      await invokeStreamingAction('streaming_ban_user', {
        platform,
        userId,
        reason: reason || null,
      });
    } catch (error) {
      console.error('❌ Failed to ban user:', error);
      throw error;
    }
  }

  /**
   * Timeout user on platform
   */
  async function timeoutUser(platform: string, userId: string, durationSeconds: number, reason?: string) {
    try {
      await invokeStreamingAction('streaming_timeout_user', {
        platform,
        userId,
        durationSeconds,
        reason: reason || null,
      });
    } catch (error) {
      console.error('❌ Failed to timeout user:', error);
      throw error;
    }
  }

  /**
   * Delete message on platform
   */
  async function deleteMessage(platform: string, messageId: string) {
    try {
      await invokeStreamingAction('streaming_delete_message', {
        platform,
        messageId,
      });
    } catch (error) {
      console.error('❌ Failed to delete message:', error);
      throw error;
    }
  }

  /**
   * Get messages for a specific account (from Rust persistent storage)
   */
  async function getMessagesForAccount(accountId: number, limit?: number): Promise<UnifiedChatMessage[]> {
    try {
      const messages = await invoke<UnifiedChatMessage[]>('streaming_get_messages', {
        accountId,
        limit: limit || null,
      });
      return messages;
    } catch (error) {
      console.error('❌ Failed to get messages:', error);
      return [];
    }
  }

  /**
   * Get messages for multiple accounts (aggregated)
   */
  async function getMessagesForAccounts(accountIds: number[], limit?: number): Promise<UnifiedChatMessage[]> {
    try {
      const messages = await invoke<UnifiedChatMessage[]>('streaming_get_messages_for_accounts', {
        accountIds,
        limit: limit || null,
      });
      return messages;
    } catch (error) {
      console.error('❌ Failed to get messages:', error);
      return [];
    }
  }

  /**
   * Register callback for chat messages
   */
  function onMessage(callback: (message: UnifiedChatMessage) => void) {
    messageCallbacks.add(callback);
    return () => messageCallbacks.delete(callback);
  }

  /**
   * Register callback for all events
   */
  function onEvent(callback: (event: CloudEvent) => void) {
    eventCallbacks.add(callback);
    return () => eventCallbacks.delete(callback);
  }

  /**
   * Register callback for stream status events
   */
  function onStreamStatus(callback: (event: CloudEvent) => void) {
    streamStatusCallbacks.add(callback);
    return () => streamStatusCallbacks.delete(callback);
  }

  /**
   * Register callback for session updates (when Rust updates a session)
   */
  function onSessionUpdate(callback: () => void) {
    sessionUpdateCallbacks.add(callback);
    return () => sessionUpdateCallbacks.delete(callback);
  }

  /**
   * Handle CloudEvent from Rust backend
   */
  function handleCloudEvent(event: CloudEvent) {
    console.log('🔄 [HANDLE] Processing CloudEvent:', event.type);

    // Notify all event callbacks
    eventCallbacks.forEach(cb => cb(event));

    // Handle specific event types
    switch (event.type) {
      case 'connection_established':
        console.log('✅ [HANDLE] Connection established:', event.display_name);

        // Process initial active streams
        if (event.active_streams && event.active_streams.length > 0) {
          console.log(`🔄 Processing ${event.active_streams.length} active streams`);
          event.active_streams.forEach(stream => {
            const streamOnlineEvent: CloudEvent = {
              type: 'stream_online',
              account_id: stream.account_id,
              platform: stream.platform,
              data: {
                stream_id: stream.stream_id,
                title: stream.title,
                started_at: stream.started_at,
                viewer_count: stream.viewer_count,
                game_name: stream.game_name,
                stream_type: stream.stream_type,
              }
            };
            streamStatusCallbacks.forEach(cb => cb(streamOnlineEvent));
          });
        }
        break;

      case 'stream_online':
        console.log('🎬 [RUST STREAMING] Stream went live:', event.platform, event.account_id);
        // Rust auto-activates the session, so notify UI to refresh
        sessionUpdateCallbacks.forEach(cb => cb());
        streamStatusCallbacks.forEach(cb => cb(event));
        break;

      case 'stream_offline':
        console.log('🔴 [RUST STREAMING] Stream went offline:', event.platform, event.account_id);
        // Rust auto-ends the session, so notify UI to refresh
        sessionUpdateCallbacks.forEach(cb => cb());
        streamStatusCallbacks.forEach(cb => cb(event));
        break;

      case 'channel_update':
        streamStatusCallbacks.forEach(cb => cb(event));
        break;

      case 'chat_message':
        console.log('💬 [HANDLE] Chat message event received:', event);
        if (event.data && event.account_id) {
          const message: UnifiedChatMessage = {
            ...event.data,
            account_id: event.account_id,
            platform: event.platform || event.data.platform
          };
          console.log('📤 [HANDLE] Notifying', messageCallbacks.size, 'message callbacks');
          // Notify message callbacks
          messageCallbacks.forEach(cb => {
            console.log('📞 [HANDLE] Calling message callback');
            cb(message);
          });
        } else {
          console.warn('⚠️ [HANDLE] Chat message missing data or account_id:', event);
        }
        break;

      case 'send_chat_result':
        console.log('📤 Chat sent:', event);
        break;

      case 'error':
        console.error('❌ Backend error:', event.message);
        lastError.value = event.message || 'Unknown error';
        break;

      default:
        console.log('📨 Event:', event.type, event);
    }
  }

  // Cleanup on unmount (optional - WebSocket persists in Rust)
  // onBeforeUnmount(() => {
  //   if (eventUnlisten) {
  //     eventUnlisten();
  //   }
  // });

  return {
    // State
    isConnected,
    connectionStatus,
    lastError,

    // Connection methods
    connect,
    disconnect,

    // Send methods
    sendChatMessage,
    banUser,
    timeoutUser,
    deleteMessage,

    // Message retrieval (from Rust persistent storage)
    getMessagesForAccount,
    getMessagesForAccounts,

    // Event listeners
    onMessage,
    onEvent,
    onStreamStatus,
    onSessionUpdate,
  };
}
