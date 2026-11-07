# Initial State Sync - Handling Late App Starts

## The Problem

**Scenario**: User starts streaming at 10:00 AM, then opens desktop app at 10:30 AM.

Without initial state sync, the desktop would miss the `stream_online` event and never activate the session, even though the stream is live.

## The Solution

Backend sends current state of all live streams in the `connection_established` event.

---

## Backend Implementation

### Connection Flow

```python
# Django Channels WebSocket Consumer

async def connect(self):
    # 1. Authenticate user
    user = await self.authenticate()

    # 2. Get all social accounts for this user
    user_accounts = await self.get_user_social_accounts(user.id)

    # 3. Check which accounts are currently streaming
    active_streams = []
    for account in user_accounts:
        stream_status = await self.get_current_stream_status(account)

        if stream_status.is_live:
            active_streams.append({
                'account_id': account.id,
                'platform': account.platform,
                'stream_id': stream_status.stream_id,
                'title': stream_status.title,
                'started_at': stream_status.started_at,
                'viewer_count': stream_status.viewer_count,
                'game_name': stream_status.game_name,
            })

    # 4. Send connection_established with active_streams
    await self.send(text_data=json.dumps({
        'type': 'connection_established',
        'streamer_id': str(user.id),
        'display_name': user.display_name,
        'timestamp': timezone.now().isoformat(),
        'active_streams': active_streams  # ← KEY: Include current state
    }))
```

### Message Format

**When streams are active:**
```json
{
  "type": "connection_established",
  "streamer_id": "uuid-abc-123",
  "display_name": "MyUsername",
  "timestamp": "2025-11-02T15:30:00Z",
  "active_streams": [
    {
      "account_id": 48,
      "platform": "twitch",
      "stream_id": "12345678901",
      "title": "Epic Gaming Stream - Day 5",
      "started_at": "2025-11-02T14:00:00Z",
      "game_name": "The Legend of Zelda",
      "viewer_count": 150,
      "stream_type": "live"
    },
    {
      "account_id": 52,
      "platform": "youtube",
      "stream_id": "dQw4w9WgXcQ",
      "title": "Epic Gaming Stream - Day 5",
      "started_at": "2025-11-02T14:05:00Z",
      "viewer_count": 75
    }
  ]
}
```

**When no streams are active:**
```json
{
  "type": "connection_established",
  "streamer_id": "uuid-abc-123",
  "display_name": "MyUsername",
  "timestamp": "2025-11-02T15:30:00Z",
  "active_streams": []
}
```

---

## Desktop Implementation

### 1. TypeScript Interfaces

useStreamingConnection.ts:41-50

```typescript
export interface ActiveStream {
  account_id: number;
  platform: 'twitch' | 'youtube' | 'kick' | 'tiktok';
  stream_id: string;
  title: string;
  started_at: string;
  game_name?: string;
  viewer_count?: number;
  stream_type?: string;
}

export interface CloudEvent {
  type: 'connection_established' | 'stream_online' | 'stream_offline' | ...;

  // Connection handshake
  streamer_id?: string;
  display_name?: string;
  timestamp?: string;
  active_streams?: ActiveStream[];  // ← Initial state

  // Event-specific fields
  account_id?: number;
  platform?: string;
  data?: any;
}
```

### 2. Event Handler

useStreamingConnection.ts:427-456

```typescript
function handleCloudEvent(event: CloudEvent) {
  switch (event.type) {
    case 'connection_established':
      console.log('✅ Connection established:', event.display_name);

      // Process initial state of active streams
      if (event.active_streams && event.active_streams.length > 0) {
        console.log(`🔄 Processing ${event.active_streams.length} active streams`);

        // Treat each active stream as a stream_online event
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
            }
          };

          // Notify callbacks (useStreamSessionSync will handle this)
          streamStatusCallbacks.forEach(cb => cb(streamOnlineEvent));
        });
      }
      break;
  }
}
```

### 3. Session Sync

useStreamSessionSync.ts:49-105

```typescript
async function handleStreamOnline(event: CloudEvent) {
  console.log('🎬 Stream went live:', event.platform, event.account_id);

  // Try to find session in WAITING_FOR_LIVE state
  let session = sessionManager.findSessionByAccountId(event.platform!, event.account_id!);

  if (!session) {
    // No waiting session - check for auto-resume candidates
    session = await sessionManager.checkAutoResume(event.platform!, event.account_id!);

    if (session) {
      console.log('🔄 Auto-resuming session:', session.name);

      // Check for conflicts
      const conflicts = await sessionManager.checkSessionConflicts(session.platforms);
      if (conflicts.length > 0) {
        console.log('⚠️ Cannot auto-resume due to conflicts');
        return;
      }

      // Resume session
      await sessionManager.updateSession(session.id, {
        status: 'waiting_for_live',
        started_at: new Date().toISOString(),
      });
    } else {
      console.log('⚠️ No session found or auto-resume candidate');
      return;
    }
  }

  // Update platform metadata
  const platform = session.platforms.find(p =>
    p.platform === event.platform &&
    String(p.account_id) === String(event.account_id)
  );

  if (platform && event.data) {
    platform.stream_id = event.data.stream_id;
    platform.title = event.data.title;
    platform.started_at = event.data.started_at;
    platform.viewer_count = event.data.viewer_count;
  }

  // Activate session
  await sessionManager.startSession(session.id);
  console.log('🔴 Session now ACTIVE:', session.name);
}
```

---

## Complete Flow Examples

### Example 1: App Opens AFTER Stream Started

```
Timeline:
10:00 - User starts stream (desktop app NOT running)
      - OBS goes live
      - Backend detects stream_online
      - Backend caches in Redis: active_stream_123 = {...}

10:30 - User opens desktop app
      - Desktop connects to WebSocket
      - Backend authenticates user
      - Backend queries Redis for active_stream_*
      - Backend sends connection_established with active_streams: [...]

      Desktop receives:
      ↓ handleCloudEvent('connection_established')
      ↓ active_streams.forEach → create stream_online events
      ↓ streamStatusCallbacks.forEach(cb)
      ↓ useStreamSessionSync.handleStreamOnline()
      ↓ findSessionByAccountId() → finds Session A in "waiting_for_live"
      ↓ startSession(sessionA.id) → Session A becomes ACTIVE
      ✅ Session synced even though app was late!
```

### Example 2: App Already Running (Normal Flow)

```
Timeline:
10:00 - Desktop app already running, connected to WebSocket
      - Session A in "waiting_for_live" state

10:05 - User starts stream
      - Backend detects stream_online
      - Backend broadcasts: stream_online event

      Desktop receives:
      ↓ handleCloudEvent('stream_online')
      ↓ streamStatusCallbacks.forEach(cb)
      ↓ useStreamSessionSync.handleStreamOnline()
      ↓ findSessionByAccountId() → finds Session A
      ↓ startSession(sessionA.id)
      ✅ Session activated normally
```

### Example 3: Multistream with Partial Late Start

```
Timeline:
10:00 - User starts Twitch stream (desktop running)
      - Session A activates (Twitch account_123)

10:30 - Desktop app crashes and restarts
      - Twitch is still live
      - YouTube not started yet

      Desktop connects:
      ↓ Backend sends: active_streams: [{account_id: 123, platform: 'twitch', ...}]
      ↓ Desktop processes stream_online for account_123
      ↓ findSessionByAccountId(twitch, 123) → finds Session A (paused from crash)
      ↓ checkAutoResume() → YES (within 30 min grace period)
      ↓ startSession(sessionA.id)
      ✅ Session auto-resumed!

10:35 - User starts YouTube stream
      - Backend sends: stream_online for account_456
      ↓ findSessionByAccountId(youtube, 456) → finds Session A (already active)
      ↓ Session A already has account_456 configured
      ↓ Updates platform metadata
      ✅ Multistream session now tracking both platforms!
```

---

## Why This Works

1. **No Race Conditions**: Backend sends initial state immediately on connection
2. **Unified Handling**: Desktop treats initial state exactly like real-time events
3. **Simple Logic**: `useStreamSessionSync.handleStreamOnline()` handles both cases
4. **Auto-Resume**: Works even if app crashed/restarted within 30-min window
5. **Multistream**: Correctly handles sessions with multiple platforms

---

## Backend Cache Strategy (Redis)

```python
# When stream goes live
async def on_stream_online(account_id, stream_data):
    # Cache for 6 hours (typical max stream length)
    await redis.setex(
        f'active_stream_{account_id}',
        6 * 60 * 60,  # 6 hours TTL
        json.dumps(stream_data)
    )

    # Broadcast to connected clients
    await channel_layer.group_send(...)

# When stream goes offline
async def on_stream_offline(account_id):
    # Remove from cache
    await redis.delete(f'active_stream_{account_id}')

    # Broadcast to connected clients
    await channel_layer.group_send(...)

# On WebSocket connection
async def get_active_streams(user_id):
    accounts = await get_user_accounts(user_id)
    active = []

    for account in accounts:
        stream_data = await redis.get(f'active_stream_{account.id}')
        if stream_data:
            active.append(json.loads(stream_data))

    return active
```

---

## Testing Checklist

- [ ] Start stream, then open app → Session activates
- [ ] Open app with no active streams → No errors
- [ ] Multistream: Start platform 1, open app, start platform 2 → Both tracked
- [ ] App crash during stream → Auto-resume on restart
- [ ] Stream for 31+ minutes, crash app → No auto-resume (grace period expired)
- [ ] Multiple sessions with same account → Conflict prevention works
- [ ] Stream offline event → Session pauses, then auto-ends after 30 min

---

## Summary

**Before**: Desktop missed events if app wasn't running when stream started

**After**: Backend sends `active_streams` array in `connection_established` event, desktop processes it as if those `stream_online` events just happened

**Key Benefit**: User can start streaming from OBS/StreamLabs, then open VortiDeck app later, and everything syncs perfectly!
