# Remote Relay and Gateway Architecture

Status: dedicated relay paths, protocol v2, and automatic LAN/Tailscale direct candidates implemented on 2026-09-04. Remote first-time pairing and payload encryption are the next security milestone; they are not represented as complete.

## Decision

Keep the VortiDeck backend as the default application gateway. Do not replace it with Tailscale or Cloudflare Tunnel.

```text
mobile ── WSS ──► VortiDeck gateway ── Redis/Channels ──► desktop
   │                    │                                      │
   └─ account auth      ├─ ownership + routing                 ├─ action engine
                        ├─ presence                            ├─ flows/approvals
                        └─ short-lived WS tickets              └─ OBS/overlays
```

This is an application protocol: it knows users, desktops, mobile clients, approvals, and action messages. Tailscale is a private IP network and Cloudflare Tunnel publishes a private service through Cloudflare. Neither product replaces VortiDeck's account-aware routing or product policy.

Tailscale is an automatically detected direct-connect transport. Cloudflare Tunnel remains an optional way for a self-hoster to expose the gateway without router port forwarding. Both sit below VortiDeck's protocol.

## Direct connection candidates

VortiDeck owns one typed endpoint contract: `transport`, `host`, `port`, `path`, `secure`, and `priority`. The running desktop server is the only source of these values. QR pairing and relay presence serialize the same candidates; neither mDNS nor the mobile client reconstructs a port or path.

At desktop startup, VortiDeck reads the official machine-readable `tailscale status --json` output without changing Tailscale configuration. It resolves the installed CLI through `PATH`, with a Windows `ProgramFiles`-relative fallback rather than a machine-specific absolute path. If the daemon is running, the unified server listens on the LAN address and detected Tailscale addresses. It publishes the stable Tailscale IP and, when available, the node's MagicDNS name. Mobile starts candidate priority groups with a short 120 ms stagger: LAN first, Tailscale IP second, and MagicDNS third. The first WebSocket handshake wins, so a slow or broken preferred route does not impose its full timeout. Automatic mode switches to a fresh single-use relay ticket only when no direct path responds.

The mobile connection preference is policy, not observed state. It is persisted independently as `automatic`, `direct`, or `relay`; the active route is reported separately as LAN, Tailscale, or VortiDeck relay. `direct` disables relay fallback, while `relay` skips private candidate probing. Explicit local discovery and initial QR pairing remain direct because a relay session cannot authenticate until secure pairing has established endpoint credentials.

Desktop Settings exposes the complementary server-side controls and diagnostics. Relay can be enabled or disabled on the desktop; Tailscale discovery remains automatic because VortiDeck consumes but does not manage the user's tailnet. The Connection panel reports the actual bound address and port, authoritative mDNS instance and DNS-safe `.local.` hostname, Tailscale detection, live relay state, and every advertised candidate. The friendly mDNS instance preserves the PC name, while its technical hostname normalizes invalid characters and adds a stable desktop-ID suffix to prevent collisions. Route preference remains mobile-owned because each client can have different reachability.

The unified server prefers port `9001` for stable OBS and development behavior. Warp reserves that port directly; if it is occupied, Warp asks the operating system for a free port. Only after the socket is reserved does VortiDeck publish the actual port through `ServerInfo`, mDNS, QR pairing, overlay URLs, Tailscale candidates, and relay presence. This avoids both startup failure and the old check-then-bind race.

This behavior requires no VortiDeck Tailscale setting, port forwarding, Tailscale API key, admin permission, or re-pairing. Both devices only need to be logged into tailnets whose access policy permits the connection. mDNS remains LAN-only; MagicDNS resolves a known Tailscale node name but does not advertise DNS-SD services.

Tailscale Serve is not configured automatically. It is designed to expose localhost services to a tailnet, but current upstream WebSocket issues make its HTTPS reverse proxy a weaker default for VortiDeck's long-lived socket. Tailscale Services are also inappropriate for ordinary user desktops because service hosts use tagged device identities. Direct node-IP binding is therefore the bounded default; Serve remains an optional future mode after its WebSocket behavior is suitable.

VortiDeck does not bundle a Tailscale executable. Although the open-source client is BSD-3-Clause licensed, a copied CLI alone is not a network: the installed daemon, authenticated identity, updates, and OS networking integration are still required. Embedding `tsnet` would instead create a separate userspace Tailscale node with its own login/auth-key and lifecycle, which is not the desired “use the user's existing Tailscale connection” behavior.

Candidates are reachability hints, not trust assertions. The account-authenticated relay sanitizes and bounds candidate metadata, while the desktop pairing token/device token plus endpoint E2EE remains authoritative. A process merely reaching the same host or tailnet receives no implicit trust. The only unsigned local bootstrap paths are the declared desktop host UI and detected OBS integration.

Current lifecycle boundary: Tailscale is detected when the desktop server starts. If Tailscale is installed or connected after VortiDeck has already launched, restart VortiDeck to publish the new direct candidates; relay remains available meanwhile.

Primary references: [Tailscale CLI JSON status](https://tailscale.com/kb/1080/cli), [stable node addresses](https://tailscale.com/docs/concepts/ip-and-dns-addresses), [Tailscale Serve](https://tailscale.com/docs/features/tailscale-serve), [Tailscale Services prerequisites](https://tailscale.com/docs/features/tailscale-services), and the current [Serve WebSocket HTTP/2 issue](https://github.com/tailscale/tailscale/issues/20882).

## Current implementation

The backend in the sibling `backend` repository is real, active architecture rather than a stub:

- Django Channels terminates `/ws/relay/desktop/` and `/ws/relay/mobile/`.
- JWT authentication establishes the account, and Redis maps an account's desktop and mobile channel presence.
- Channel-layer messages forward commands and state in either direction.
- The desktop and mobile retain their shared HMAC secret; the backend does not receive it.
- Local direct connections and remote relay connections enter the same desktop command/action boundary.

The old relay mode inside `/ws/streaming/` and the old `/ws/relay/` alias were duplicate paths and have been removed. Streaming events and remote device control now have separate lifecycles.

Protocol v2 relay envelopes contain signed `protocol_version`, `message_id`, `sent_at`, `expires_at`, `payload`, and `nonce` fields. The gateway enforces envelope size, shape, identifier bounds, lifetime bounds, and per-connection rate limits before forwarding. Endpoint verification remains authoritative.

WebSocket upgrades use a random, single-use, 60-second ticket. The account JWT is submitted to the HTTPS ticket endpoint in an authorization header and is no longer placed in relay URLs. Only a hash of the ticket is stored in Redis, and consumption is atomic.

## Why a working relay was still incomplete

"Works in a normal test" proves the happy path, not all production guarantees. The implementation successfully routed one desktop and mobile, but had these concrete gaps:

- the committed mobile release configuration did not provide `WS_URL`;
- long-lived JWTs appeared in WebSocket query strings;
- one desktop helper omitted `target_client_id`, turning private responses into broadcasts;
- envelope validation only checked whether three keys existed;
- relay payloads and partial shared-secret material were written to debug logs;
- dead relay logic remained inside the streaming consumer;
- Redis stores each complete session as one JSON value, so simultaneous mobile updates can overwrite each other;
- Redis Pub/Sub forwarding is low-latency but not durable delivery.

The first five items and the dead path are now corrected. Redis atomicity and delivery recovery remain explicit work, not hidden claims.

## Pairing and encryption

Public-key pairing metadata does not need confidentiality. A desktop public key, mobile public key, random challenge, protocol version, and one-time capability identifier may travel in plaintext over TLS. Their integrity, freshness, account binding, and single-use behavior matter. Private keys and established shared secrets must never leave their endpoint.

The current first pairing is local. A QR code can contain relay metadata, but the active mobile flow still establishes its shared secret through the local desktop socket, and the desktop rejects first-time relay handshakes. Consequently, remote reconnect works after pairing; first-time remote pairing does not.

Remote pairing should be added as one gateway-mediated bootstrap:

1. The authenticated desktop creates a random, short-lived, single-use pairing capability and supplies its public key.
2. The QR contains the capability identifier, desktop identity, desktop public key, relay URL, and expiry—never a private key or established shared secret.
3. The authenticated mobile claims the capability and supplies its client identity and ephemeral public key.
4. The gateway atomically verifies account, desktop, capability, expiry, and unused state, then forwards the claim.
5. The desktop verifies the capability it created, derives the shared secret, and returns key confirmation bound to both identities and the transcript.
6. Both endpoints persist the established secret; the capability is consumed and cannot be replayed.

Normal relay payloads currently have HMAC integrity and replay protection but are visible to the gateway. TLS protects them on each network hop; HMAC prevents the gateway from silently forging a valid endpoint message. This must not be described as payload-confidential E2EE.

Protocol v3 should replace plaintext `payload` with ChaCha20-Poly1305 ciphertext. Keep only routing metadata outside the ciphertext and bind it as AEAD associated data. This can reuse the established endpoint secret and does not conflict with remote pairing: the bootstrap's public material remains visible, while all post-pairing application content is encrypted.

## Delivery guarantees

The control relay should stay low latency. It should not become a durable workflow executor or a video pipe.

- `message_id` correlates transport observations with action/invocation IDs.
- Application-level action acknowledgements and persisted invocation status remain the source of truth.
- On reconnect, mobile requests a fresh deck/approval/readiness snapshot instead of assuming every Pub/Sub message arrived.
- Commands that can cause side effects use the invocation service's idempotency key.
- A relay send means accepted for forwarding, not that OBS, Discord, or another provider applied the action.

Before supporting many mobiles per desktop, replace JSON read-modify-write session mutations with atomic Redis hashes/transactions or move each desktop session actor to a Durable Object. This is a concurrency fix, not a reason to rewrite the desktop engine.

## Discord and remote video

Discord integration belongs behind the normal integration capability contract. REST/webhook actions handle outgoing messages and moderation; a bot/Gateway connector supplies incoming Discord events. Those provider details do not belong in the device relay protocol.

Do not send another person's live video frames through this JSON/Redis control relay. Use one of these media paths:

- simplest: obtain an authorized Discord/media attachment URL, download through the moderated media-ingestion boundary, then render it in a targeted overlay/media player;
- live guest video: use WebRTC with an SFU such as LiveKit, mediasoup, or Cloudflare Realtime, then add its browser view as an OBS source;
- existing communication-app capture: use the app's supported capture/virtual-camera path where permitted.

The VortiDeck gateway may mint short-lived room/media capabilities and coordinate state, but media bytes should use a media-specialized transport.

## Hosting choices

The hosted default requires no inbound ports on the user's desktop or phone; both make outbound WSS connections.

A self-hosted public gateway requires a reachable HTTPS/WSS endpoint. That can be provided by:

- router port forwarding plus TLS/reverse proxy;
- Cloudflare Tunnel, with no inbound router port;
- Tailscale, when every participating device joins the same tailnet.

The product should ship hosted relay as the seamless default and expose self-hosting as an advanced server URL/configuration option. Do not bundle an automatically public desktop listener.

## Next bounded milestones

1. Make Redis session mutations atomic and fence stale desktop disconnects by channel/connection generation.
2. Implement the single-use remote pairing capability and transcript-bound key confirmation.
3. Introduce encrypted protocol-v3 payload envelopes and remove protocol v2 in one declared compatibility window.
4. Add reconnect snapshot/resynchronization tests and two-mobile concurrency tests.
5. Evaluate Durable Objects only when global placement, simpler per-desktop state ownership, or hosted operational scale justifies a backend migration.
