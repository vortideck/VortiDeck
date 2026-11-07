/**
 * Package manifest — TS mirror of the Rust plugin manifest's `contributions`
 * (see `plugins/contributions.rs` + `bundle.rs`).
 *
 * A package declares typed contributions; a widget is one contribution type
 * (alongside flow-node, ui-panel, macro). The CapabilityDialog reviews a
 * package's capabilities on install — it's package-general, not widget-only.
 * "Capability" is the user-facing word; under the hood capabilities map to
 * `PluginPermission` enforced by `PermissionChecker`.
 *
 * See `docs/WIDGET_CAPABILITY_ARCHITECTURE.md` §3.5 (Package & Contribution
 * Model) and §4 (capabilities vs permissions).
 */

/** Network access — selects the sandbox iframe's CSP `connect-src`. */
export type NetworkCapability = 'none' | { whitelist: string[] } | 'full'

/** Capabilities a widget declares (and the user grants at install). */
export interface WidgetCapabilities {
  /** Inbound event types the widget receives, e.g. "follow", "chatMessage".
   * The overlay bridge filters delivery by this list (least privilege). */
  subscribe?: string[]
  /** Outbound actions the widget may call, e.g. "triggerFlow:flow_42", "openUi".
   * Checked by PermissionChecker before execution. */
  actions?: string[]
  /** Whether the widget may open a UI panel. */
  ui?: boolean
  /** Network access — drives the sandbox iframe CSP `connect-src`. */
  network?: NetworkCapability
}

/**
 * Convert framework/reactive capability values into an owned transport-safe
 * value. WebView structuredClone cannot clone Vue proxies, and saved overlay
 * elements must not retain references to the package-list response.
 */
export function copyWidgetCapabilities(
  capabilities?: WidgetCapabilities,
): WidgetCapabilities {
  const network = capabilities?.network
  return {
    subscribe: [...(capabilities?.subscribe ?? [])],
    actions: [...(capabilities?.actions ?? [])],
    ui: Boolean(capabilities?.ui),
    network: network && typeof network === 'object'
      ? { whitelist: [...network.whitelist] }
      : (network ?? 'none'),
  }
}

/** A typed contribution declared by a package (internally tagged by `type`). */
export type Contribution =
  | { type: 'flowNode'; node: unknown }
  | {
      type: 'widget'
      /** Stable contribution identity within a package. */
      id?: string
      name?: string
      description?: string
      icon?: string
      /** Reviewed host renderer; honored only for `builtin` packages. */
      schemaId?: string
      width?: number
      height?: number
      /** JSON Schema draft 7 for saved authoring settings (separate from live data). */
      configSchema?: Record<string, unknown>
      capabilities: WidgetCapabilities
      /** Sandboxed package HTML. Empty only for builtin schema widgets. */
      content: string
    }
  | { type: 'uiPanel'; panel: string }
  | { type: 'macro'; macroSpec: unknown }

export type WidgetContribution = Extract<Contribution, { type: 'widget' }>

/** Trust level — drives sandbox strictness and Capability Dialog copy. */
export type PackageTrust = 'builtin' | 'verified' | 'community'

/** A plugin/widget package manifest (package-general; widget is one contribution type). */
export interface PackageManifest {
  id: string
  name: string
  version: string
  author?: string
  description?: string
  /** Legacy package-level permissions (PluginPermission). Optional. */
  permissions?: string[]
  /** Typed contributions. A package has ≥1 contribution. */
  contributions: Contribution[]
  /** Trust/attestation level. Defaults to 'community' for side-loaded. */
  trust?: PackageTrust
}

/**
 * Unified package view — the unit the "My Plugins" surface lists and manages.
 * Mirrors the Rust `PackageInfo` returned by `list_packages`. Covers runtime
 * plugins AND widget packages (and bundles) in one shape. `has_runtime` /
 * `is_granted` are snake_case on the wire (no serde rename in Rust).
 */
export interface PackageInfo {
  id: string
  name: string
  version: string
  author?: string
  description?: string
  /** Optional icon path (runtime plugins). Convert via convertFileSrc. */
  icon_path?: string
  /** 'builtin' | 'verified' | 'community'. */
  trust: string
  /** Typed contributions (widget/flowNode/uiPanel/macro). */
  contributions: Contribution[]
  /** Legacy PluginPermission strings (lowercase on the wire). Empty for widget-only. */
  permissions: string[]
  has_runtime: boolean
  /** Whether the package runtime is loaded in the current engine process. */
  runtime_active: boolean
  is_granted: boolean
  /** Granted widget changed since consent (content/capabilities drift) → re-consent. */
  needs_reconsent: boolean
}

// ── Helpers ─────────────────────────────────────────────────────────────

/** All widget contributions in a package (works for PackageManifest or PackageInfo). */
export function widgetContributions(pkg: { contributions: Contribution[] }): WidgetContribution[] {
  return pkg.contributions.filter((c): c is WidgetContribution => c.type === 'widget')
}

/**
 * Plain-language label for a capability string, for the Capability Dialog.
 * Falls back to the raw capability if unmapped.
 */
export function capabilityLabel(cap: string): string {
  const map: Record<string, string> = {
    // inbound events
    follow: 'see when someone follows you',
    raid: 'see when someone raids you',
    subscription: 'see when someone subscribes',
    bits: 'see when someone cheers bits',
    chatMessage: 'read chat messages',
    viewerCount: 'see your viewer count',
    streamOnline: 'see when you go live',
    streamOffline: 'see when you go offline',
  }
  if (map[cap]) return map[cap]
  if (cap === 'network') return 'network access'
  if (cap === 'openUi') return 'open a settings panel'
  if (cap.startsWith('triggerFlow:')) return `trigger the "${cap.slice('triggerFlow:'.length)}" automation`
  return cap
}

/**
 * Plain-language label for a legacy `PluginPermission` string (runtime plugins),
 * parallel to `capabilityLabel`. PluginPermission is lowercase on the wire
 * (serde `rename_all = "lowercase"`). Falls back to the raw string if unmapped.
 */
export function permissionLabel(perm: string): string {
  const map: Record<string, string> = {
    dom: 'modify the overlay/page DOM',
    css: 'inject custom CSS',
    network: 'make network requests',
    filesystem: 'read and write files',
    executecommands: 'run system commands',
    accesscamera: 'access your camera',
    accessmicrophone: 'access your microphone',
    flows: 'run automations (flows)',
    storage: 'use local storage',
  }
  return map[perm] ?? perm
}

/**
 * The single subscription vocabulary bridge (Phase 3). Maps a widget's
 * user-facing `subscribe` capability string → the live `streaming:event`
 * CloudEvent `type`(s) it corresponds to. The community-widget forwarder
 * (Phase 4) uses this to deliver ONLY consented event types to each widget
 * iframe (least privilege).
 *
 * CloudEvent types come from `useStreamingRust`'s `CloudEvent` union (snake_case
 * on the wire). Note `viewerCount` rides on `channel_update` (which carries
 * `viewer_count`); it has no dedicated event type.
 */
export const widgetSubscribeToCloudEvents: Record<string, string[]> = {
  follow: ['follow'],
  chatMessage: ['chat_message'],
  viewerCount: ['channel_update'],
  subscription: ['subscription'],
  raid: ['raid'],
  bits: ['bits'],
  streamOnline: ['stream_online'],
  streamOffline: ['stream_offline'],
}

/// Legacy PluginPermission strings that are high-risk (flagged in lists/buttons).
const HIGH_RISK_PERMS = new Set([
  'network',
  'filesystem',
  'executecommands',
  'accesscamera',
  'accessmicrophone',
])

/** A flattened access item for display (used by the dialog + per-plugin settings). */
export interface AccessItem {
  label: string
  risk: 'normal' | 'high'
}

/**
 * Flatten a package's requested access into a plain-language list: legacy runtime
 * `permissions` first, then widget `capabilities`. Shared by the CapabilityDialog
 * and the per-plugin Settings modal so they never drift. High-risk items (full
 * network, filesystem, system commands, camera/mic) are flagged.
 */
export function flattenPackageAccess(pkg: {
  permissions?: string[]
  contributions: Contribution[]
}): AccessItem[] {
  const items: AccessItem[] = []
  for (const perm of pkg.permissions ?? []) {
    items.push({ label: permissionLabel(perm), risk: HIGH_RISK_PERMS.has(perm) ? 'high' : 'normal' })
  }
  for (const w of widgetContributions(pkg)) {
    for (const evt of w.capabilities.subscribe ?? []) {
      items.push({ label: capabilityLabel(evt), risk: 'normal' })
    }
    for (const action of w.capabilities.actions ?? []) {
      items.push({ label: capabilityLabel(action), risk: 'normal' })
    }
    if (w.capabilities.ui) {
      items.push({ label: capabilityLabel('openUi'), risk: 'normal' })
    }
    const net = w.capabilities.network
    if (net === 'full') {
      items.push({ label: 'unrestricted network access', risk: 'high' })
    } else if (net && typeof net === 'object' && 'whitelist' in net) {
      const n = net.whitelist.length
      items.push({ label: `limited network access (${n} host${n === 1 ? '' : 's'})`, risk: 'normal' })
    }
  }
  return items
}

/** Validate a package manifest. Returns error strings (empty = valid). */
export function validatePackage(pkg: unknown): string[] {
  const errors: string[] = []
  if (!isObject(pkg)) return ['manifest must be an object']
  const m = pkg as unknown as PackageManifest
  if (!m.id) errors.push('missing id')
  if (!m.name) errors.push('missing name')
  if (!m.version) errors.push('missing version')
  if (!Array.isArray(m.contributions) || m.contributions.length === 0) {
    errors.push('manifest must declare at least one contribution')
    return errors
  }
  for (const [i, c] of m.contributions.entries()) {
    if (c.type === 'widget' && !c.content) {
      errors.push(`contribution[${i}] (widget): missing content`)
    }
    if (c.type === 'uiPanel' && !c.panel) {
      errors.push(`contribution[${i}] (uiPanel): missing panel`)
    }
  }
  return errors
}

function isObject(v: unknown): v is Record<string, unknown> {
  return typeof v === 'object' && v !== null && !Array.isArray(v)
}
