# Streaming Architecture: Logical & Design Analysis

## Executive Summary

This document analyzes the **architectural design decisions**, **logical flow**, and **conceptual patterns** used in the VortiDeck streaming system. Rather than focusing on performance optimizations, this examines **why** things are designed the way they are, **how** the pieces fit together logically, and what design patterns enable the system to work.

---

## Table of Contents

1. [Core Architectural Concepts](#core-architectural-concepts)
2. [The Three-Layer Separation Pattern](#the-three-layer-separation-pattern)
3. [Session Lifecycle State Machine](#session-lifecycle-state-machine)
4. [The Bridge Pattern Implementation](#the-bridge-pattern-implementation)
5. [Message Flow & Event-Driven Design](#message-flow--event-driven-design)
6. [Data Ownership & Synchronization Strategy](#data-ownership--synchronization-strategy)
7. [Command Execution Architecture](#command-execution-architecture)
8. [Design Strengths & Weaknesses](#design-strengths--weaknesses)
9. [Alternative Architectural Approaches](#alternative-architectural-approaches)
10. [Recommendations for Logical Improvements](#recommendations-for-logical-improvements)

---

## 1. Core Architectural Concepts

### The Problem Space

VortiDeck needs to solve a complex coordination problem:

1. **Backend Cloud Service** detects when users go live on multiple platforms (Twitch, YouTube, Kick, TikTok)
2. **Desktop Application** has user-configured session templates with deck actions and commands
3. **These two must sync** - when backend detects a stream, activate the matching desktop session
4. **Commands from chat** must trigger deck actions on the desktop app

### The Chosen Solution: Event-Driven Three-Layer Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Layer 1: Backend Sessions (Cloud - Source of Truth)   │
│  - Stream detection via platform APIs                  │
│  - Chat message aggregation                            │
│  - Real-time events via WebSocket                      │
└───────────────────┬─────────────────────────────────────┘
                    │ Events: session_created,
                    │         chat_message, etc.
                    ▼
┌─────────────────────────────────────────────────────────┐
│  Layer 2: Session Bridge (Coordination Logic)          │
│  - Maps backend sessions → local sessions              │
│  - Executes commands from chat                         │
│  - Handles auto-activation logic                       │
└───────────────────┬─────────────────────────────────────┘
                    │ Triggers state changes
                    ▼
┌─────────────────────────────────────────────────────────┐
│  Layer 3: Local Sessions (Desktop - User Config)       │
│  - Session templates (name, platforms, commands)       │
│  - Persisted to Tauri Store (JSON file)                │
│  - Deck action mappings                                │
└─────────────────────────────────────────────────────────┘
```

**Key Insight**: This is a **distributed state machine** where the backend and desktop maintain separate but coordinated state.

---

## 2. The Three-Layer Separation Pattern

### Why Three Layers?

#### **Layer 1: Backend Sessions (useStreamingConnection)**
- **Purpose**: Real-time stream detection and chat aggregation
- **Lifecycle**: Created when stream goes live, destroyed when stream ends
- **Data Model**: Platform-agnostic session with `session_id`, `platforms[]`, `is_multistream`
- **Ownership**: Backend is source of truth for "what's happening right now"

**Design Rationale**:
- Backend has OAuth tokens and can call platform APIs
- Backend can detect multistreaming (same person on Twitch + YouTube simultaneously)
- Backend handles rate limiting and API complexity
- Backend delivers real-time events via WebSocket

#### **Layer 2: Session Bridge (useSessionBridge)**
- **Purpose**: Coordination layer that maps backend events to desktop actions
- **Lifecycle**: Lives as long as the app is running
- **Logic**: Pattern matching between backend session data and local session configs
- **Ownership**: No state storage, pure coordination logic

**Design Rationale**:
- Decouples backend from desktop - they don't need to know about each other's data models
- Enables "smart matching" - backend doesn't need to know about local session templates
- Allows flexible mapping strategies (by account_id, by platform, etc.)
- Centralizes command execution logic

#### **Layer 3: Local Sessions (useSessionManager)**
- **Purpose**: User-created session templates with configurations
- **Lifecycle**: Persistent - survives app restarts
- **Data Model**: Rich session object with commands, settings, metadata
- **Ownership**: Desktop is source of truth for "what the user configured"

**Design Rationale**:
- Users create templates before streaming (like OBS scenes)
- Sessions can be reused - create once, activate many times
- Allows for features like auto-resume (session remembers past streams)
- Stores statistics (total messages, command executions)

### The Separation Principle

**Why not merge backend + local sessions?**

This would create tight coupling:
- Backend would need to store user's deck action configurations
- Desktop would need to handle stream detection logic
- User couldn't configure sessions while offline
- Backend downtime would lose user configurations

**The current design treats them as separate concerns**:
- Backend: "Here's what's happening on the streaming platforms"
- Desktop: "Here's what I want to do when that happens"
- Bridge: "Let me connect these two"

---

## 3. Session Lifecycle State Machine

### State Diagram

```
User Creates Session
        ↓
    [ DRAFT ]  ────────────────────┐
        ↓ activateSession()         │
        │                           │ User never activates
        ▼                           │
[ WAITING_FOR_LIVE ] ←──────────┐  │
        ↓                        │  │
        │ Backend: session_created│  │
        ▼                        │  │
    [ ACTIVE ]                   │  │
        ↓                        │  │
        │ Backend: session_ended  │  │
        ▼                        │  │
    [ PAUSED ]                   │  │
        ├─────────────────────┐  │  │
        │                     │  │  │
        │ Stream restarts     │  │  │
        │ (within 30 min)     │  │  │
        │                     │  │  │
        └─→ resumeSession() ──┘  │  │
        │                        │  │
        │ 30 min timeout         │  │
        │ OR user manual end     │  │
        ▼                        │  │
[ ENDED_AUTO / ENDED_BY_USER ]   │  │
        │                        │  │
        └────────────────────────┴──┘
        (Sessions stay in storage as history)
```

### State Transition Logic

**DRAFT → WAITING_FOR_LIVE**
- Trigger: User clicks "Activate" on session
- Logic: Check for conflicts (one account can't have multiple active sessions)
- Validation: Ensure platforms have connected accounts
- Result: Session is "armed" and waiting for stream

**WAITING_FOR_LIVE → ACTIVE**
- Trigger: Backend sends `session_created` event
- Logic: Bridge matches backend session to local session by `account_id`
- Validation: Session must be in `waiting_for_live` state
- Result: Session is now live, commands active, stats tracking begins

**ACTIVE → PAUSED**
- Trigger: Backend sends `session_ended` event (all platforms offline)
- Logic: Stream ended but might restart (network issue, platform bug, etc.)
- Grace Period: 30 minutes for stream to come back
- Result: Session paused, commands inactive, grace timer started

**PAUSED → ACTIVE (Auto-Resume)**
- Trigger: Backend sends new `session_created` with same `account_id`
- Logic: Bridge checks for paused sessions within grace period
- Validation: Session must have `autoResume: true` in settings
- Result: Session resumes seamlessly, stats continue

**PAUSED → ENDED_AUTO**
- Trigger: Grace period timeout (30 minutes elapsed)
- Logic: Stream didn't restart, assume intentional end
- Result: Session archived with final statistics

**ANY → ENDED_BY_USER**
- Trigger: User clicks "End Session"
- Logic: Immediate termination regardless of stream status
- Result: Session archived, user took manual action

### Why This State Machine?

**The Problem**: Stream platforms are unreliable
- Disconnections happen (network issues, platform bugs)
- Users test streams (go live, stop, go live again)
- Multistreaming can fail on one platform while others work

**The Solution**: Grace period + auto-resume
- Don't immediately end session when stream stops
- Give 30 minutes for stream to recover
- Auto-resume if stream comes back
- Prevents users from having to manually restart session

**Alternative Approaches Considered**:

1. **No grace period** - End immediately when stream stops
   - ❌ Problem: Too aggressive, users would complain about false endings

2. **Infinite grace period** - Never auto-end
   - ❌ Problem: Sessions would stay "active" forever, confusing UI

3. **Configurable grace period** - User sets duration
   - ✅ Could be added later, 30 min is reasonable default

---

## 4. The Bridge Pattern Implementation

### What is the Bridge Pattern?

Classic Bridge Pattern:
```
Abstraction ──uses──> Implementation
    │                      │
    ├─ RefinedAbstraction  ├─ ConcreteImplementation1
    └─ ...                 └─ ConcreteImplementation2
```

In VortiDeck's case:
```
Backend Sessions ──bridge──> Local Sessions
      │                           │
      ├─ Cloud data model         ├─ Desktop data model
      ├─ Ephemeral                ├─ Persistent
      └─ Platform-focused         └─ User-focused
```

### How the Bridge Works

**File**: `useSessionBridge.ts`

#### 1. **Subscription to Events**

```typescript
function initialize() {
  // Subscribe to backend session lifecycle
  streaming.onSession((sessionId, session, action) => {
    handleBackendSession(sessionId, session, action);
  });

  // Subscribe to chat messages for commands
  streaming.onMessage((message) => {
    handleChatMessage(message);
  });
}
```

**Design Decision**: Observer pattern with callbacks
- Bridge doesn't poll - it reacts to events
- Loose coupling - backend doesn't know bridge exists
- Multiple bridges could theoretically exist (extensible)

#### 2. **Session Matching Logic**

```typescript
function handleSessionCreated(backendSession) {
  const accountIds = backendSession.platforms.map(p => p.account_id);

  // Find local session in WAITING_FOR_LIVE with matching account
  const localSession = sessionManager.sessions.value.find(s => {
    if (s.status !== 'waiting_for_live') return false;

    return s.platforms.some(localPlatform =>
      accountIds.includes(String(localPlatform.account_id))
    );
  });

  if (localSession) {
    sessionManager.startSession(localSession.id);
    localSession.backend_session_id = backendSession.id;
  }
}
```

**Key Design Decision**: Match by `account_id`
- Why not by platform name? Multiple accounts per platform (e.g., two Twitch accounts)
- Why not by session name? Backend doesn't know local session names
- Why not by user ID? User might have multiple sessions for different content types

**The Matching Problem**:
1. User creates session "Gaming Stream" with Twitch account #123
2. User activates session → status becomes `waiting_for_live`
3. User goes live on Twitch
4. Backend detects stream, creates backend session with `account_id: 123`
5. Bridge receives `session_created` event
6. Bridge finds local session with `account_id: 123` in `waiting_for_live` state
7. Bridge activates local session → status becomes `active`

**Edge Cases Handled**:
- Multistreaming: Backend session has multiple platforms, match by any account_id
- Multiple waiting sessions: First match wins (FIFO)
- No waiting session: Backend session exists but no local session is activated (user just chatting)

#### 3. **Command Execution Bridge**

```typescript
function handleChatMessage(message) {
  if (!message.message.startsWith('!')) return;

  const trigger = message.message.split(' ')[0]; // "!alert"

  // Find active session with matching platform
  const localSession = sessionManager.sessions.value.find(s => {
    if (s.status !== 'active') return false;

    return s.platforms.some(p =>
      p.platform === message.platform && p.stream_id
    );
  });

  if (localSession) {
    executeCommand(localSession, trigger);
  }
}

async function executeCommand(session, trigger) {
  const command = session.commands.find(c =>
    c.trigger === trigger && c.enabled
  );

  if (command) {
    await invoke('execute_deck_action', {
      actionId: command.deckActionId,
    });
  }
}
```

**Design Decision**: Chat message → Deck action pipeline
1. WebSocket receives chat message from backend
2. Bridge checks if message is a command (starts with `!`)
3. Bridge finds active session for that platform
4. Bridge looks up command in session's command list
5. Bridge executes deck action via Tauri invoke

**Why this flow?**
- Commands are session-specific (different sessions, different commands)
- Only active sessions should respond to commands (no accidental triggers)
- Deck actions are desktop-only (can't execute in web app)

---

## 5. Message Flow & Event-Driven Design

### WebSocket Event Types

**File**: `useStreamingConnection.ts`

```typescript
export interface CloudEvent {
  type:
    | 'connection_established'  // Initial handshake
    | 'session_created'         // Stream went live
    | 'session_updated'         // Multistream detected / title changed
    | 'session_ended'           // Stream ended
    | 'chat_message'            // Real-time chat
    | 'stream_event'            // Sub, raid, follow, etc.
    | 'platform_status'         // Platform online/offline
    | 'send_chat_result'        // Chat send confirmation
    | 'error';                  // Backend error
}
```

### Event Flow Diagram

```
User goes live on Twitch
         │
         ▼
Twitch EventSub → Backend detects stream
         │
         ▼
Backend creates session {
  id: "backend-123",
  platforms: [{ platform: "twitch", account_id: 456 }]
}
         │
         ▼
WebSocket sends: {
  type: "session_created",
  session: {...}
}
         │
         ▼
Desktop receives event → handleCloudEvent()
         │
         ├─→ Notifies all eventCallbacks
         ├─→ Notifies all sessionCallbacks
         └─→ Stores session in Map
         │
         ▼
Session Bridge receives callback
         │
         ▼
Bridge matches to local session
         │
         ▼
Local session status: WAITING_FOR_LIVE → ACTIVE
         │
         ▼
UI updates (chat view shows session is live)
```

### Callback Subscription Pattern

**Design Pattern**: Publish-Subscribe (Observer)

```typescript
// Publisher (useStreamingConnection)
const sessionCallbacks = new Set<(sessionId, session, action) => void>();

function onSession(callback) {
  sessionCallbacks.add(callback);
  return () => sessionCallbacks.delete(callback); // Unsubscribe
}

// When event occurs
sessionCallbacks.forEach(cb => cb(sessionId, session, 'created'));
```

```typescript
// Subscriber (useSessionBridge)
const unsubscribe = streaming.onSession((sessionId, session, action) => {
  handleBackendSession(sessionId, session, action);
});

// Cleanup
onUnmounted(() => {
  unsubscribe();
});
```

**Why callbacks instead of reactive refs?**

Option 1: Reactive refs (Vue way)
```typescript
const backendSessions = ref(new Map());
// Components watch() the ref
```
- ✅ Pro: Automatic reactivity
- ❌ Con: All components re-render on any session change
- ❌ Con: Can't have custom logic per subscriber

Option 2: Callbacks (Current approach)
```typescript
onSession((sessionId, session, action) => {
  // Custom logic here
});
```
- ✅ Pro: Subscribers can execute custom logic
- ✅ Pro: Decoupled - publisher doesn't know subscribers
- ✅ Pro: Fine-grained control over when to react
- ❌ Con: More manual - need to manage subscriptions

**The Choice**: Callbacks for flexibility
- Bridge needs to do complex matching logic
- Chat UI needs to update message lists
- Stats components need to update counters
- Each has different needs → callbacks allow customization

---

## 6. Data Ownership & Synchronization Strategy

### The Dual-State Problem

**Problem**: Backend and desktop both have "sessions" but they're different:

Backend Session:
```typescript
{
  id: "backend-abc123",
  title: "Playing Valorant",
  platforms: ["twitch", "youtube"],
  started_at: "2025-01-15T10:30:00Z",
  is_multistream: true
}
```

Local Session:
```typescript
{
  id: "local-xyz789",
  name: "Gaming Stream",
  platforms: [
    { platform: "twitch", account_id: 456 }
  ],
  commands: [
    { trigger: "!alert", deckActionId: "action-1" }
  ],
  status: "active",
  backend_session_id: "backend-abc123"  // Link!
}
```

### Synchronization Strategy

**Current Approach**: One-way binding with reference link

```
Backend Session (authoritative for stream state)
        │
        │ backend_session_id stored in local session
        ▼
Local Session (authoritative for user config)
```

**Flow**:
1. Backend creates session → Bridge receives event
2. Bridge finds matching local session
3. Bridge stores `backend_session_id` in local session
4. Backend updates session (multistream) → Bridge receives event
5. Bridge uses `backend_session_id` to find local session
6. Bridge updates local session's platform metadata

**What gets synced?**

From Backend → Desktop:
- ✅ `stream_id` (stream is live)
- ✅ `title` (stream title)
- ✅ `started_at` (when stream started)
- ✅ `viewer_count` (current viewers)
- ✅ `is_multistream` (multiple platforms detected)

From Desktop → Backend:
- ❌ Nothing

**Why one-way?**

Backend doesn't need to know:
- Session names ("Gaming Stream")
- Commands configured ("!alert")
- Deck action mappings
- User preferences (autoResume, notifications)

Desktop doesn't control:
- When stream goes live (platform does)
- Stream title (user sets on platform)
- Viewer count (platform reports)

**The Design Principle**: Separation of Concerns
- Backend: "What's happening on platforms"
- Desktop: "What to do when it happens"
- No circular dependencies

### Alternative Approaches

**1. Two-Way Sync (Rejected)**
```
Backend ←→ Desktop
```
- Would require backend to store user's deck configurations
- Would require desktop to report to backend constantly
- Creates tight coupling and potential sync conflicts

**2. Backend as Single Source of Truth (Rejected)**
```
Backend (stores everything)
   ↓
Desktop (just a viewer)
```
- User couldn't configure sessions offline
- Deck actions are desktop-only (can't store in cloud)
- Backend would need to know about Tauri-specific data

**3. Desktop as Single Source of Truth (Rejected)**
```
Desktop (does everything)
   ↓
Polls platforms for stream status
```
- Desktop would need OAuth tokens for all platforms
- Desktop would need to handle rate limiting
- Can't aggregate chat from multiple platforms
- Doesn't work for web app (no Tauri)

**4. Current: Distributed State with Bridge (Chosen)**
```
Backend ←─bridge─→ Desktop
(Stream state)    (User config)
```
- Each owns their domain
- Bridge coordinates
- Loosely coupled
- Can evolve independently

---

## 7. Command Execution Architecture

### The Command Flow

```
User types "!alert" in Twitch chat
         │
         ▼
Twitch → Backend receives chat message via EventSub
         │
         ▼
Backend sends to Desktop via WebSocket: {
  type: "chat_message",
  session_id: "backend-123",
  platform: "twitch",
  message: "!alert",
  user_id: "user456",
  ...
}
         │
         ▼
useStreamingConnection receives message
         │
         ├─→ Stores in chatMessagesBySession Map
         └─→ Calls all messageCallbacks
         │
         ▼
useSessionBridge receives callback
         │
         ▼
Bridge checks: message.startsWith('!')? → YES
         │
         ▼
Bridge extracts trigger: "!alert"
         │
         ▼
Bridge finds active local session for platform "twitch"
         │
         ▼
Bridge looks up command:
  session.commands.find(c => c.trigger === "!alert")
         │
         ▼
Command found: { trigger: "!alert", deckActionId: "action-1" }
         │
         ▼
Bridge executes: invoke('execute_deck_action', { actionId: "action-1" })
         │
         ▼
Tauri backend executes deck action (plays sound, changes scene, etc.)
```

### Command Data Model

```typescript
interface SessionCommand {
  trigger: string;        // "!alert" (without !)
  deckActionId: string;   // "action-abc123"
  cooldown?: number;      // Seconds between executions
  enabled: boolean;       // Can be toggled off
}
```

### Design Decisions

**Q: Why store commands in local sessions instead of backend?**

A: Commands are desktop-specific
- Deck actions only exist on desktop (Tauri invoke)
- User configures commands in desktop UI
- Commands might trigger local files, apps, OBS scenes
- Web app doesn't have deck actions

**Q: Why not use a command service/registry pattern?**

Registry Pattern:
```typescript
commandRegistry.register("!alert", () => {
  // Execute action
});
```

Current Pattern:
```typescript
session.commands = [
  { trigger: "!alert", deckActionId: "..." }
];
```

**Reasoning**:
- ✅ Commands are session-specific (different streams, different commands)
- ✅ Easy to persist in session JSON
- ✅ Users can copy/paste sessions with commands included
- ✅ Clear data model - commands are session properties
- ❌ Global registry would lose session context

**Q: Why execute in bridge instead of useStreamingConnection?**

If in useStreamingConnection:
- ❌ Connection layer would need to know about deck actions
- ❌ Connection layer would need to know about sessions
- ❌ Tight coupling between WebSocket and desktop features

Current approach (in bridge):
- ✅ Connection layer is pure - just WebSocket + events
- ✅ Bridge has context of both backend and desktop
- ✅ Command logic is centralized
- ✅ Easy to test bridge in isolation

### Cooldown System (Future Enhancement)

Currently missing but designed for:
```typescript
const commandCooldowns = new Map<string, number>(); // trigger → last execution time

async function executeCommand(session, trigger) {
  const command = session.commands.find(c => c.trigger === trigger);
  if (!command || !command.enabled) return;

  // Check cooldown
  if (command.cooldown) {
    const lastExecution = commandCooldowns.get(trigger) || 0;
    const timeSince = Date.now() - lastExecution;

    if (timeSince < command.cooldown * 1000) {
      console.log('⏱️ Command on cooldown');
      return;
    }
  }

  await invoke('execute_deck_action', { actionId: command.deckActionId });
  commandCooldowns.set(trigger, Date.now());
}
```

---

## 8. Design Strengths & Weaknesses

### Architectural Strengths

#### ✅ 1. **Clear Separation of Concerns**
- Backend handles platform integrations
- Desktop handles user configurations
- Bridge coordinates without coupling

#### ✅ 2. **Event-Driven Scalability**
- Adding new event types doesn't break existing code
- Subscribers can be added/removed dynamically
- WebSocket handles backpressure naturally

#### ✅ 3. **Offline-First Local Sessions**
- Users can create/edit sessions without internet
- Sessions persist across app restarts
- No dependency on backend for configuration

#### ✅ 4. **Flexible Matching Logic**
- Bridge can evolve matching algorithms
- Can add fuzzy matching, AI-based matching, etc.
- Backend and desktop data models can evolve independently

#### ✅ 5. **State Machine Clarity**
- Session states are well-defined
- Transitions are explicit
- Easy to reason about session lifecycle

#### ✅ 6. **Composable Architecture (Vue 3)**
- Each composable has single responsibility
- Easy to test in isolation
- Can reuse composables in different contexts

### Architectural Weaknesses

#### ❌ 1. **No Explicit State Synchronization Protocol**

**Problem**: If desktop is offline when stream goes live
```
1. User activates session → waiting_for_live
2. User goes live on Twitch
3. Backend creates session
4. Desktop is OFFLINE → misses session_created event
5. User reconnects
6. Session still in waiting_for_live (should be active)
```

**Current Behavior**:
- `connection_established` event sends all active sessions
- But bridge only matches if session is in `waiting_for_live`
- If session was never activated, won't match

**Fix Needed**:
- Store activation timestamp in local session
- On reconnect, check if stream is currently live
- Auto-activate session if it was waiting

#### ❌ 2. **No Conflict Resolution Strategy**

**Problem**: Multiple local sessions waiting for same account
```
Session A: "Gaming Stream" (account 123) → waiting_for_live
Session B: "Just Chatting" (account 123) → waiting_for_live
User goes live → Backend creates session for account 123
Which local session should activate?
```

**Current Behavior**: First match wins (order-dependent)

**Better Approach**:
- Priority system (user sets preferred session)
- Last-modified wins
- Prompt user to choose
- Block activation if conflict exists

#### ❌ 3. **Tight Coupling Between Chat Messages and Sessions**

**Problem**: Chat messages require `session_id`
```typescript
if (data.session_id) {
  const messages = chatMessagesBySession.value.get(data.session_id) || [];
  messages.push(data);
} else {
  console.warn('⚠️ Message without session_id');
}
```

**Issue**:
- If backend hasn't detected stream yet, messages have no session_id
- Messages are lost or not stored properly
- User is chatting but messages don't appear

**Better Approach**:
- Store messages by platform + account_id temporarily
- Once session is created, migrate messages to session
- Never lose messages

#### ❌ 4. **No Versioning Strategy**

**Problem**: Session data model changes over time
```typescript
// v1
{ id, name, platforms }

// v2 (added commands)
{ id, name, platforms, commands }

// v3 (added settings)
{ id, name, platforms, commands, settings }
```

**Current Behavior**: Load from JSON, hope it works

**Risk**:
- Old sessions might crash app
- Missing fields cause errors
- No migration path

**Better Approach**:
```typescript
interface StreamSession {
  version: number;
  // ...
}

async function loadSessions() {
  const sessions = await store.get('sessions');
  return sessions.map(migrateSession);
}

function migrateSession(session) {
  if (session.version === 1) {
    session.commands = [];
    session.version = 2;
  }
  if (session.version === 2) {
    session.settings = { autoStart: true, ... };
    session.version = 3;
  }
  return session;
}
```

#### ❌ 5. **Manual Subscription Management**

**Problem**: Components must remember to unsubscribe
```typescript
onMounted(() => {
  const unsubscribe = streaming.onSession(handleSession);
});
// What if component unmounts? Memory leak!
```

**Current Approach**: Manual cleanup
```typescript
let unsubscribe = null;

onMounted(() => {
  unsubscribe = streaming.onSession(handleSession);
});

onUnmounted(() => {
  if (unsubscribe) unsubscribe();
});
```

**Better Approach**: Auto-cleanup with composable
```typescript
function useStreamingEvents() {
  const subscriptions = [];

  function onSession(callback) {
    const unsub = streaming.onSession(callback);
    subscriptions.push(unsub);
  }

  onUnmounted(() => {
    subscriptions.forEach(unsub => unsub());
  });

  return { onSession };
}
```

#### ❌ 6. **Implicit Bridge Initialization**

**Problem**: Bridge only initializes when component mounts
```typescript
// useSessionBridge.ts
onMounted(() => {
  initialize();
});
```

**Issue**:
- If user never visits streaming page, bridge never starts
- Commands won't execute
- Sessions won't auto-activate
- Confusing for users

**Better Approach**:
- Initialize bridge in app root (plugins/streaming.ts)
- Or use Nuxt plugin with app lifecycle
- Ensure bridge is always active when logged in

---

## 9. Alternative Architectural Approaches

### Approach 1: CQRS (Command Query Responsibility Segregation)

**Concept**: Separate read and write models

```
┌─────────────────┐
│  Command Side   │  (Mutations)
│  - createSession│
│  - activateSession│
│  - sendChatMessage│
└────────┬────────┘
         │
         ▼
    Event Store
         │
         ▼
┌────────┴────────┐
│   Query Side    │  (Reads)
│  - getSessions  │
│  - getMessages  │
└─────────────────┘
```

**For VortiDeck**:
```typescript
// Command side
sessionCommands.activate(sessionId);
sessionCommands.end(sessionId);

// Query side
const sessions = sessionQueries.getActive();
const messages = sessionQueries.getMessagesForSession(sessionId);

// Event sourcing
eventStore.emit('SessionActivated', { sessionId, timestamp });
```

**Pros**:
- ✅ Clear separation between actions and queries
- ✅ Event sourcing enables time-travel debugging
- ✅ Can replay events to rebuild state
- ✅ Audit trail built-in

**Cons**:
- ❌ More complex than current approach
- ❌ Eventual consistency can be confusing
- ❌ Overkill for current scale

**When to use**: If you need full audit history of all session changes

---

### Approach 2: State Machine Library (XState)

**Concept**: Formalize state machine with library

```typescript
import { createMachine, interpret } from 'xstate';

const sessionMachine = createMachine({
  id: 'session',
  initial: 'draft',
  states: {
    draft: {
      on: { ACTIVATE: 'waiting_for_live' }
    },
    waiting_for_live: {
      on: {
        STREAM_DETECTED: 'active',
        CANCEL: 'draft'
      }
    },
    active: {
      on: {
        STREAM_ENDED: 'paused',
        USER_END: 'ended_by_user'
      }
    },
    paused: {
      on: {
        STREAM_RESUMED: 'active',
        TIMEOUT: 'ended_auto'
      },
      after: {
        1800000: 'ended_auto' // 30 min
      }
    },
    ended_by_user: { type: 'final' },
    ended_auto: { type: 'final' }
  }
});

const sessionService = interpret(sessionMachine);
sessionService.start();

// Use it
sessionService.send('ACTIVATE');
sessionService.send('STREAM_DETECTED');
```

**Pros**:
- ✅ Impossible to enter invalid states
- ✅ Visualize state machine with tools
- ✅ Guards and actions built-in
- ✅ Type-safe transitions

**Cons**:
- ❌ Learning curve
- ❌ Adds dependency
- ❌ Current simple state machine works fine

**When to use**: If state transitions become more complex (e.g., multiple paused states, pending states)

---

### Approach 3: Actor Model (Message Passing)

**Concept**: Each session is an actor that receives messages

```typescript
class SessionActor {
  constructor(id) {
    this.id = id;
    this.state = 'draft';
  }

  receive(message) {
    switch (message.type) {
      case 'ACTIVATE':
        this.state = 'waiting_for_live';
        this.emit('activated');
        break;

      case 'STREAM_DETECTED':
        if (this.state === 'waiting_for_live') {
          this.state = 'active';
          this.emit('started');
        }
        break;

      case 'CHAT_MESSAGE':
        if (this.state === 'active') {
          this.handleChatMessage(message.data);
        }
        break;
    }
  }

  handleChatMessage(message) {
    if (message.startsWith('!')) {
      this.executeCommand(message);
    }
  }
}

// Usage
const sessionActor = new SessionActor('session-1');
sessionActor.receive({ type: 'ACTIVATE' });
sessionActor.receive({ type: 'STREAM_DETECTED' });
sessionActor.receive({ type: 'CHAT_MESSAGE', data: '!alert' });
```

**Pros**:
- ✅ Each session is isolated
- ✅ Easy to test individual sessions
- ✅ Scales to many sessions
- ✅ Clear message-passing semantics

**Cons**:
- ❌ Different paradigm from current Vue approach
- ❌ More boilerplate
- ❌ Current callback approach works well

**When to use**: If managing 100s of concurrent sessions

---

### Approach 4: Reactive Streams (RxJS)

**Concept**: Model events as observable streams

```typescript
import { fromEvent, merge } from 'rxjs';
import { filter, map, mergeMap } from 'rxjs/operators';

const backendEvents$ = websocketMessages$.pipe(
  map(msg => JSON.parse(msg.data))
);

const sessionCreated$ = backendEvents$.pipe(
  filter(event => event.type === 'session_created'),
  map(event => event.session)
);

const chatMessages$ = backendEvents$.pipe(
  filter(event => event.type === 'chat_message'),
  map(event => event.data)
);

const commands$ = chatMessages$.pipe(
  filter(msg => msg.message.startsWith('!')),
  map(msg => ({
    trigger: msg.message.split(' ')[0],
    message: msg
  }))
);

// React to streams
sessionCreated$.subscribe(session => {
  matchAndActivateLocalSession(session);
});

commands$.subscribe(cmd => {
  executeCommand(cmd.trigger);
});
```

**Pros**:
- ✅ Declarative event handling
- ✅ Built-in operators for filtering, mapping, debouncing
- ✅ Easy to combine multiple streams
- ✅ Backpressure handling

**Cons**:
- ❌ Learning curve (RxJS is complex)
- ❌ Adds heavy dependency
- ❌ Vue reactivity system is simpler

**When to use**: If dealing with complex event streams (rate limiting, debouncing, retries)

---

## 10. Recommendations for Logical Improvements

### Priority 1: Fix Session Matching Edge Cases

**Issue**: Current matching is fragile

**Current Code**:
```typescript
const localSession = sessionManager.sessions.value.find(s => {
  if (s.status !== 'waiting_for_live') return false;

  return s.platforms.some(localPlatform =>
    accountIds.includes(String(localPlatform.account_id))
  );
});
```

**Problems**:
1. What if multiple sessions match? (first wins - unpredictable)
2. What if session was activated but desktop was offline?
3. What if user creates session AFTER going live?

**Recommended Approach**:

```typescript
function findBestMatchingSession(backendSession) {
  const accountIds = backendSession.platforms.map(p => p.account_id);

  // 1. Find all potential matches
  const candidates = sessionManager.sessions.value.filter(s => {
    // Match by account_id
    const hasMatchingAccount = s.platforms.some(p =>
      accountIds.includes(String(p.account_id))
    );

    if (!hasMatchingAccount) return false;

    // Valid states for matching
    const validStates = ['waiting_for_live', 'paused'];
    return validStates.includes(s.status);
  });

  if (candidates.length === 0) return null;
  if (candidates.length === 1) return candidates[0];

  // 2. Multiple matches - use priority system
  const prioritized = candidates.sort((a, b) => {
    // Priority 1: Exact platform match (all platforms)
    const aExactMatch = a.platforms.length === backendSession.platforms.length;
    const bExactMatch = b.platforms.length === backendSession.platforms.length;
    if (aExactMatch && !bExactMatch) return -1;
    if (!aExactMatch && bExactMatch) return 1;

    // Priority 2: Most recently activated
    const aTime = new Date(a.started_at || 0).getTime();
    const bTime = new Date(b.started_at || 0).getTime();
    return bTime - aTime;
  });

  return prioritized[0];
}
```

**Additional Fix**: Prevent conflicts at activation time

```typescript
async function activateSession(sessionId) {
  const session = sessions.value.find(s => s.id === sessionId);
  if (!session) throw new Error('Session not found');

  // Check for conflicts
  const conflicts = sessions.value.filter(s => {
    if (s.id === sessionId) return false;
    if (!['waiting_for_live', 'active', 'paused'].includes(s.status)) return false;

    return s.platforms.some(p1 =>
      session.platforms.some(p2 =>
        p1.platform === p2.platform &&
        p1.account_id === p2.account_id
      )
    );
  });

  if (conflicts.length > 0) {
    throw new Error(
      `Cannot activate: account is already used in session "${conflicts[0].name}". ` +
      `End that session first.`
    );
  }

  // Proceed with activation
  await updateSession(sessionId, {
    status: 'waiting_for_live',
    started_at: new Date().toISOString(),
  });
}
```

---

### Priority 2: Add Session Sync Protocol

**Issue**: Desktop doesn't recover if offline during session creation

**Recommended Approach**: Add sync on reconnect

```typescript
function handleConnectionEstablished(data) {
  console.log('✅ Connection established');
  isConnected.value = true;

  // Receive initial backend sessions
  if (data.sessions) {
    initializeSessions(data.sessions);

    // NEW: Sync local waiting sessions with backend
    syncWaitingSessionsWithBackend(data.sessions);
  }
}

function syncWaitingSessionsWithBackend(backendSessions) {
  const waitingSessions = sessionManager.sessions.value.filter(
    s => s.status === 'waiting_for_live'
  );

  waitingSessions.forEach(localSession => {
    const accountIds = localSession.platforms.map(p => p.account_id);

    // Find matching backend session
    const backendSession = backendSessions.find(bs =>
      bs.platforms.some(p =>
        accountIds.includes(String(p.account_id || p))
      )
    );

    if (backendSession) {
      // Stream is live! Activate local session
      console.log('🔄 Syncing waiting session:', localSession.name);
      sessionManager.startSession(localSession.id);
      localSession.backend_session_id = backendSession.id;
      sessionManager.updateSession(localSession.id, localSession);
    }
  });
}
```

---

### Priority 3: Implement Proper Error Boundaries

**Issue**: Errors in bridge can crash entire app

**Recommended Approach**: Add try-catch with error recovery

```typescript
function handleBackendSession(sessionId, session, action) {
  try {
    console.log('🔗 Bridge: Backend session event:', action, sessionId);

    switch (action) {
      case 'created':
        handleSessionCreated(session);
        break;

      case 'updated':
        handleSessionUpdated(sessionId, session);
        break;

      case 'ended':
        handleSessionEnded(sessionId);
        break;
    }
  } catch (error) {
    console.error('❌ Bridge error:', error);

    // Log to error tracking service (Sentry, etc.)
    logErrorToService({
      context: 'session-bridge',
      action,
      sessionId,
      error: error.message,
      stack: error.stack,
    });

    // Show user-friendly notification
    mainStore.addNotification({
      title: 'Session Sync Error',
      message: 'Failed to sync streaming session. Refresh the page if issues persist.',
      type: 'error',
      autoClose: true,
    });

    // Don't crash - let other sessions continue working
  }
}
```

---

### Priority 4: Add Session Versioning & Migration

**Issue**: Data model changes break old sessions

**Recommended Approach**: Version migrations

```typescript
const CURRENT_SESSION_VERSION = 3;

interface StreamSession {
  version: number;  // NEW
  id: string;
  name: string;
  // ...
}

async function loadSessions() {
  loading.value = true;

  try {
    const sessionStore = await initStore();
    const storedSessions = await sessionStore.get<StreamSession[]>('sessions');

    // Migrate sessions to current version
    sessions.value = (storedSessions || []).map(migrateSession);

    // Sort by created_at
    sessions.value.sort((a, b) =>
      new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
    );

    console.log(`📊 Loaded ${sessions.value.length} sessions from Tauri Store`);
  } catch (err) {
    error.value = err instanceof Error ? err.message : 'Failed to load sessions';
    console.error('Failed to load sessions:', err);
  } finally {
    loading.value = false;
  }
}

function migrateSession(session: any): StreamSession {
  // Default version 1 if not specified
  let current = { ...session, version: session.version || 1 };

  // v1 → v2: Add commands array
  if (current.version === 1) {
    current.commands = current.commands || [];
    current.version = 2;
  }

  // v2 → v3: Add settings object
  if (current.version === 2) {
    current.settings = current.settings || {
      autoStart: true,
      notifications: true,
      autoResume: true,
    };
    current.version = 3;
  }

  // Future migrations
  // if (current.version === 3) { ... }

  return current as StreamSession;
}
```

---

### Priority 5: Centralize Command Execution Logic

**Issue**: Command logic is scattered

**Recommended Approach**: Extract to dedicated composable

```typescript
// New file: useCommandExecutor.ts

export function useCommandExecutor() {
  const sessionManager = useSessionManager();
  const cooldowns = new Map<string, number>();

  async function executeCommand(
    sessionId: string,
    trigger: string,
    context: {
      userId: string;
      platform: string;
      message: string;
    }
  ) {
    const session = sessionManager.sessions.value.find(s => s.id === sessionId);
    if (!session) {
      console.warn('⚠️ Session not found:', sessionId);
      return { success: false, reason: 'session_not_found' };
    }

    if (session.status !== 'active') {
      console.warn('⚠️ Session not active:', session.status);
      return { success: false, reason: 'session_not_active' };
    }

    // Find command
    const command = session.commands.find(c =>
      c.trigger === trigger && c.enabled
    );

    if (!command) {
      console.warn('⚠️ Command not found or disabled:', trigger);
      return { success: false, reason: 'command_not_found' };
    }

    // Check cooldown
    if (command.cooldown) {
      const cooldownKey = `${sessionId}:${trigger}`;
      const lastExecution = cooldowns.get(cooldownKey) || 0;
      const timeSince = Date.now() - lastExecution;

      if (timeSince < command.cooldown * 1000) {
        const remaining = Math.ceil((command.cooldown * 1000 - timeSince) / 1000);
        console.warn(`⏱️ Command on cooldown: ${remaining}s remaining`);
        return { success: false, reason: 'cooldown', remaining };
      }

      cooldowns.set(cooldownKey, Date.now());
    }

    // Execute deck action
    try {
      await invoke('execute_deck_action', {
        actionId: command.deckActionId,
      });

      // Update stats
      session.command_executions = (session.command_executions || 0) + 1;
      await sessionManager.updateSession(sessionId, session);

      console.log('✅ Command executed:', trigger, 'by', context.userId);

      return {
        success: true,
        command,
        context
      };
    } catch (error) {
      console.error('❌ Failed to execute command:', error);
      return {
        success: false,
        reason: 'execution_failed',
        error: error.message
      };
    }
  }

  return {
    executeCommand,
  };
}
```

**Then simplify bridge**:

```typescript
// useSessionBridge.ts

import { useCommandExecutor } from './useCommandExecutor';

export function useSessionBridge() {
  const streaming = useStreamingConnection();
  const sessionManager = useSessionManager();
  const commandExecutor = useCommandExecutor();

  function handleChatMessage(message: any) {
    if (!message.message.startsWith('!')) return;

    const trigger = message.message.split(' ')[0].substring(1); // Remove '!'

    // Find active session
    const localSession = sessionManager.sessions.value.find(s => {
      if (s.status !== 'active') return false;

      return s.platforms.some(p =>
        p.platform === message.platform && p.stream_id
      );
    });

    if (!localSession) {
      console.log('⚠️ No active session for platform:', message.platform);
      return;
    }

    // Execute command
    commandExecutor.executeCommand(localSession.id, trigger, {
      userId: message.user_id,
      platform: message.platform,
      message: message.message,
    });
  }

  // ... rest of bridge code
}
```

---

### Priority 6: Add Integration Tests

**Issue**: No tests for critical flows

**Recommended Approach**: Test scenarios end-to-end

```typescript
// tests/integration/session-lifecycle.test.ts

import { describe, it, expect, beforeEach } from 'vitest';
import { useSessionManager } from '@/composables/streaming/useSessionManager';
import { useStreamingConnection } from '@/composables/streaming/useStreamingConnection';
import { useSessionBridge } from '@/composables/streaming/useSessionBridge';

describe('Session Lifecycle Integration', () => {
  let sessionManager;
  let streaming;
  let bridge;

  beforeEach(() => {
    sessionManager = useSessionManager();
    streaming = useStreamingConnection();
    bridge = useSessionBridge();

    // Clear state
    sessionManager.sessions.value = [];
    streaming.sessions.value.clear();
  });

  it('should activate local session when backend detects stream', async () => {
    // 1. Create local session
    const session = await sessionManager.createSession({
      name: 'Test Stream',
      platforms: [{ platform: 'twitch', account_id: 123 }],
    });

    expect(session.status).toBe('draft');

    // 2. Activate session
    await sessionManager.activateSession(session.id);

    const activated = sessionManager.sessions.value.find(s => s.id === session.id);
    expect(activated.status).toBe('waiting_for_live');

    // 3. Simulate backend session creation
    const backendSession = {
      id: 'backend-abc',
      platforms: [{ platform: 'twitch', account_id: 123 }],
      started_at: new Date().toISOString(),
    };

    bridge.handleBackendSession(backendSession.id, backendSession, 'created');

    // 4. Verify local session is now active
    const active = sessionManager.sessions.value.find(s => s.id === session.id);
    expect(active.status).toBe('active');
    expect(active.backend_session_id).toBe('backend-abc');
  });

  it('should execute command when chat message received', async () => {
    // Setup...
    const executeSpy = vi.spyOn(window, 'invoke');

    // Simulate chat message
    const chatMessage = {
      platform: 'twitch',
      message: '!alert',
      user_id: 'user123',
    };

    bridge.handleChatMessage(chatMessage);

    // Verify deck action was executed
    expect(executeSpy).toHaveBeenCalledWith('execute_deck_action', {
      actionId: expect.any(String),
    });
  });

  it('should auto-resume session after stream restarts', async () => {
    // Test auto-resume logic...
  });
});
```

---

## Summary

### What Works Well

1. **Clear architectural layers** - Backend, Bridge, Local separation is logical
2. **Event-driven design** - Scalable and extensible
3. **State machine** - Session lifecycle is well-defined
4. **Composable pattern** - Vue 3 composables are maintainable
5. **Offline-first local sessions** - Users can configure without internet

### What Needs Improvement

1. **Session matching logic** - Handle edge cases and conflicts
2. **Synchronization protocol** - Recover from offline scenarios
3. **Error boundaries** - Graceful degradation
4. **Data versioning** - Schema migration strategy
5. **Command execution** - Centralize and add cooldowns
6. **Testing** - Integration tests for critical flows

### Key Takeaways

The architecture is **fundamentally sound** but needs **refinement in edge cases**:
- The three-layer pattern is the right choice
- The bridge pattern effectively decouples backend and desktop
- The state machine could benefit from formal validation (XState)
- Need better synchronization protocol for reconnection scenarios
- Command execution should be extracted to dedicated composable

The design prioritizes **flexibility** and **separation of concerns** over rigid coupling, which is appropriate for a system that needs to evolve as new platforms and features are added.
