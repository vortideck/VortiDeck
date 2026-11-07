/**
 * WebSocket message handlers - extracted from useConnectWs
 * Each handler is responsible for one message type
 */

import { RELAY_MESSAGE_TYPES, MESSAGE_ACTIONS } from '../constants/connection'
import type { useConnectionStore } from '@/stores/Connection'
import type { useDeckStore } from '@/stores/Deck'
import type { useApprovalsStore } from '@/stores/Approvals'
import { verifyRelayPayloadEnvelope } from '../utils/crypto'
import { useSyncedDeckWidgets } from '@base/composables/useSyncedDeckWidgets'
import { useWidgetRegistry } from '@base/composables/useWidgetRegistry'
import { usePortableWidgetAssets } from '@base/composables/usePortableWidgetAssets'
import { useMainStore } from '@base/stores/Main'
import type {
  DeckWidgetDefinitionsMessage,
  DeckWidgetObjectsMessage,
  DeckWidgetManifestMessage,
  DeckWidgetDeltaMessage,
  DeckWidgetIntentAckMessage,
  DeckWidgetSnapshotMessage,
  DeckWidgetStateMessage,
} from '@base/types/deckWidgetProtocol'

const activeWidgetManifests = new Map<number, {
  definitionsByHash: Map<string, string>
  assetsByHash: Map<string, string>
}>()

export interface WebSocketHandlerContext {
  connectionStore: ReturnType<typeof useConnectionStore>
  deckStore: ReturnType<typeof useDeckStore>
  approvalsStore: ReturnType<typeof useApprovalsStore>
  getSharedSecret: () => Uint8Array | null
  getMobilePublicKey: () => string | null
  isE2EEActive: () => boolean
  completeKeyExchange: (desktopPublicKeyB64: string, desktopId?: string | null) => Promise<void>
  connectToDesktop: (desktopId: string) => Promise<void>
  connectToPreferredDesktop: (desktop: any) => Promise<void>
  handleNewToken: (token: string) => Promise<void>
  handleServerInfo: (server: any) => void
  handleVolumeChange: (message: any) => void
  requestDeckWidgetSnapshot: () => void | Promise<void>
  sendSignedMessage: (message: unknown) => void | Promise<void>
  storedDesktopId: string | undefined | null
  storedPairingToken: string | undefined | null
  storedDesktopPublicKey: string | undefined | null
}

export function handleWorkflowApprovalRequested(message: any, ctx: WebSocketHandlerContext) {
  if (message.protocolVersion !== 1 || !message.approval?.id) return;
  ctx.approvalsStore.upsert(message.approval);
}

export function handleWorkflowApprovalsSnapshot(message: any, ctx: WebSocketHandlerContext) {
  if (message.protocolVersion !== 1 || !Array.isArray(message.approvals)) return;
  ctx.approvalsStore.replace(message.approvals);
}

export function handleWorkflowApprovalResolved(message: any, ctx: WebSocketHandlerContext) {
  if (typeof message.wakeId === "string") ctx.approvalsStore.remove(message.wakeId);
}

export function handleWorkflowApprovalDecisionAck(message: any, ctx: WebSocketHandlerContext) {
  if (message.error) {
    console.error("Workflow approval decision failed:", message.error);
    return;
  }
  if (typeof message.wakeId === "string") ctx.approvalsStore.remove(message.wakeId);
}

/**
 * Handle 'connection_established' relay message
 */
export async function handleConnectionEstablished(
  message: any,
  ctx: WebSocketHandlerContext
) {
  console.log("✅ Connection established to relay server")

  if (message.user_id) {
    console.log(`📡 User ID: ${message.user_id}, Email: ${message.email}`)
  }

  // Handle available desktops
  if (message.available_desktops) {
    ctx.connectionStore.setAvailableDesktops(message.available_desktops)
    console.log(`✅ Found ${message.available_desktops.length} desktops`)

    // Auto-reconnect to last used desktop if saved
    if (ctx.storedDesktopId) {
      console.log(`🔄 Auto-reconnecting to last used desktop: ${ctx.storedDesktopId}`)

      const desktop = message.available_desktops.find(
        (d: any) => d.desktop_id === ctx.storedDesktopId
      )

      if (desktop) {
        console.log(`✅ Desktop found in available list - selecting the best transport`)

        // 🐛 FIX: Add small delay to ensure WebSocket is fully ready (OPEN state)
        // Race condition: connection_established arrives before socket.readyState === OPEN
        setTimeout(async () => {
          try {
            await ctx.connectToPreferredDesktop(desktop)
          } catch (error) {
            console.error(`❌ Auto-reconnect failed:`, error)
          }
        }, 150) // 150ms delay to ensure socket is ready
      } else {
        console.warn(`⚠️ Last used desktop (${ctx.storedDesktopId}) not found - user must select manually`)
      }
    } else {
      console.log(`ℹ️ No saved desktop_id - user must select desktop manually`)
    }
  }
}

/**
 * Handle 'relay_registered' message
 */
export function handleRelayRegistered(message: any, ctx: WebSocketHandlerContext) {
  console.log("✅ Mobile registered for backend relay")

  if (message.relay_session_id) {
    console.log("📡 Relay Session ID:", message.relay_session_id)
    ctx.connectionStore.setRelaySessionId(message.relay_session_id)
  }
}

/**
 * Handle 'available_desktops' message
 */
export function handleAvailableDesktops(message: any, ctx: WebSocketHandlerContext) {
  console.log("📋 Received available desktops list")

  const desktops = message.desktops || message.available_desktops
  if (desktops) {
    ctx.connectionStore.setAvailableDesktops(desktops)
    console.log(`✅ Found ${desktops.length} desktops`)
  }
}

/**
 * Handle 'relay_from_desktop' message with signature verification
 */
export async function handleRelayFromDesktop(
  message: any,
  ctx: WebSocketHandlerContext,
  handleWebSocketMessage: (event: MessageEvent, relayEnvelopeVerified?: boolean) => Promise<void>
) {
  console.log("📥 Message from desktop via relay")

  if (!message.payload) {
    console.warn("⚠️ relay_from_desktop missing payload")
    return
  }

  // Verify E2EE signature
  const sharedSecret = ctx.getSharedSecret()
  if (!sharedSecret || !message.signature || !message.nonce) {
    console.error("🚫 Unsigned relay payload rejected: an active E2EE session, signature, and nonce are required")
    return
  }

  if (!verifyRelayPayloadEnvelope(sharedSecret, message)) {
    console.error("🚫 Invalid or replayed HMAC relay message rejected")
    return
  }

  // Process the unwrapped message
  await handleWebSocketMessage({
    data: JSON.stringify(message.payload)
  } as MessageEvent, true)
}

/**
 * Handle 'connected_to_desktop' message with E2EE status check
 */
export function handleConnectedToDesktop(message: any, ctx: WebSocketHandlerContext) {
  console.log("🖥️ Desktop connected to relay!")

  const mobilePublicKey = ctx.getMobilePublicKey()
  const currentMode = ctx.connectionStore.getConnectionMode
  const isCurrentlyRelayMode = currentMode === 'relay'
  const e2eeAlreadyActive = ctx.isE2EEActive()

  console.log('🔐 E2EE Handshake check:', {
    hasMobilePublicKey: !!mobilePublicKey,
    isRelayMode: isCurrentlyRelayMode,
    e2eeActive: e2eeAlreadyActive
  })

  // Set relay status and check E2EE atomically to avoid UI flash
  ctx.connectionStore.setDesktopRelayStatus('connected')

  // Check E2EE status
  if (e2eeAlreadyActive) {
    console.log('✅ E2EE already active (loaded from storage) - setting status to paired')
    ctx.connectionStore.setStatus('paired')
    ctx.connectionStore.setErrorMessage(null) // Clear any previous error
  } else if (isCurrentlyRelayMode) {
    // Relay mode without saved shared secret = not paired yet
    console.error('❌ Cannot use relay without prior E2EE pairing')
    console.error('🔒 E2EE keys must be exchanged via QR code pairing (direct connection)')
    console.error('📱 Please scan QR code while on same network as desktop')
    ctx.connectionStore.setStatus('error')
    ctx.connectionStore.setErrorMessage('E2EE pairing required. Please scan the QR code while on the same network as your desktop to establish a secure connection.')
  } else {
    // Direct connection mode
    console.warn('⚠️ Connected without E2EE - unusual state')
    ctx.connectionStore.setStatus('paired')
    ctx.connectionStore.setErrorMessage(null) // Clear any previous error
  }
}

/**
 * Handle 'disconnected_from_desktop' message
 */
export function handleDisconnectedFromDesktop(message: any, ctx: WebSocketHandlerContext) {
  console.log("🖥️ Desktop disconnected from relay")
  ctx.connectionStore.setDesktopRelayStatus('disconnected')
  ctx.connectionStore.setStatus('disconnected')
  ctx.connectionStore.setErrorMessage(null) // Clear error - normal disconnect
  console.log("⚠️ Desktop disconnected - status set to 'disconnected'")
}

/**
 * Handle 'desktop_disconnected' message
 */
export function handleDesktopDisconnected(message: any, ctx: WebSocketHandlerContext) {
  console.log("🖥️ Desktop went offline")
  ctx.connectionStore.setDesktopRelayStatus('disconnected')
  ctx.connectionStore.setStatus('disconnected')
  ctx.connectionStore.setErrorMessage(null) // Clear error - normal disconnect
  // ModalReconnecting will show automatically via clean.vue layout
}

/**
 * Handle 'desktop_connected' message - Desktop reconnected to relay
 */
export async function handleDesktopConnected(message: any, ctx: WebSocketHandlerContext) {
  console.log("🖥️ Desktop reconnected to relay!")
  console.log("📡 Desktop metadata:", message.desktop_metadata)

  // Check if E2EE is active BEFORE setting relay status to avoid UI flash
  const e2eeActive = ctx.isE2EEActive()

  if (e2eeActive) {
    console.log('✅ E2EE already active - setting status to paired')
    ctx.connectionStore.setDesktopRelayStatus('connected')
    ctx.connectionStore.setStatus('paired')
    ctx.connectionStore.setErrorMessage(null) // Clear any previous error

    // 🔌 CRITICAL FIX: Re-establish connection to desktop after it reconnects
    // When desktop disconnects and reconnects, backend loses mobile-to-desktop association
    // We must send connect_to_desktop again to resume message routing
    const activeDesktopId = ctx.connectionStore.getActiveDesktopId
    if (activeDesktopId) {
      console.log(`🔄 Re-establishing connection to desktop: ${activeDesktopId}`)
      try {
        await ctx.connectToDesktop(activeDesktopId)
        console.log('✅ Successfully reconnected to desktop')
      } catch (error) {
        console.error('❌ Failed to reconnect to desktop:', error)
      }
    } else {
      console.warn('⚠️ No active desktop ID - cannot reconnect')
    }
  } else {
    // Relay mode without saved shared secret = not paired
    console.error('❌ Desktop reconnected but E2EE not active')
    console.error('🔒 E2EE keys must be exchanged via QR code pairing (direct connection)')
    console.error('📱 Please scan QR code while on same network as desktop')
    ctx.connectionStore.setDesktopRelayStatus('connected')
    ctx.connectionStore.setStatus('error')  // Set to 'error' to trigger modal
    ctx.connectionStore.setErrorMessage('E2EE pairing required. Please scan the QR code while on the same network as your desktop to establish a secure connection.')
  }
}

/**
 * Handle relay error messages
 */
export function handleRelayError(message: any, ctx: WebSocketHandlerContext) {
  console.error("❌ Relay error:", message.message || message.error)

  // Handle pairing required error (desktop doesn't have shared secret)
  if (message.error_code === 'PAIRING_REQUIRED' || message.message === 'E2EE pairing required') {
    console.error("❌ Desktop rejected relay messages: No shared secret found")
    console.error("🔒 Desktop does not have E2EE keys for this mobile device")
    console.error("📱 User must re-pair by scanning QR code on same network")
    ctx.connectionStore.setStatus('error')
    ctx.connectionStore.setErrorMessage('Pairing required. Desktop does not recognize this device. Please scan the QR code while on the same network to re-establish pairing.')
    return
  }

  // Handle desktop offline error
  if (message.message === 'Desktop not connected or offline') {
    ctx.connectionStore.setDesktopRelayStatus('disconnected')
    ctx.connectionStore.setStatus('disconnected')
    // ModalReconnecting will show automatically via clean.vue layout
  }
}

/**
 * Handle 'relay_message' action (desktop to mobile)
 */
export async function handleRelayMessage(
  message: any,
  ctx: WebSocketHandlerContext,
  handleWebSocketMessage: (event: MessageEvent, relayEnvelopeVerified?: boolean) => Promise<void>
) {
  if (!message.payload) {
    console.warn("⚠️ relay_message missing payload")
    return
  }

  // Verify E2EE signature
  const sharedSecret = ctx.getSharedSecret()

  if (!sharedSecret || !message.signature || !message.nonce) {
    console.error("🚫 Unsigned relay payload rejected: E2EE secret, signature, and nonce are required")
    return
  }

  if (!verifyRelayPayloadEnvelope(sharedSecret, message)) {
    console.error("🚫 Invalid or replayed HMAC relay message rejected")
    return
  }
  console.log("✅ Relay message signature verified (canonical)")

  // Process the unwrapped message
  await handleWebSocketMessage({
    data: JSON.stringify(message.payload)
  } as MessageEvent, true)
}

/**
 * Handle 'new_token' action
 */
export async function handleNewTokenAction(
  message: any,
  ctx: WebSocketHandlerContext,
  setPairingKey: (key: string | null) => void
) {
  // Check if payload is encrypted
  if (message.encrypted_payload) {
    console.log('🔐 Encrypted new_token received - decrypting...')

    const sharedSecret = ctx.getSharedSecret()
    if (!sharedSecret) {
      console.error('❌ No shared secret available to decrypt new_token')
      return
    }

    try {
      const { decrypt } = await import('../utils/crypto')
      const decryptedPayload = decrypt(sharedSecret, message.encrypted_payload)
      const decrypted = JSON.parse(decryptedPayload)

      console.log('✅ Successfully decrypted new_token')

      if (decrypted.token) {
        await ctx.handleNewToken(decrypted.token)
        setPairingKey(null) // Clear pairing key after successful token

        // Handle desktop_public_key if present (for E2EE completion)
        if (decrypted.desktop_public_key) {
          console.log('🔐 Desktop public key received in encrypted new_token')
        }
      }
    } catch (error) {
      console.error('❌ Failed to decrypt new_token:', error)
    }
  } else if (message.token) {
    // Plaintext token (legacy/unencrypted mode)
    await ctx.handleNewToken(message.token)
    setPairingKey(null) // Clear pairing key after successful token
  }
}

/**
 * Handle 'e2ee_handshake_rejected' action
 */
export function handleE2EEHandshakeRejected(message: any, ctx: WebSocketHandlerContext) {
  console.error("❌ E2EE handshake rejected by desktop")
  console.error("🔒 Reason:", message.error || "Unknown")
  console.error("📱 You must pair via QR code while on same network as desktop")
  ctx.connectionStore.setStatus('error')
}

/**
 * Handle 'e2ee_handshake_complete' action
 */
export async function handleE2EEHandshakeComplete(
  message: any,
  ctx: WebSocketHandlerContext,
  clearPairingData: () => void
) {
  // Complete key exchange with desktop public key
  if (message.desktop_public_key) {
    console.log("🔐 Received desktop public key, completing key exchange...")
    try {
      const activeDesktopId = ctx.connectionStore.getActiveDesktopId
      await ctx.completeKeyExchange(message.desktop_public_key, activeDesktopId)
      console.log("✅ E2EE shared secret derived - secure communication enabled!")
    } catch (error) {
      console.error("❌ Failed to complete key exchange:", error)
    }
  }

  if (message.success) {
    console.log("✅ E2EE handshake completed successfully")

    // Set status to 'paired' for relay mode
    if (ctx.connectionStore.getConnectionMode === 'relay') {
      ctx.connectionStore.setStatus('paired')
      console.log("✅ E2EE complete - status set to 'paired', ready for signed messages")
    }

    clearPairingData()
  } else if (message.error) {
    console.error("❌ E2EE handshake failed:", message.error)
  }
}

/**
 * Handle 'active_deck' action
 */
export function handleActiveDeck(message: any, ctx: WebSocketHandlerContext) {
  if (message.deck) {
    ctx.deckStore.updateActiveDeck(message.deck)
  }
}

/**
 * Handle 'deck_updated' action
 */
export function handleDeckUpdated(message: any, ctx: WebSocketHandlerContext) {
  if (message.deck) {
    // Update active deck
    ctx.deckStore.updateActiveDeck(message.deck)

    // Find and update deck in decks array
    const decks = ctx.deckStore.getAllDecks
    const deckIndex = decks.findIndex(d => d.id === message.deck.id)
    if (deckIndex !== -1) {
      ctx.deckStore.updateDeckById(message.deck.id, message.deck)
    }
  }
}

/**
 * Handle 'all_decks' action
 */
export function handleAllDecks(message: any, ctx: WebSocketHandlerContext) {
  if (message.decks) {
    ctx.deckStore.setDecks(message.decks)
  }
}

/**
 * Handle 'server_info' action
 */
export function handleServerInfoAction(message: any, ctx: WebSocketHandlerContext) {
  if (message.server) {
    ctx.handleServerInfo(message.server)
  }

  // Cache feature limits in connection store
  if (message.feature_limits) {
    console.log('📊 Caching feature limits from server_info:', message.feature_limits)
    ctx.connectionStore.setFeatureLimits(message.feature_limits)
  }
}

/**
 * Handle 'volume_changed' action
 */
export function handleVolumeChangedAction(message: any, ctx: WebSocketHandlerContext) {
  if (message.volume !== undefined) {
    ctx.handleVolumeChange(message)
  }
}

export function handleActionRuntimeReadiness(message: any, ctx: WebSocketHandlerContext) {
  const readiness = message.readiness
  if (!readiness || !['initializing', 'ready', 'failed'].includes(readiness.status)) {
    console.warn('Ignoring invalid action runtime readiness payload')
    return
  }
  ctx.connectionStore.setActionRuntimeReadiness({
    status: readiness.status,
    phase: typeof readiness.phase === 'string' ? readiness.phase : 'unknown',
    warnings: Array.isArray(readiness.warnings)
      ? readiness.warnings.filter((warning: unknown) => typeof warning === 'string')
      : [],
    error: typeof readiness.error === 'string' ? readiness.error : null,
  })
}

/**
 * Handle 'component' action
 */
export function handleComponentAction(message: any, ctx: WebSocketHandlerContext) {
  if (message.payload) {
    console.log("Component action received:", message.payload)
  }
}

export async function handleDeckWidgetSnapshot(
  message: DeckWidgetSnapshotMessage,
  ctx: WebSocketHandlerContext,
) {
  activeWidgetManifests.clear()
  const desktopId = ctx.connectionStore.getActiveDesktopId
  if (desktopId) {
    const {
      cachePortableDefinitions,
      loadPortableAssets,
      loadPortableDefinitions,
      portableCacheInventory,
      verifyPortableDefinition,
    } = await import("../utils/portableWidgetCache")
    const received = (message.definitions ?? []).filter((definition) => (
      definition.id.startsWith("vortideck.") || verifyPortableDefinition(definition)
    ))
    await cachePortableDefinitions(desktopId, received.filter((definition) => (
      Boolean(definition.contentHash)
    )))
    const cached = await loadPortableDefinitions(desktopId)
    const cachedAssets = await loadPortableAssets(desktopId)
    usePortableWidgetAssets().register(cachedAssets)
    const byId = new Map(cached.map((definition) => [definition.id, definition]))
    for (const definition of received) byId.set(definition.id, definition)
    message = { ...message, definitions: Array.from(byId.values()) }
    const inventory = await portableCacheInventory(desktopId)
    await ctx.sendSignedMessage({
      action: "deck_widget_cache_inventory",
      protocolVersion: 2,
      schemaVersions: [1],
      deckId: message.deckId,
      ...inventory,
    })
  }
  return useSyncedDeckWidgets().applySnapshot(message)
}

export async function handleDeckWidgetDelta(
  message: DeckWidgetDeltaMessage,
  ctx: WebSocketHandlerContext,
) {
  const applied = useSyncedDeckWidgets().applyDelta(message)
  if (!applied) await ctx.requestDeckWidgetSnapshot()
  return applied
}

export async function handleDeckWidgetState(
  message: DeckWidgetStateMessage,
  ctx: WebSocketHandlerContext,
) {
  const applied = useSyncedDeckWidgets().applyState(message)
  if (!applied) await ctx.requestDeckWidgetSnapshot()
  return applied
}

export function handleDeckWidgetIntentAck(message: DeckWidgetIntentAckMessage) {
  if (message.protocolVersion !== 1 || message.status !== "rejected") return
  const notifications = useMainStore()
  const key = `widget-intent:${JSON.stringify([
    message.deckId, message.instanceId, message.portId,
    message.interactionSessionId ?? '', message.sequence, message.correlationId ?? '',
  ])}`
  // Replayed acknowledgements must not reopen a dismissed toast.
  if (notifications.notifications.some(item => item.key === key)) return
  notifications.addNotification({
    key,
    title: 'Action could not run',
    message: message.message?.trim() || message.code || 'The host rejected this action.',
    type: 'error',
    autoClose: true,
    duration: 5000,
  }, 'mobile')
}

export async function handleDeckWidgetDefinitions(
  message: DeckWidgetDefinitionsMessage,
  ctx: WebSocketHandlerContext,
) {
  if (message.protocolVersion !== 1) return false
  activeWidgetManifests.clear()
  const registry = useWidgetRegistry()
  for (const definitionId of message.removeDefinitions) registry.unregisterRemote(definitionId)

  const desktopId = ctx.connectionStore.getActiveDesktopId
  const {
    cachePortableDefinitions,
    removePortableDefinitions,
    verifyPortableDefinition,
  } = await import('../utils/portableWidgetCache')
  const accepted = message.definitions.filter((definition) => (
    definition.id.startsWith('vortideck.') || verifyPortableDefinition(definition)
  ))
  for (const definition of accepted) registry.registerRemote(definition)
  if (desktopId) {
    await removePortableDefinitions(desktopId, message.removeDefinitions)
    await cachePortableDefinitions(desktopId, accepted.filter(({ contentHash }) => Boolean(contentHash)))
    const { portableCacheInventory } = await import('../utils/portableWidgetCache')
    const inventory = await portableCacheInventory(desktopId)
    const deckId = useSyncedDeckWidgets().deckId.value
    if (deckId !== undefined && deckId !== null) {
      await ctx.sendSignedMessage({
        action: 'deck_widget_cache_inventory',
        protocolVersion: 2,
        schemaVersions: [1],
        deckId,
        ...inventory,
      })
    }
  }
  return true
}

export async function handleDeckWidgetObjects(
  message: DeckWidgetObjectsMessage,
  ctx: WebSocketHandlerContext,
) {
  if (message.protocolVersion !== 2) return false
  if (useSyncedDeckWidgets().deckId.value !== message.deckId) return false
  const desktopId = ctx.connectionStore.getActiveDesktopId
  if (!desktopId) return false
  const {
    cachePortableAssets,
    cachePortableDefinitions,
    verifyPortableAssetObject,
    verifyPortableDefinition,
  } = await import('../utils/portableWidgetCache')
  const manifest = activeWidgetManifests.get(message.deckId)
  if (!manifest) return false
  const definitions = message.definitions.filter((definition) => (
    Boolean(definition.contentHash
      && manifest.definitionsByHash.get(definition.contentHash) === definition.id)
    && verifyPortableDefinition(definition)
  ))
  const assets = message.assets.filter((object) => (
    manifest.assetsByHash.get(object.asset.sha256) === JSON.stringify(object.asset)
    && verifyPortableAssetObject(object)
  ))
  await Promise.all([
    cachePortableDefinitions(desktopId, definitions),
    cachePortableAssets(desktopId, assets),
  ])
  const registry = useWidgetRegistry()
  for (const definition of definitions) registry.registerRemote(definition)
  usePortableWidgetAssets().register(assets)
  return definitions.length === message.definitions.length && assets.length === message.assets.length
}

export function handleDeckWidgetManifest(message: DeckWidgetManifestMessage) {
  const hashPattern = /^[a-f0-9]{64}$/
  if (message.protocolVersion !== 2 || !message.schemaVersions.includes(1)
    || !Number.isInteger(message.deckId) || !Number.isInteger(message.deckRevision)
    || message.deckRevision < 0 || message.definitions.length > 128 || message.assets.length > 256) {
    return false
  }
  const definitionsByHash = new Map<string, string>()
  for (const definition of message.definitions) {
    if (!definition.definitionId || definition.definitionId.length > 256
      || !hashPattern.test(definition.contentHash)) return false
    if (definitionsByHash.has(definition.contentHash)) return false
    definitionsByHash.set(definition.contentHash, definition.definitionId)
  }
  const assetsByHash = new Map<string, string>()
  for (const asset of message.assets) {
    if (!asset.id || asset.id.length > 256 || !hashPattern.test(asset.sha256)
      || !['image/png', 'image/jpeg', 'image/webp'].includes(asset.mediaType)
      || !Number.isInteger(asset.byteLength) || asset.byteLength <= 0
      || asset.byteLength > 2 * 1024 * 1024
      || !['icon', 'artwork', 'background'].includes(asset.purpose)) return false
    const serialized = JSON.stringify(asset)
    if (assetsByHash.has(asset.sha256) && assetsByHash.get(asset.sha256) !== serialized) return false
    assetsByHash.set(asset.sha256, serialized)
  }
  activeWidgetManifests.clear()
  activeWidgetManifests.set(message.deckId, { definitionsByHash, assetsByHash })
  return true
}

// ❌ REMOVED: Individual feature limit response handling
// Feature limits are now received in bulk via server_info message on connection
// and stored in Connection store for instant access with no WebSocket delay
