/**
 * Session Manager Composable - Rust Backend Wrapper
 *
 * This is a thin wrapper around Rust session management commands.
 * All session state is managed in Rust for persistence and performance.
 */

import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { emit, listen } from '@tauri-apps/api/event';

/**
 * Session lifecycle states (reusable sessions model)
 */
export type SessionStatus =
  | 'draft'              // Template created, not activated
  | 'waiting_for_live'   // Activated, waiting for stream to go live
  | 'active'             // Stream is live
  | 'paused'             // Stream temporarily paused
  | 'inactive';          // Ended, can be reactivated

/**
 * Historical record of a single stream run (camelCase for Rust compatibility)
 */
export interface StreamRun {
  startedAt: string;
  endedAt: string;
  duration: number;              // Duration in ms
  messageCount: number;
  commandExecutions: number;
  endedBy: 'user' | 'auto';     // How it ended
}

/**
 * Platform stream data from backend (camelCase for Rust compatibility)
 */
export interface PlatformStream {
  platform: 'twitch' | 'youtube' | 'kick' | 'tiktok';
  accountId: string;             // Always string for consistency
  streamId?: string;             // Present when stream is live
  title?: string;                 // Stream title
  startedAt?: string;            // When stream went live
  viewerCount?: number;          // Current viewer count
}

/**
 * Session command configuration (camelCase for Rust compatibility)
 */
export interface SessionCommand {
  id?: string;
  permissionVersion?: number;
  action: string;
  allowedRoles?: Record<string, string[]>;
  userWhitelist?: string[];
  userBlacklist?: string[];
  inputBindings?: Record<string, import('@base/types/actionInputs').CommandInputSource>;
  trigger: string;        // Chat command trigger (e.g., "!alert")
  deckActionId?: string;   // Legacy imported deck reference
  cooldown?: number;      // Cooldown in seconds
  enabled: boolean;       // Whether command is active
}

/**
 * Session settings (camelCase - Tauri converts to snake_case for Rust)
 */
export interface SessionSettings {
  autoStart: boolean;         // Auto-activate when stream goes live
  notifications: boolean;      // Show notifications for session events
  autoResume: boolean;        // Auto-resume if stream restarts within grace period
  gracePeriodMinutes?: number; // Customizable grace period (default: 30 minutes)
  priority?: number;           // Session priority for auto-resume (higher = first, default: 0)
}

/**
 * Stream session template (reusable) - camelCase for Rust compatibility
 */
export interface StreamSession {
  id: string;
  name: string;
  description?: string;

  // Platform configuration (what user configured)
  platforms: PlatformStream[];            // Platform + account pairs configured for this session

  commands: SessionCommand[];
  settings: SessionSettings;
  status: SessionStatus;

  // Metadata
  createdAt: string;
  updatedAt: string;

  // Current run data (resets on each activation)
  currentRunStartedAt?: string;        // When current run started
  pausedAt?: string;                     // When session was paused

  // Current run stats (resets on each end)
  duration?: number;                      // Current run duration in ms
  messageCount?: number;                 // Current run messages
  commandExecutions?: number;            // Current run commands

  // Historical data (persists across runs)
  runs: StreamRun[];                      // History of all stream runs
  totalRuns: number;                     // Total number of times activated
  lastUsedAt?: string;                  // Last time session was activated
  totalDuration?: number;                // Aggregate duration across all runs
  totalMessages?: number;                // Aggregate messages across all runs
  totalCommands?: number;                // Aggregate commands across all runs
}

// Shared global state for sessions
const sessions = ref<StreamSession[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);

// Register window-level listener for session updates (handles multi-window Tauri sync)
if (typeof window !== 'undefined') {
  listen('session_event', async (event) => {
    console.log('🔄 [SESSION MANAGER] Session event received:', event.payload, 'reloading sessions in this window...');
    try {
      const allSessions = await invoke<StreamSession[]>('streaming_get_all_sessions');
      sessions.value = allSessions.sort((a, b) =>
        new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime()
      );
    } catch (err) {
      console.error('❌ [SESSION MANAGER] Failed to reload sessions on event:', err);
    }
  });
}

/**
 * Session Manager Composable - Calls Rust backend
 */
export const useSessionManager = () => {

  /**
   * Load all sessions from Rust
   */
  const loadSessions = async () => {
    loading.value = true;
    error.value = null;

    try {
      const allSessions = await invoke<StreamSession[]>('streaming_get_all_sessions');
      sessions.value = allSessions.sort((a, b) =>
        new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime()
      );

      console.log(`📊 [SESSION MANAGER] Loaded ${sessions.value.length} sessions from Rust`);
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to load sessions';
      console.error('❌ [SESSION MANAGER] Failed to load sessions:', err);
    } finally {
      loading.value = false;
    }
  };

  /**
   * Create a new session
   */
  const createSession = async (sessionData: Partial<StreamSession>): Promise<StreamSession> => {
    loading.value = true;
    error.value = null;

    try {
      // Normalize the caller input to the canonical camelCase wire contract.
      const normalizedPlatforms = (sessionData.platforms || []).map(p => ({
        platform: p.platform,
        accountId: String(p.accountId),
        streamId: p.streamId,
        title: p.title,
        startedAt: p.startedAt,
        viewerCount: p.viewerCount,
      }));

      const newSession = {
        id: '', // Rust will generate ID
        name: sessionData.name || 'Untitled Session',
        description: sessionData.description,
        platforms: normalizedPlatforms,
        commands: sessionData.commands || [],
        settings: sessionData.settings || {
          autoStart: true,
          notifications: true,
          autoResume: true,
        },
        status: 'draft',
        createdAt: '',  // Rust will set
        updatedAt: '',  // Rust will set
        runs: [],
        totalRuns: 0,
      };

      console.log('🔧 [SESSION MANAGER] Creating session:', newSession);

      const created = await invoke<StreamSession>('streaming_create_session', { session: newSession });

      // Add to local cache
      sessions.value.unshift(created);

      console.log('✅ [SESSION MANAGER] Created session:', created.id);
      return created;
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to create session';
      console.error('❌ [SESSION MANAGER] Failed to create session:', err);
      throw err;
    } finally {
      loading.value = false;
    }
  };

  /**
   * Update existing session
   */
  const updateSession = async (sessionId: string, updates: Partial<StreamSession>): Promise<StreamSession> => {
    loading.value = true;
    error.value = null;

    try {
      // Convert to Rust-compatible updates map
      const updatesMap: Record<string, any> = {};
      Object.entries(updates).forEach(([key, value]) => {
        if (value !== undefined) {
          updatesMap[key] = value;
        }
      });

      const updated = await invoke<StreamSession>('streaming_update_session', {
        sessionId,
        updates: updatesMap,
      });

      // Update local cache
      const index = sessions.value.findIndex(s => s.id === sessionId);
      if (index !== -1) {
        sessions.value[index] = updated;
      }

      console.log('✅ [SESSION MANAGER] Updated session:', sessionId);
      return updated;
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to update session';
      console.error('❌ [SESSION MANAGER] Failed to update session:', err);
      throw err;
    } finally {
      loading.value = false;
    }
  };

  /**
   * Delete session
   */
  const deleteSession = async (sessionId: string): Promise<void> => {
    loading.value = true;
    error.value = null;

    try {
      await invoke('streaming_delete_session', { sessionId });

      // Remove from local cache
      sessions.value = sessions.value.filter(s => s.id !== sessionId);

      console.log('✅ [SESSION MANAGER] Deleted session:', sessionId);
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to delete session';
      console.error('❌ [SESSION MANAGER] Failed to delete session:', err);
      throw err;
    } finally {
      loading.value = false;
    }
  };

  /**
   * Activate session (move to waiting_for_live state)
   */
  const activateSession = async (sessionId: string): Promise<void> => {
    const session = sessions.value.find(s => s.id === sessionId);
    if (!session) {
      throw new Error(`Session ${sessionId} not found`);
    }

    // Check for conflicts via Rust
    const conflicts = await invoke<StreamSession[]>('streaming_check_conflicts', {
      platforms: session.platforms,
    });

    if (conflicts.length > 0) {
      throw new Error(`Conflict: Account already has an active session`);
    }

    const updated = await invoke<StreamSession>('streaming_activate_session', { sessionId });

    // Update local cache
    const index = sessions.value.findIndex(s => s.id === sessionId);
    if (index !== -1) {
      sessions.value[index] = updated;
    }

    console.log('🎬 [SESSION MANAGER] Session activated:', sessionId);
  };

  /**
   * Start session (move to active state when stream goes live)
   * NOTE: This is usually called automatically by Rust when stream_online event is received
   */
  const startSession = async (sessionId: string): Promise<void> => {
    const updated = await invoke<StreamSession>('streaming_start_session', { sessionId });

    // Update local cache
    const index = sessions.value.findIndex(s => s.id === sessionId);
    if (index !== -1) {
      sessions.value[index] = updated;
    }

    console.log('🔴 [SESSION MANAGER] Session started (LIVE):', sessionId);
  };

  /**
   * Pause session
   */
  const pauseSession = async (sessionId: string): Promise<void> => {
    const updated = await invoke<StreamSession>('streaming_pause_session', { sessionId });

    // Update local cache
    const index = sessions.value.findIndex(s => s.id === sessionId);
    if (index !== -1) {
      sessions.value[index] = updated;
    }

    console.log('⏸️ [SESSION MANAGER] Session paused:', sessionId);
  };

  /**
   * Resume session from pause
   */
  const resumeSession = async (sessionId: string): Promise<void> => {
    const updated = await invoke<StreamSession>('streaming_resume_session', { sessionId });

    // Update local cache
    const index = sessions.value.findIndex(s => s.id === sessionId);
    if (index !== -1) {
      sessions.value[index] = updated;
    }

    console.log('▶️ [SESSION MANAGER] Session resumed:', sessionId);
  };

  /**
   * End session - Save current run to history and set to inactive
   */
  const endSession = async (sessionId: string, auto = false): Promise<void> => {
    const updated = await invoke<StreamSession>('streaming_end_session', {
      sessionId,
      auto,
    });

    // Update local cache
    const index = sessions.value.findIndex(s => s.id === sessionId);
    if (index !== -1) {
      sessions.value[index] = updated;
    }

    console.log('⏹️ [SESSION MANAGER] Session ended:', sessionId, auto ? '(auto)' : '(manual)');
  };

  /**
   * Check for session conflicts (one account = one active session)
   */
  const checkSessionConflicts = async (
    platformsToCheck: PlatformStream[]
  ): Promise<StreamSession[]> => {
    return await invoke<StreamSession[]>('streaming_check_conflicts', {
      platforms: platformsToCheck,
    });
  };

  /**
   * Auto-resume session logic
   */
  const checkAutoResume = async (platform: string, accountId: string): Promise<StreamSession | null> => {
    return await invoke<StreamSession | null>('streaming_check_auto_resume', {
      platform,
      accountId,
    });
  };

  /**
   * Find session by account_id (platform + account pair)
   * Searches in active, waiting, and paused sessions
   */
  const findSessionByAccountId = async (platform: string, accountId: string): Promise<StreamSession | null> => {
    return await invoke<StreamSession | null>('streaming_find_session_by_account', {
      platform,
      accountId,
    });
  };

  /**
   * Find ANY session by account_id (including inactive)
   * Used for auto-activation when stream goes live
   */
  const findAnySessionByAccountId = (platform: string, accountId: string | number): StreamSession | null => {
    return sessions.value.find(s => {
      return s.platforms.some(p =>
        p.platform === platform &&
        p.accountId === String(accountId)
      );
    }) || null;
  };

  /**
   * Check if a session has any live streams (stream_id present)
   */
  const isSessionLive = async (sessionId: string): Promise<boolean> => {
    return await invoke<boolean>('streaming_is_session_live', { sessionId });
  };

  /**
   * Get all account IDs from a session
   */
  const getSessionAccountIds = async (sessionId: string): Promise<string[]> => {
    return await invoke<string[]>('streaming_get_session_account_ids', { sessionId });
  };

  /**
   * Computed properties
   */
  const activeSessions = computed(() =>
    sessions.value.filter(s =>
      s.status === 'active' || s.status === 'waiting_for_live' || s.status === 'paused'
    )
  );

  const draftSessions = computed(() =>
    sessions.value.filter(s => s.status === 'draft')
  );

  const inactiveSessions = computed(() =>
    sessions.value
      .filter(s => s.status === 'inactive')
      .sort((a, b) =>
        new Date(b.lastUsedAt ?? b.createdAt).getTime() -
        new Date(a.lastUsedAt ?? a.createdAt).getTime()
      )
  );

  const recentSessions = computed(() =>
    inactiveSessions.value.slice(0, 10)
  );

  return {
    // State
    sessions,
    loading,
    error,

    // Computed
    activeSessions,
    draftSessions,
    inactiveSessions,
    recentSessions,

    // Methods
    loadSessions,
    createSession,
    updateSession,
    deleteSession,
    activateSession,
    startSession,
    pauseSession,
    resumeSession,
    endSession,
    checkSessionConflicts,
    checkAutoResume,

    // Helper functions
    findSessionByAccountId,
    findAnySessionByAccountId,
    isSessionLive,
    getSessionAccountIds,
  };
};
