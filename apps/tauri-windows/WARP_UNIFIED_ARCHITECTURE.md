# VortiDeck Unified Warp Server Architecture

## Overview
The VortiDeck application now uses a single unified Warp server that handles all HTTP and WebSocket traffic on port 9001, replacing the previous complex Pingora reverse proxy setup.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                    VortiDeck Unified Server                    │
│                         (Warp Framework)                       │
│                      Port 9001 (Single Port)                  │
└─────────────────────────────────────────────────────────────────┘
                                  │
                    ┌─────────────┼─────────────┐
                    │             │             │
┌───────────────────▼──┐ ┌────────▼────────┐ ┌──▼──────────────────┐
│   WebSocket Route    │ │  Static Routes  │ │    API Routes       │
│      /ws             │ │                 │ │                     │
├──────────────────────┤ ├─────────────────┤ ├─────────────────────┤
│ • Real-time comms    │ │ /icons/*        │ │ /overlay.html       │
│ • Device control     │ │ • App icons     │ │ • Overlay content   │
│ • OBS integration    │ │ • Static files  │ │ /check-update       │
│ • State sync         │ │ • File serving  │ │ • Update check API  │
└──────────────────────┘ └─────────────────┘ └─────────────────────┘
```

## Client Connection Flow

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Mobile App    │    │   Desktop App   │    │   OBS Plugin    │
│                 │    │                 │    │                 │
└─────────┬───────┘    └─────────┬───────┘    └─────────┬───────┘
          │                      │                      │
          │ mDNS Discovery       │ Direct Connect       │ mDNS Discovery
          │ ws://IP:9001/ws      │ ws://IP:9001/ws      │ ws://IP:9001/ws
          │                      │                      │
          └──────────────────────┼──────────────────────┘
                                 │
                                 ▼
                    ┌─────────────────────────────┐
                    │    Unified Warp Server      │
                    │        Port 9001            │
                    │                             │
                    │  ┌─────────────────────┐    │
                    │  │  WebSocket Handler  │    │
                    │  │                     │    │
                    │  │ • Authentication    │    │
                    │  │ • Message routing   │    │
                    │  │ • State management  │    │
                    │  │ • Device tracking   │    │
                    │  └─────────────────────┘    │
                    └─────────────────────────────┘
```

## Request Routing

```
Incoming Request → Warp Router → Route Handler
                      │
        ┌─────────────┼─────────────┐
        │             │             │
        ▼             ▼             ▼
┌──────────────┐ ┌──────────┐ ┌─────────────┐
│ WebSocket    │ │ Static   │ │ API         │
│ /ws          │ │ /icons/* │ │ /overlay.   │
│              │ │          │ │ /check-     │
│ Upgrades to  │ │ Serves   │ │ Returns     │
│ WebSocket    │ │ files    │ │ JSON/HTML   │
│ protocol     │ │ from     │ │ responses   │
│              │ │ icons/   │ │             │
│              │ │ dir      │ │             │
└──────────────┘ └──────────┘ └─────────────┘
```

## Service Integration

```
┌─────────────────────────────────────────────────────────────────┐
│                        Main Application                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────┐  │
│  │  State Manager  │    │    Database     │    │   Tauri     │  │
│  │                 │    │                 │    │   Context   │  │
│  │ • Device state  │◄──►│ • Peer storage  │◄──►│ • App state │  │
│  │ • OBS comms     │    │ • Button config │    │ • UI events │  │
│  │ • WebSocket     │    │ • Settings      │    │ • Commands  │  │
│  │   management    │    │                 │    │             │  │
│  └─────────────────┘    └─────────────────┘    └─────────────┘  │
│           │                       │                      │     │
└───────────┼───────────────────────┼──────────────────────┼─────┘
            │                       │                      │
            └───────────────────────┼──────────────────────┘
                                    │
                          ┌─────────▼─────────┐
                          │  Warp Server      │
                          │  Port 9001        │
                          │                   │
                          │ All services      │
                          │ unified in one    │
                          │ server process    │
                          └───────────────────┘
```

## Network Flow

```
External Network Access:
┌─────────────────┐
│   Router/LAN    │ ──► 192.168.x.x:9001
└─────────────────┘

Local Machine:
┌─────────────────┐
│   127.0.0.1     │ ──► 127.0.0.1:9001
└─────────────────┘

mDNS Advertisement:
┌─────────────────┐
│ _vortideck._tcp │ ──► Advertises port 9001
└─────────────────┘                │
                                   ▼
                        ┌─────────────────┐
                        │ Clients discover│
                        │ via Bonjour/    │
                        │ Zeroconf        │
                        └─────────────────┘
```

## Key Benefits

### 🎯 **Simplified Architecture**
- Single port (9001) for all services
- No complex proxy routing
- Unified error handling and logging

### ⚡ **Better Performance**
- Direct request handling (no proxy overhead)
- Native Warp WebSocket support
- Efficient static file serving

### 🔧 **Easier Maintenance**
- Single server process to manage
- Consistent configuration
- Simplified deployment

### 🛡️ **Improved Reliability**
- Fewer moving parts
- No proxy-related failures
- Better error recovery

## Implementation Details

### Server Startup
```rust
// Single server handling all routes
start_warp_server(ip, 9001, icons_dir, app_handle).await;
```

### Route Configuration
```rust
let routes = ws_route           // /ws → WebSocket
    .or(icons_route)           // /icons/* → Static files
    .or(overlay_route)         // /overlay.html → HTML
    .or(check_update_route);   // /check-update → JSON API
```

### WebSocket Handling
- Warp's native WebSocket support
- Message format conversion for compatibility
- Direct integration with existing state management

This unified architecture eliminates the complexity of the previous Pingora proxy setup while maintaining all functionality and improving performance and reliability.