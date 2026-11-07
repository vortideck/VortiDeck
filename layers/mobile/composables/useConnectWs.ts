import { ref, onUnmounted, nextTick } from 'vue'
import { invoke } from "@tauri-apps/api/core"
import DeviceDetector from "device-detector-js"
import { useConnectionStore } from "@/stores/Connection"
import { useDeckStore } from "@/stores/Deck"
import { useApprovalsStore } from "@/stores/Approvals"
import { load } from '@tauri-apps/plugin-store'
import { useRouter } from '#app'
import { useE2EEKeys } from './useE2EEKeys'
import { usePairedDesktops } from './usePairedDesktops'
import { encodePublicKey, signMessage, verifyMessage, createHmacSignature, createRelayPayloadEnvelope } from '../utils/crypto'
import { v4 as uuidv4 } from 'uuid'
import canonicalize from 'canonicalize'
import {
  TAURI_STORE_KEYS,
  CONNECTION_MODES,
  CONNECTION_SETTINGS,
  CONNECTION_STATUSES,
  RELAY_MESSAGE_TYPES,
  MESSAGE_ACTIONS
} from '../constants/connection'
import { detectConnectionMode, updateConnectionInfo, shouldVerifySignature } from '../utils/connectionHelpers'
import type { WebSocketHandlerContext } from './useWebSocketHandlers'
import * as handlers from './useWebSocketHandlers'
import {
  connectionCandidateUrl,
  findReachableConnectionCandidate,
  normalizeConnectionCandidates,
} from '../utils/connectionCandidates'

// 🔒 SINGLETON: Global WebSocket instance to prevent multiple connections
let globalWebSocketInstance: any = null

interface ServiceInfo {
  name: string;
  websocket_url: string;
}

interface ServerInfo {
  id: string;
  name: string;
  type: string;
}

interface WebSocketMessage {
  action?: 'new_token' | 'active_deck' | 'all_decks' | 'server_info' | 'action_runtime_readiness' | 'component' | 'deck_updated' | 'volume_changed' | 'e2ee_handshake_complete' | 'e2ee_handshake_rejected' | 'relay_message' | 'deck_widget_snapshot' | 'deck_widget_delta' | 'deck_widget_state' | 'deck_widget_intent_ack' | 'deck_widget_definitions' | 'deck_widget_objects' | 'deck_widget_manifest';
  type?: 'relay_registered' | 'relay_from_desktop' | 'relay_message' | 'connected_to_desktop' | 'disconnected_from_desktop' | 'desktop_disconnected' | 'desktop_connected' | 'error' | 'available_desktops' | 'connection_established';  // Backend relay messages
  token?: string;
  desktop_public_key?: string; // 🔐 E2EE: Desktop's public key
  deck?: any;
  decks?: any;
  desktops?: any[];  // Backend relay: Available desktops list
  available_desktops?: any[];  // Backend relay: Available desktops list (alternative field name)
  server?: ServerInfo;
  readiness?: {
    status: 'initializing' | 'ready' | 'failed';
    phase: string;
    warnings: string[];
    error: string | null;
  };
  payload?: any;
  volume?: number;
  success?: boolean; // 🔐 E2EE: Handshake confirmation
  error?: string; // 🔐 E2EE: Handshake error
  signature?: string; // 🔐 E2EE: HMAC signature
  nonce?: string; // 🔐 E2EE: Replay prevention nonce
  relay_session_id?: string;  // Backend relay: Session ID
  message?: string;  // Backend relay: Error message
  user_id?: number;  // Backend relay: User ID
  email?: string;  // Backend relay: User email
  timestamp?: string;  // Backend relay: Timestamp
  desktop_metadata?: any;  // Backend relay: Desktop metadata (for desktop_connected)
}

export const useConnectWs = async (
  wsUrl?: string,
  key?: string,
  pairingToken?: string,
  desktopPublicKey?: string,
  desktopId?: string,
  webSocketUrlProvider?: () => Promise<string>,
  fallbackWebSocketUrlProvider?: () => Promise<string>,
) => {
  console.log('🔍 useConnectWs called with parameters:', {
    hasWsUrl: !!wsUrl,
    hasPairingKey: !!key,
    hasPairingToken: !!pairingToken,
    hasDesktopPublicKey: !!desktopPublicKey,
    desktopId: desktopId || 'UNDEFINED!'
  })

  // 📱 Initialize paired desktops composable FIRST (used in both singleton and new instance paths)
  const { updatePairedDesktop: updatePairedDesktopRegistry } = usePairedDesktops()

  // 💾 Load Tauri store FIRST (needed for both singleton and new instance paths)
  // Reuse existing store from singleton if available, otherwise create new
  const store = globalWebSocketInstance?.store || await load('store.json', { autoSave: false })

  // 🔌 Load saved wsUrl from connection store if not provided (for token reconnection)
  // MUST be done before singleton check so effectiveWsUrl is available
  const connectionStore = useConnectionStore()
  let effectiveWsUrl = wsUrl
  if (!effectiveWsUrl && !globalWebSocketInstance) {
    const savedWsUrl = connectionStore.wsUrl
    if (savedWsUrl) {
      effectiveWsUrl = savedWsUrl
      console.log('🔌 Loaded saved wsUrl from connection store:', effectiveWsUrl)
    } else {
      console.log('⚠️ No saved wsUrl found - will attempt connection anyway')
    }
  }

  // 🔒 SINGLETON: Return existing instance if already created
  if (globalWebSocketInstance) {
    console.log('🔒 Reusing existing WebSocket instance (singleton pattern)')
    if (webSocketUrlProvider) {
      globalWebSocketInstance.webSocketUrlProvider = webSocketUrlProvider
    } else if (effectiveWsUrl && detectConnectionMode(effectiveWsUrl) === CONNECTION_MODES.DIRECT) {
      globalWebSocketInstance.webSocketUrlProvider = undefined
    }
    globalWebSocketInstance.setFallbackWebSocketUrlProvider(fallbackWebSocketUrlProvider)
    globalWebSocketInstance.setDesktopId(desktopId)

    // 🔐 QR CODE SCAN: Handle fresh pairing FIRST (even if URL is same!)
    // This ensures we clear old tokens and reinitialize keys for every QR scan
    // Note: desktopId is only present in relay mode, so don't require it here
    if (key && pairingToken && desktopPublicKey) {
      console.log('🔐 QR CODE SCAN detected - performing fresh pairing...')
      console.log('🔍 QR Pairing parameters:', {
        pairingToken: !!pairingToken,
        desktopPublicKey: !!desktopPublicKey,
        desktopId: desktopId
      })

      const connectionStore = useConnectionStore()
      const connectionMode = detectConnectionMode(effectiveWsUrl || globalWebSocketInstance.currentWsUrl)
      const isRelayMode = connectionMode === CONNECTION_MODES.RELAY

      if (!isRelayMode) {
        const { initializeKeys, completeKeyExchange, getMobilePublicKey } = useE2EEKeys()

        // 🔐 ALWAYS generate NEW keys when scanning QR code
        console.log('🔐 Generating NEW E2EE keys for fresh QR pairing...')
        initializeKeys()
        const publicKey = getMobilePublicKey()
        console.log('🔐 E2EE keys initialized for QR pairing')
        console.log('🔐 Mobile public key generated:', !!publicKey)

        if (desktopId) {
          // 🗑️ CLEAR old token for this desktop (new pairing = fresh start!)
          console.log(`🗑️ Clearing old token for desktop ${desktopId} (new QR scan)`)
          await store.delete(TAURI_STORE_KEYS.TOKEN_FOR_DESKTOP(desktopId))
          await store.save()

          console.log('🔐 Desktop ID present - deriving and saving shared secret for:', desktopId)
          await completeKeyExchange(desktopPublicKey, desktopId)
          console.log('✅ E2EE shared secret derived and saved for desktop:', desktopId)

          // 📱 Register this desktop in paired desktops registry
          await updatePairedDesktopRegistry(desktopId, {
            desktop_name: `Desktop ${desktopId.substring(0, 8)}`,
            last_connected: new Date().toISOString(),
            has_token: false,  // Token comes later in handleNewToken
            has_e2ee_secret: true,
            connection_mode: 'direct',
            ws_url: wsUrl
          })
          console.log('📱 Registered paired desktop:', desktopId)
        } else {
          // Direct mode without desktop_id - derive shared secret without persisting
          console.log('🔐 Direct mode (no desktop_id) - deriving shared secret in-memory')
          await completeKeyExchange(desktopPublicKey)
          console.log('✅ E2EE shared secret derived (in-memory, no desktop_id)')
        }
      }

      // Update pairing parameters in instance for QR scan
      globalWebSocketInstance.pairingKey = key
      globalWebSocketInstance.storedPairingToken = pairingToken
      globalWebSocketInstance.storedDesktopPublicKey = desktopPublicKey
      globalWebSocketInstance.storedDesktopId = desktopId
    } else {
      // 🗑️ Clear old pairing params when NOT doing QR scan (reconnection scenario)
      console.log('🗑️ Clearing old pairing params (reconnection without QR scan)')
      globalWebSocketInstance.pairingKey = null
      globalWebSocketInstance.storedPairingToken = null
      globalWebSocketInstance.storedDesktopPublicKey = null
    }

    // If relay URL provided, update the stored URL and reconnect if needed
    if (effectiveWsUrl && effectiveWsUrl !== globalWebSocketInstance.currentWsUrl) {
      console.log(
        '🔄 WebSocket endpoint changed from',
        String(globalWebSocketInstance.currentWsUrl || '').split('?')[0],
        'to',
        effectiveWsUrl.split('?')[0],
      )
      globalWebSocketInstance.currentWsUrl = effectiveWsUrl
      // 🔥 CRITICAL: Update effectiveWsUrl in the instance so connectWebSocket uses the new URL
      globalWebSocketInstance.effectiveWsUrl = effectiveWsUrl

      // 🔐 Detect connection mode and update connection info atomically
      const connectionStore = useConnectionStore()
      const connectionMode = detectConnectionMode(effectiveWsUrl)
      const isRelayMode = connectionMode === CONNECTION_MODES.RELAY

      console.log(`🔌 Connection mode updated to: ${connectionMode}`)
      updateConnectionInfo(connectionStore, {
        wsUrl: effectiveWsUrl,
        connectionMode,
        activeDesktopId: desktopId
      })

      // Close existing connection (intentional close - don't trigger reconnection)
      if (globalWebSocketInstance.socket.value) {
        console.log('🔌 Closing existing WebSocket connection (intentional)...')
        globalWebSocketInstance.setIntentionalClose(true)  // 🔒 Set flag BEFORE closing
        globalWebSocketInstance.socket.value.close()
        globalWebSocketInstance.socket.value = null
      }

      // Reconnect to new URL
      console.log('🔌 Reconnecting to new URL...')
      await globalWebSocketInstance.connectWebSocket()
      globalWebSocketInstance.setIntentionalClose(false)  // 🔒 Reset flag after new connection established

      // If desktop ID provided, call connectToDesktop after connection established
      if (desktopId) {
        console.log('📞 Will call connectToDesktop after connection: ', desktopId)
        // Store desktop ID for later use after connection_established
        globalWebSocketInstance.pendingDesktopId = desktopId
      }
    }

    return globalWebSocketInstance
  }

  console.log('🆕 Creating new WebSocket instance (singleton)')

  // Note: store already loaded at line 81 before singleton check

  const socket = ref<WebSocket | null>(null)
  const isWebSocketConnected = ref(false)
  // connectionStore already declared at top of function
  const router = useRouter()
  const deviceDetector = new DeviceDetector()

  // 🔐 E2EE: Initialize E2EE keys management
  const { initializeKeys, completeKeyExchange, loadSharedSecret, getSharedSecret, getMobilePublicKey, isE2EEActive } = useE2EEKeys()

  const RECONNECT_INTERVAL = CONNECTION_SETTINGS.RECONNECT_INTERVAL
  let reconnectTimeout: ReturnType<typeof setTimeout> | null = null
  let isReconnecting = false
  let isIntentionalClose = false  // 🔒 Flag to prevent reconnection on intentional socket closes
  let pairingKey = key
  let storedPairingToken = pairingToken
  let storedDesktopPublicKey = desktopPublicKey
  let storedDesktopId = desktopId
  let currentWebSocketUrlProvider = webSocketUrlProvider
  let currentFallbackWebSocketUrlProvider = fallbackWebSocketUrlProvider
  let mobileClientId: string = '' // Will be initialized from Tauri store

  // 🔐 Load desktop_id from Tauri store if not provided (for token reconnection with E2EE)
  if (!storedDesktopId) {
    const savedDesktopId = await store.get(TAURI_STORE_KEYS.LAST_PAIRED_DESKTOP_ID)
    if (savedDesktopId && typeof savedDesktopId === 'string') {
      storedDesktopId = savedDesktopId
      console.log('🔐 Loaded desktop_id from Tauri store for E2EE reconnection:', storedDesktopId)
      // Also set it in connection store so UI components can access it
      connectionStore.setActiveDesktopId(storedDesktopId)
    } else {
      console.log('⚠️ No saved desktop_id found in Tauri store')
    }
  }

  // Generate or load stable client_id for this mobile device (Tauri store)
  const getStableClientId = async () => {
    let clientId = await store.get(TAURI_STORE_KEYS.MOBILE_CLIENT_ID)
    if (!clientId) {
      clientId = `mobile_${uuidv4()}`
      await store.set(TAURI_STORE_KEYS.MOBILE_CLIENT_ID, clientId)
      await store.save()
      console.log('🆔 Generated new mobile client_id:', clientId)
    } else {
      console.log('🆔 Loaded mobile client_id from Tauri store:', clientId)
    }
    return clientId as string
  }

  // effectiveWsUrl already loaded at top of function

  // Detect connection mode and update connection info atomically
  const connectionMode = detectConnectionMode(effectiveWsUrl)
  const isRelayMode = connectionMode === CONNECTION_MODES.RELAY

  console.log(`🔌 Connection mode: ${connectionMode}`)
  updateConnectionInfo(connectionStore, {
    wsUrl: effectiveWsUrl?.split('?')[0],
    connectionMode,
    activeDesktopId: desktopId
  })

  // 🔐 E2EE: Initialize keys ONLY for QR pairing (direct connection)
  // For relay mode, keys will be initialized when selecting a desktop (if needed)
  if (!isRelayMode && pairingToken && desktopPublicKey) {
    console.log('🔐 Initializing E2EE keys for QR pairing...')
    console.log('🔍 QR Pairing parameters:', {
      pairingToken: !!pairingToken,
      desktopPublicKey: !!desktopPublicKey,
      desktopId: desktopId || 'MISSING!'
    })

    // 🗑️ CLEAR old token for this desktop (new pairing = fresh start!)
    if (desktopId) {
      const oldToken = await store.get(TAURI_STORE_KEYS.TOKEN_FOR_DESKTOP(desktopId))
      if (oldToken) {
        console.log(`🗑️ Clearing old token for desktop ${desktopId} (new pairing detected)`)
        await store.delete(TAURI_STORE_KEYS.TOKEN_FOR_DESKTOP(desktopId))
        await store.save()
      }
    }

    initializeKeys()
    const publicKey = getMobilePublicKey()
    console.log('🔐 E2EE keys initialized for QR pairing')
    console.log('🔐 Mobile public key generated:', !!publicKey)

    // 🔐 IMMEDIATELY derive shared secret from desktop public key
    if (desktopId) {
      console.log('🔐 Desktop ID present - deriving and saving shared secret for:', desktopId)
      await completeKeyExchange(desktopPublicKey, desktopId)
      console.log('✅ E2EE shared secret derived and saved for desktop:', desktopId)
    } else {
      console.error('❌ Desktop ID missing! Cannot save shared secret for relay mode')
      console.log('🔐 Deriving shared secret without saving (legacy mode)')
      await completeKeyExchange(desktopPublicKey)
    }
  } else if (isRelayMode) {
    console.log('🔐 Relay mode: Keys will be initialized when selecting desktop (if not already paired)')
  } else {
    console.warn('⚠️ E2EE NOT initialized - conditions not met', { isRelayMode, pairingToken, hasDesktopPublicKey: !!desktopPublicKey })
  }

  const discoverService = async (): Promise<ServiceInfo[] | null> => {
    try {
      // Add timeout to prevent hanging
      const timeoutPromise = new Promise<never>((_, reject) => {
        setTimeout(() => reject(new Error("Service discovery timeout")), 5000) // 5 second timeout
      })
      
      const discoveryPromise = invoke<ServiceInfo[]>("discover_websocket")
      
      // Race between discovery and timeout
      const result = await Promise.race([discoveryPromise, timeoutPromise])
      console.log("Discovered services:", result)
      return result
    } catch (error) {
      console.error("Error discovering services:", error)
      // Don't block the app - return null and let it handle gracefully
      return null
    }
  }


  /**
   * Main WebSocket message handler - refactored with separate handler functions
   */
  const handleWebSocketMessage = async (
    event: MessageEvent,
    relayEnvelopeVerified = false,
  ) => {
    try {
      const message = JSON.parse(event.data) as WebSocketMessage
      console.log("Received message:", message)

      // Create handler context once
      const deckStore = useDeckStore()
      const approvalsStore = useApprovalsStore()
      const ctx: WebSocketHandlerContext = {
        connectionStore,
        deckStore,
        approvalsStore,
        getSharedSecret,
        getMobilePublicKey,
        isE2EEActive,
        completeKeyExchange,
        connectToDesktop,
        connectToPreferredDesktop,
        handleNewToken,
        handleServerInfo,
        handleVolumeChange,
        requestDeckWidgetSnapshot: () => sendSignedMessage({ action: "get_current_deck" }),
        sendSignedMessage,
        storedDesktopId,
        storedPairingToken,
        storedDesktopPublicKey,
      }

      // Handle relay-specific messages first
      if (message.type) {
        switch (message.type) {
          case RELAY_MESSAGE_TYPES.CONNECTION_ESTABLISHED:
            await handlers.handleConnectionEstablished(message, ctx)
            break
          case RELAY_MESSAGE_TYPES.RELAY_REGISTERED:
            handlers.handleRelayRegistered(message, ctx)
            break
          case RELAY_MESSAGE_TYPES.AVAILABLE_DESKTOPS:
            handlers.handleAvailableDesktops(message, ctx)
            break
          case RELAY_MESSAGE_TYPES.RELAY_FROM_DESKTOP:
            await handlers.handleRelayFromDesktop(message, ctx, handleWebSocketMessage)
            break
          case RELAY_MESSAGE_TYPES.RELAY_MESSAGE:
            await handlers.handleRelayMessage(message, ctx, handleWebSocketMessage)
            break
          case RELAY_MESSAGE_TYPES.CONNECTED_TO_DESKTOP:
            handlers.handleConnectedToDesktop(message, ctx)
            break
          case RELAY_MESSAGE_TYPES.DISCONNECTED_FROM_DESKTOP:
            handlers.handleDisconnectedFromDesktop(message, ctx)
            break
          case RELAY_MESSAGE_TYPES.DESKTOP_DISCONNECTED:
            handlers.handleDesktopDisconnected(message, ctx)
            break
          case RELAY_MESSAGE_TYPES.DESKTOP_CONNECTED:
            await handlers.handleDesktopConnected(message, ctx)
            break
          case RELAY_MESSAGE_TYPES.ERROR:
            handlers.handleRelayError(message, ctx)
            break
        }
        return
      }

      // 🔐 E2EE: Verify HMAC signature if we have a shared secret (direct mode)
      if (shouldVerifySignature(getSharedSecret(), message, relayEnvelopeVerified)) {
        const isValid = verifyMessage(getSharedSecret()!, message)
        if (!isValid) {
          console.error("🚫 Invalid HMAC signature - message rejected")
          return
        }
        console.log("✅ Verified HMAC signature for incoming message")
      }

      // Handle action-based messages
      switch (message.action) {
        case MESSAGE_ACTIONS.RELAY_MESSAGE:
          await handlers.handleRelayMessage(message, ctx, handleWebSocketMessage)
          break
        case MESSAGE_ACTIONS.NEW_TOKEN:
          await handlers.handleNewTokenAction(message, ctx, (key) => { pairingKey = key })
          break
        case MESSAGE_ACTIONS.E2EE_HANDSHAKE_REJECTED:
          handlers.handleE2EEHandshakeRejected(message, ctx)
          break
        case MESSAGE_ACTIONS.E2EE_HANDSHAKE_COMPLETE:
          await handlers.handleE2EEHandshakeComplete(message, ctx, () => {
            storedPairingToken = null
            storedDesktopPublicKey = null
          })
          break
        case MESSAGE_ACTIONS.ACTIVE_DECK:
          handlers.handleActiveDeck(message, ctx)
          break
        case MESSAGE_ACTIONS.DECK_UPDATED:
          handlers.handleDeckUpdated(message, ctx)
          break
        case MESSAGE_ACTIONS.ALL_DECKS:
          handlers.handleAllDecks(message, ctx)
          break
        case MESSAGE_ACTIONS.SERVER_INFO:
          handlers.handleServerInfoAction(message, ctx)
          break
        case MESSAGE_ACTIONS.ACTION_RUNTIME_READINESS:
          handlers.handleActionRuntimeReadiness(message, ctx)
          break
        case MESSAGE_ACTIONS.WORKFLOW_APPROVAL_REQUESTED:
          handlers.handleWorkflowApprovalRequested(message, ctx)
          break
        case MESSAGE_ACTIONS.WORKFLOW_APPROVALS_SNAPSHOT:
          handlers.handleWorkflowApprovalsSnapshot(message, ctx)
          break
        case MESSAGE_ACTIONS.WORKFLOW_APPROVAL_RESOLVED:
          handlers.handleWorkflowApprovalResolved(message, ctx)
          break
        case MESSAGE_ACTIONS.WORKFLOW_APPROVAL_DECISION_ACK:
          handlers.handleWorkflowApprovalDecisionAck(message, ctx)
          break
        case MESSAGE_ACTIONS.VOLUME_CHANGED:
          handlers.handleVolumeChangedAction(message, ctx)
          break
        case MESSAGE_ACTIONS.COMPONENT:
          handlers.handleComponentAction(message, ctx)
          break
        case MESSAGE_ACTIONS.DECK_WIDGET_SNAPSHOT:
          await handlers.handleDeckWidgetSnapshot(message as any, ctx)
          break
        case MESSAGE_ACTIONS.DECK_WIDGET_DELTA:
          await handlers.handleDeckWidgetDelta(message as any, ctx)
          break
        case MESSAGE_ACTIONS.DECK_WIDGET_STATE:
          await handlers.handleDeckWidgetState(message as any, ctx)
          break
        case MESSAGE_ACTIONS.DECK_WIDGET_INTENT_ACK:
          handlers.handleDeckWidgetIntentAck(message as any)
          break
        case MESSAGE_ACTIONS.DECK_WIDGET_DEFINITIONS:
          await handlers.handleDeckWidgetDefinitions(message as any, ctx)
          break
        case MESSAGE_ACTIONS.DECK_WIDGET_OBJECTS:
          await handlers.handleDeckWidgetObjects(message as any, ctx)
          break
        case MESSAGE_ACTIONS.DECK_WIDGET_MANIFEST:
          handlers.handleDeckWidgetManifest(message as any)
          break
        default:
          console.warn("Unknown message action:", message.action)
      }
    } catch (error) {
      console.error("Error handling WebSocket message:", error)
    }
  }

  const handleNewToken = async (token: string) => {
    if (typeof token !== 'string') {
      console.error('Invalid token received')
      return
    }

    // Get desktop_id from active connection
    const targetDesktopId = storedDesktopId || connectionStore.getActiveDesktopId
    if (!targetDesktopId) {
      console.error('❌ Cannot save token: desktop_id missing')
      return
    }

    console.log('🔑 Received new token from desktop:', targetDesktopId, `${token.substring(0, 8)}...`)

    // Save per-desktop token (NEW!)
    await store.set(TAURI_STORE_KEYS.TOKEN_FOR_DESKTOP(targetDesktopId), token)

    // Legacy: Also save to global token for backward compatibility
    localStorage.setItem("token", token)
    await store.set(TAURI_STORE_KEYS.TOKEN, token)

    await store.save()
    console.log(`✅ Token saved to Tauri store for desktop: ${targetDesktopId}`)
    console.log('✅ Authentication complete - E2EE already set up during QR scan')

    // 🔑 CRITICAL: Clear pairing key from singleton after successful token reception
    // This ensures future reconnections use the token instead of re-authenticating with secret
    if (globalWebSocketInstance) {
      globalWebSocketInstance.pairingKey = null
      console.log('🧹 Cleared pairing key from singleton - future reconnections will use token')
    }

    // Update paired desktops registry
    await updatePairedDesktopRegistry(targetDesktopId, {
      last_connected: new Date().toISOString(),
      has_token: true
    })
  }

  const handleServerInfo = async (server: ServerInfo) => {
    connectionStore.setServerInfo(server)
    connectionStore.setPeers(server)
    connectionStore.setStatus(CONNECTION_STATUSES.PAIRED)

    // Update desktop name with name from server info
    const desktopId = storedDesktopId || connectionStore.getActiveDesktopId
    if (desktopId && server.name) {
      await updatePairedDesktopRegistry(desktopId, {
        desktop_name: server.name
      })
      console.log(`📝 Updated desktop name to: ${server.name}`)
    }
  }

  const handleVolumeChange = (message: WebSocketMessage) => {
    const deckStore = useDeckStore()
    
    // Optimize: Use nextTick for batched updates and better performance
    nextTick(() => {
      deckStore.setSystemVolume(
        message.volume || 0, 
        message.payload?.is_button_change || false, 
        message.payload?.button_id || null
      )
    })
    
    console.log("📱 Volume updated on mobile:", {
      volume: message.volume,
      isButtonChange: message.payload?.is_button_change,
      buttonId: message.payload?.button_id
    })
  }

  const setupWebSocketEventHandlers = async (mobileClientId: string) => {
    if (!socket.value) return
    const configuredSocket = socket.value

    configuredSocket.onopen = async () => {
      if (socket.value !== configuredSocket) return
      console.log("WebSocket connection established")
      isWebSocketConnected.value = true
      connectionStore.setStatus(CONNECTION_STATUSES.CONNECTED)
      isReconnecting = false

      if (reconnectTimeout) {
        clearTimeout(reconnectTimeout)
        reconnectTimeout = null
      }

      const mode = connectionStore.getConnectionMode

      if (mode === 'relay') {
        // Relay mode: OAuth-authenticated connection
        console.log("📡 Relay mode connection established")
        // For OAuth relay connections, authentication happens via JWT in WebSocket URL
        // No need to send register_mobile - just wait for connection_established message
        console.log("📡 Waiting for connection_established message from relay server...")

        // Desktop selection will be handled by connectToDesktop() function
      } else {
        // Direct mode: Authenticate immediately
        const userAgent = navigator.userAgent
        const device = deviceDetector.parse(userAgent)

        // 🔑 Try to get token for active desktop first, fallback to global token
        let token = null
        if (storedDesktopId) {
          token = await store.get(TAURI_STORE_KEYS.TOKEN_FOR_DESKTOP(storedDesktopId))
          console.log(`🔑 Loaded per-desktop token for ${storedDesktopId}:`, token ? 'found' : 'not found')

          // Migrate legacy global token to per-desktop storage
          if (!token) {
            const globalToken = await store.get(TAURI_STORE_KEYS.TOKEN)
            if (globalToken) {
              token = globalToken
              await store.set(TAURI_STORE_KEYS.TOKEN_FOR_DESKTOP(storedDesktopId), globalToken)
              await store.save()
              console.log('📦 Migrated global token to per-desktop storage for:', storedDesktopId)
            }
          }
        }

        // Fallback to global token for backward compatibility
        if (!token) {
          token = await store.get(TAURI_STORE_KEYS.TOKEN)
          console.log('🔑 Using global token (legacy):', token ? 'found' : 'not found')
        }

        // 🔑 Read pairing key from instance (updated by QR scan) or local variable
        const currentPairingKey = globalWebSocketInstance?.pairingKey || pairingKey

        console.log('🔐 Authentication check:', {
          hasPairingKey: !!currentPairingKey,
          hasToken: !!token,
          tokenValue: token ? `${String(token).substring(0, 8)}...` : 'null',
          desktopId: storedDesktopId
        })

        if (currentPairingKey) {
          console.log('🔐 Authenticating with SECRET (first-time pairing)')

          // 🔐 Check if E2EE is active (shared secret derived from QR code)
          const sharedSecret = getSharedSecret()
          const mobilePublicKey = getMobilePublicKey()
          const currentPairingToken = globalWebSocketInstance?.storedPairingToken || pairingToken

          if (sharedSecret && mobilePublicKey) {
            // 🔐 E2EE ACTIVE: Send encrypted authenticate
            console.log('🔐 E2EE active - sending ENCRYPTED authenticate')

            // Prepare sensitive data to encrypt (only secret and device)
            const sensitivePayload = {
              device: device.device,
              secret: currentPairingKey
            }

            // Encrypt the sensitive payload
            const { encrypt } = await import('../utils/crypto')
            const encryptedPayload = encrypt(sharedSecret, JSON.stringify(sensitivePayload))

            // Send encrypted authenticate with public info in plaintext
            const authMessage = {
              action: "authenticate",
              clientId: mobileClientId,
              mobile_public_key: encodePublicKey(mobilePublicKey),  // Public key (plaintext)
              pairing_token: currentPairingToken,  // ✅ PLAINTEXT - desktop needs this to derive shared secret
              encrypted_payload: encryptedPayload  // Encrypted secret + device only
            }

            socket.value?.send(JSON.stringify(authMessage))
            console.log('✅ Sent encrypted authenticate message with plaintext pairing_token')
          } else {
            console.error('🚫 Pairing authentication rejected locally: E2EE key exchange is incomplete')
            connectionStore.setErrorMessage('Secure pairing failed. Scan a fresh desktop QR code and try again.')
            connectionStore.setStatus(CONNECTION_STATUSES.DISCONNECTED)
            socket.value?.close()
            return
          }
        } else if (token) {
          console.log('🔑 Authenticating with TOKEN (reconnection)')

          // 🔐 CRITICAL: Load E2EE shared secret for token reconnection
          // When reconnecting with saved token, we need to restore E2EE session
          console.log('🔐 Token reconnection check:', { storedDesktopId, isRelayMode })
          if (storedDesktopId && !isRelayMode) {
            console.log('🔐 Token reconnection - loading saved E2EE shared secret for:', storedDesktopId)
            try {
              const loaded = await loadSharedSecret(storedDesktopId)
              console.log('🔐 loadSharedSecret returned:', loaded)
              if (isE2EEActive()) {
                console.log('✅ E2EE shared secret loaded for token reconnection')
              } else {
                console.log('⚠️ No saved E2EE shared secret found for desktop:', storedDesktopId)
              }
            } catch (error) {
              console.error('❌ Failed to load E2EE shared secret:', error)
            }
          } else {
            console.log('⚠️ Skipping E2EE load - conditions not met:', { storedDesktopId: !!storedDesktopId, isRelayMode })
          }

          // 🔐 Check if E2EE is active (for token reconnection)
          const sharedSecret = getSharedSecret()

          if (sharedSecret) {
            // 🔐 E2EE ACTIVE: Send encrypted token authenticate
            console.log('🔐 E2EE active - sending ENCRYPTED token authenticate')

            // Prepare sensitive data to encrypt
            const sensitivePayload = {
              device: device.device,
              token: token
            }

            // Encrypt the sensitive payload
            const { encrypt } = await import('../utils/crypto')
            const encryptedPayload = encrypt(sharedSecret, JSON.stringify(sensitivePayload))

            // Send encrypted authenticate
            const authMessage = {
              action: "authenticate",
              clientId: mobileClientId,
              encrypted_payload: encryptedPayload  // Encrypted token + device
            }

            socket.value?.send(JSON.stringify(authMessage))
            console.log('✅ Sent encrypted token authenticate message')
          } else {
            console.error('🚫 Token authentication rejected locally: no persisted E2EE secret')
            connectionStore.setErrorMessage('Secure pairing is missing. Scan the desktop QR code again.')
            connectionStore.setStatus(CONNECTION_STATUSES.DISCONNECTED)
            socket.value?.close()
            return
          }
        } else {
          console.error('❌ No authentication credentials available!')
          console.error('❌ Cannot authenticate without secret or token')
          console.error('❌ Connection will be rejected by desktop')
          console.error('❌ Please scan QR code to pair with desktop')
          connectionStore.setStatus(CONNECTION_STATUSES.DISCONNECTED)
          socket.value?.close()
          return
        }

        // 🆕 AUTO-SUBSCRIPTION: Volume updates are now automatically enabled on connection
        // No manual subscription needed - handled by server-side auto-registration
        console.log("📱 Volume updates will be auto-enabled by server")
      }
    }

    configuredSocket.onerror = (error: Event) => {
      if (socket.value !== configuredSocket) return
      console.error("WebSocket error:", error)
      connectionStore.setStatus(CONNECTION_STATUSES.DISCONNECTED)
     
      handleReconnection()
    }

    configuredSocket.onclose = () => {
      if (socket.value !== configuredSocket) return
      console.log("WebSocket connection closed")

      isWebSocketConnected.value = false
      connectionStore.setStatus(CONNECTION_STATUSES.DISCONNECTED)

      // 🔒 Don't reconnect if this was an intentional close (URL change, etc.)
      if (!isIntentionalClose) {
        handleReconnection()
      } else {
        console.log("🔒 Intentional close - skipping reconnection")
      }
    }

    configuredSocket.onmessage = handleWebSocketMessage
  }

  // 📤 Send signed message with E2EE
  // RELAY MODE FLOW:
  // 1. connect_to_desktop -> sent DIRECTLY to relay server (NOT wrapped)
  // 2. All other messages -> wrapped in relay_message envelope
  const sendSignedMessage = async (message: any) => {
    if (!socket.value) {
      console.error('❌ Socket is null - cannot send message')
      return
    }

    // 🔌 Wait for socket to be in OPEN state before sending
    if (socket.value.readyState === WebSocket.CONNECTING) {
      console.log('⏳ Socket is CONNECTING, waiting for OPEN state...')
      await new Promise<void>((resolve) => {
        const checkInterval = setInterval(() => {
          if (socket.value?.readyState === WebSocket.OPEN) {
            clearInterval(checkInterval)
            console.log('✅ Socket now in OPEN state')
            resolve()
          } else if (!socket.value || socket.value.readyState === WebSocket.CLOSED || socket.value.readyState === WebSocket.CLOSING) {
            clearInterval(checkInterval)
            console.error('❌ Socket closed while waiting')
            resolve()
          }
        }, 50) // Check every 50ms
      })
    }

    // Double-check socket is still valid and open
    if (!socket.value || socket.value.readyState !== WebSocket.OPEN) {
      console.error("Socket not in OPEN state, cannot send message")
      return
    }

    const sharedSecret = getSharedSecret()
    const mode = connectionStore.getConnectionMode
    const e2eeActive = isE2EEActive()

    console.log('📨 [sendSignedMessage] Status:', {
      hasSocket: !!socket.value,
      hasSharedSecret: !!sharedSecret,
      e2eeActive,
      mode,
      messageAction: message.action
    })

    if (sharedSecret && e2eeActive) {
      if (mode === 'relay') {
        // SPECIAL CASE: connect_to_desktop must be sent directly to relay server, NOT wrapped in relay_message
        if (message.action === 'connect_to_desktop') {
          // Add signature and nonce at message level
          const nonce = uuidv4()
          const messageWithNonce = { ...message, nonce }
          // ⭐ Use RFC 8785 canonical JSON for deterministic serialization
          const messageStr = canonicalize(messageWithNonce)!
          const signature = createHmacSignature(sharedSecret, messageStr)

          const signedMessage = {
            ...message,
            signature,
            nonce,
            client_id: mobileClientId
          }
          socket.value.send(JSON.stringify(signedMessage))
          console.log("📤 Sent connect_to_desktop (E2EE signed, canonical, direct to relay)")
        } else {
          // All application messages use a versioned, expiring relay envelope.
          const envelope = createRelayPayloadEnvelope(sharedSecret, message)
          const relayMessage = {
            action: 'relay_message',
            ...envelope,
            client_id: mobileClientId  // Include client_id for desktop to lookup shared secret
          }
          socket.value.send(JSON.stringify(relayMessage))
          console.log("📤 Sent relay_message (E2EE signed, canonical):", message.action)
        }
      } else {
        // Direct mode: sign entire message (uses canonical JSON via signMessage helper)
        const signedMessage = signMessage(sharedSecret, message)
        socket.value.send(JSON.stringify(signedMessage))
        console.log("📤 Sent direct message (E2EE signed, canonical):", message.action)
      }
    } else {
      // 🔐 SECURITY: E2EE not active - reject message instead of sending unsigned
      // After initial pairing, all messages MUST be signed to prevent MITM attacks
      console.error("❌ Cannot send message: E2EE not active!")
      console.error("   Message action:", message.action)
      console.error("   Mode:", mode)
      console.error("   This indicates a race condition - message sent before E2EE loaded")
      console.error("   🚫 REJECTING unsigned message to prevent security breach")

      // DO NOT send unsigned messages after E2EE has been established
      // This prevents race conditions and maintains security
      return
    }
  }

  const handleReconnection = () => {
    if (isReconnecting) return

    isReconnecting = true

    if (
      connectionStore.getConnectionMode === CONNECTION_MODES.DIRECT
      && connectionStore.getConnectionPreference !== 'direct'
      && currentFallbackWebSocketUrlProvider
    ) {
      console.log('☁️ Direct endpoint failed; switching to authenticated relay fallback')
      currentWebSocketUrlProvider = currentFallbackWebSocketUrlProvider
      currentFallbackWebSocketUrlProvider = undefined
      effectiveWsUrl = ''
      if (globalWebSocketInstance) {
        globalWebSocketInstance.webSocketUrlProvider = currentWebSocketUrlProvider
        globalWebSocketInstance.fallbackWebSocketUrlProvider = undefined
        globalWebSocketInstance.effectiveWsUrl = ''
      }
    }

    if (reconnectTimeout) {
      clearTimeout(reconnectTimeout)
    }

    reconnectTimeout = setTimeout(async () => {
      console.log("Attempting to reconnect...")
      console.log("🔍 Reconnection context:", {
        hasWsUrl: !!wsUrl,
        isRelayMode,
        connectionMode
      })
      await connectWebSocket()
      isReconnecting = false
    }, RECONNECT_INTERVAL)
  }

  const connectWebSocket = async () => {

    try {

      if (socket.value && (
        socket.value.readyState === WebSocket.OPEN
        || socket.value.readyState === WebSocket.CONNECTING
      )) {
        console.log('🔌 WebSocket is already connected or connecting')
        return
      }


      let websocketUrl: string

      const urlProvider = globalWebSocketInstance?.webSocketUrlProvider || currentWebSocketUrlProvider
      if (urlProvider) {
        effectiveWsUrl = await urlProvider()
        const providerMode = detectConnectionMode(effectiveWsUrl)
        updateConnectionInfo(connectionStore, {
          wsUrl: effectiveWsUrl.split('?')[0],
          connectionMode: providerMode,
          activeDesktopId: storedDesktopId,
        })
        if (globalWebSocketInstance) {
          globalWebSocketInstance.effectiveWsUrl = effectiveWsUrl
          globalWebSocketInstance.currentWsUrl = effectiveWsUrl
        }
      }

      // A relay URL provider returns a fresh one-use ticket for every attempt.
      const urlToUse = globalWebSocketInstance?.effectiveWsUrl || effectiveWsUrl || connectionStore.getWsUrl

      if (urlToUse) {
        websocketUrl = urlToUse
        console.log('🔌 Using WebSocket URL:', urlToUse.includes('relay') ? 'RELAY MODE' : 'DIRECT MODE')
        console.log('🔍 WebSocket URL:', websocketUrl.split('?')[0]) // Hide token
      } else {
        // ⚠️ CRITICAL: If we're in relay mode, do NOT fall back to local discovery
        const storedConnectionMode = connectionStore.getConnectionMode
        if (storedConnectionMode === 'relay') {
          console.error('❌ Relay mode active but wsUrl lost - cannot reconnect!')
          console.error('🔍 Debug info:', {
            paramWsUrl: !!wsUrl,
            storedWsUrl: !!connectionStore.getWsUrl,
            connectionMode: storedConnectionMode
          })
          connectionStore.setStatus(CONNECTION_STATUSES.DISCONNECTED)
          return
        }

        console.log('🔌 No URL provided, discovering local services...')
        const serviceInfos = await discoverService()
        if (!serviceInfos?.length) {
            console.warn("⚠️ No WebSocket services found during discovery")
            connectionStore.setStatus(CONNECTION_STATUSES.DISCONNECTED)
            handleReconnection()
            return
        }

        const serviceInfo = serviceInfos[0]
        if (!serviceInfo?.websocket_url) {
            console.warn("⚠️ Invalid WebSocket URL from service discovery")
            connectionStore.setStatus(CONNECTION_STATUSES.DISCONNECTED)
            handleReconnection()
            return
        }
        websocketUrl = serviceInfo.websocket_url
        console.log('🔍 Discovered local WebSocket URL:', websocketUrl)
      }

      console.log(`🔌 Creating WebSocket connection to: ${websocketUrl.includes('wss://') ? 'RELAY SERVER' : 'LOCAL DESKTOP'}`)
      socket.value = new WebSocket(websocketUrl)

      // Initialize and store mobile client ID from Tauri store
      mobileClientId = await getStableClientId()
      console.log('🆔 Using mobile client ID:', mobileClientId)
      await setupWebSocketEventHandlers(mobileClientId)


      
    } catch (error) {
      console.error("WebSocket connection error:", error)
      console.warn("⚠️ WebSocket connection failed, will retry:", error)
      connectionStore.setStatus(CONNECTION_STATUSES.DISCONNECTED)
      handleReconnection()
    }
  }

//   onUnmounted(() => {
//     if (reconnectTimeout) {
//       clearTimeout(reconnectTimeout)
//     }
//     socket.value?.close()
//   })

 

  const connectToDesktop = async (desktopId: string) => {
    storedDesktopId = desktopId
    if (globalWebSocketInstance) globalWebSocketInstance.storedDesktopId = desktopId
    if (!socket.value) {
      console.error("Socket not connected")
      return
    }

    // 🔌 Wait for socket to be in OPEN state before sending
    if (socket.value.readyState === WebSocket.CONNECTING) {
      console.log('⏳ Socket is CONNECTING, waiting for OPEN state...')
      await new Promise<void>((resolve) => {
        const checkInterval = setInterval(() => {
          if (socket.value?.readyState === WebSocket.OPEN) {
            clearInterval(checkInterval)
            console.log('✅ Socket now in OPEN state')
            resolve()
          } else if (!socket.value || socket.value.readyState === WebSocket.CLOSED || socket.value.readyState === WebSocket.CLOSING) {
            clearInterval(checkInterval)
            console.error('❌ Socket closed while waiting')
            resolve()
          }
        }, 50) // Check every 50ms
      })
    }

    // Double-check socket is still valid and open
    if (!socket.value || socket.value.readyState !== WebSocket.OPEN) {
      console.error("Socket not in OPEN state, cannot send message")
      return
    }

    console.log(`🖥️ Selecting desktop: ${desktopId}`)

    // 🔐 Try to load saved shared secret (no handshake needed!)
    const secretLoaded = await loadSharedSecret(desktopId)

    if (secretLoaded) {
      console.log('✅ E2EE shared secret loaded - secure communication ready!')
      console.log('🔐 No handshake needed - using saved shared secret')
    } else {
      console.error(`🚫 Relay connection rejected: no shared secret for ${desktopId}`)
      connectionStore.setErrorMessage('Relay requires prior secure QR pairing on the local network.')
      connectionStore.setStatus(CONNECTION_STATUSES.DISCONNECTED)
      return
    }

    // Send connect_to_desktop message (sign if E2EE is active)
    const connectMessage = {
      action: 'connect_to_desktop',
      desktop_id: desktopId
    }

    await sendSignedMessage(connectMessage)
    console.log('✅ Desktop selection sent (E2EE signed)')

    connectionStore.setActiveDesktopId(desktopId)
    connectionStore.setActiveConnectionTransport('relay')
    connectionStore.setActiveConnectionLatencyMs(null)
    connectionStore.setRelayFallbackAvailable(true)

    // 💾 CRITICAL: Save desktop_id for auto-reconnect after disconnect
    await store.set(TAURI_STORE_KEYS.LAST_PAIRED_DESKTOP_ID, desktopId)
    console.log('💾 Saved desktop_id for future auto-reconnect:', desktopId)
  }

  const connectToPreferredDesktop = async (desktop: any) => {
    const desktopId = desktop?.desktop_id
    if (typeof desktopId !== 'string' || !desktopId) {
      throw new Error('Desktop identity is missing')
    }

    const preference = connectionStore.getConnectionPreference
    connectionStore.setActiveConnectionTransport(null)
    connectionStore.setActiveConnectionLatencyMs(null)
    const relayUrlProvider = globalWebSocketInstance?.fallbackWebSocketUrlProvider
      || globalWebSocketInstance?.webSocketUrlProvider
    connectionStore.setRelayFallbackAvailable(typeof relayUrlProvider === 'function')

    if (preference === 'relay') {
      console.log('☁️ Relay-only connection preference selected')
      await connectToDesktop(desktopId)
      return
    }

    const candidates = normalizeConnectionCandidates(
      desktop?.metadata?.connection_candidates,
    )
    const reachable = await findReachableConnectionCandidate(candidates)
    if (!reachable) {
      if (preference === 'direct') {
        const message = 'No direct LAN or Tailscale endpoint is reachable. Relay fallback is disabled.'
        connectionStore.setErrorMessage(message)
        throw new Error(message)
      }

      console.log('☁️ No direct endpoint responded; using relay')
      await connectToDesktop(desktopId)
      return
    }

    const directUrl = connectionCandidateUrl(reachable.candidate)
    console.log(`⚡ Using reachable ${reachable.candidate.transport} endpoint (${reachable.latencyMs} ms)`)
    const directConnection = await useConnectWs(
      directUrl,
      undefined,
      undefined,
      undefined,
      desktopId,
      undefined,
      preference === 'automatic' ? relayUrlProvider : undefined,
    )
    await directConnection.connectWebSocket()
    connectionStore.setActiveConnectionTransport(reachable.candidate.transport)
    connectionStore.setActiveConnectionLatencyMs(reachable.latencyMs)
  }

  const setDesktopId = (desktopId?: string) => {
    if (desktopId) storedDesktopId = desktopId
  }

  const setFallbackWebSocketUrlProvider = (provider?: () => Promise<string>) => {
    currentFallbackWebSocketUrlProvider = provider
  }

  const setIntentionalClose = (value: boolean) => {
    isIntentionalClose = value
  }

  // 🔒 Create the instance object
  const instance = {
    socket,
    isWebSocketConnected,
    connectWebSocket,
    sendSignedMessage,
    connectToDesktop,
    connectToPreferredDesktop,
    setDesktopId,
    setFallbackWebSocketUrlProvider,
    setIntentionalClose,
    discoverService,  // Export for local desktop discovery
    store,  // Store for accessing Tauri storage (needed for token deletion in QR scan)
    currentWsUrl: effectiveWsUrl,  // Track current URL for singleton comparison
    effectiveWsUrl: effectiveWsUrl,  // CRITICAL: Store for connectWebSocket to use
    pendingDesktopId: undefined as string | undefined,  // Store desktop ID for deferred connection
    pairingKey,  // Store pairing key (secret from QR code)
    storedPairingToken,  // Store pairing token for E2EE handshake
    storedDesktopPublicKey,  // Store desktop public key for E2EE
    storedDesktopId,  // Store desktop ID for relay mode
    webSocketUrlProvider: currentWebSocketUrlProvider,
    fallbackWebSocketUrlProvider: currentFallbackWebSocketUrlProvider,
    isIntentionalClose  // 🔒 Flag to prevent reconnection on intentional socket closes
  }

  // 🔒 SINGLETON: Store as global instance
  globalWebSocketInstance = instance

  return instance
}
