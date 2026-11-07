//! Unified local HTTP server and route composition.
use super::*;

/// Webview origins of the VortiDeck desktop and mobile apps (Tauri 2 serves
/// bundled assets from `tauri://localhost` on macOS/iOS/Linux and
/// `http(s)://tauri.localhost` on Windows/Android).
const APP_WEBVIEW_ORIGINS: [&str; 3] = [
    "tauri://localhost",
    "http://tauri.localhost",
    "https://tauri.localhost",
];

/// Nuxt dev server origins (`build.devUrl` in tauri.conf.json); debug builds only.
#[cfg(debug_assertions)]
const DEV_SERVER_ORIGINS: [&str; 2] = ["http://127.0.0.1:4450", "http://localhost:4450"];

/// Decide whether a control WebSocket (`/ws`, `/`) upgrade may proceed.
///
/// Browsers always send `Origin` on WebSocket upgrades and scripts cannot
/// forge it, so any web page open on this PC would otherwise be able to reach
/// the control channel from the LAN address and claim to be the desktop host.
/// Only the app's own webviews are accepted. Native clients (the OBS companion
/// plugin and other non-browser integrations) send no `Origin` and are still
/// subject to the per-connection authentication in `handle_connection_internal`.
/// Pages served by this server (overlay, banners) use `/ws/overlay` and
/// `/ws/banner`, never the control channel, so they are not allowed here.
fn control_ws_origin_allowed(origin: Option<&str>) -> bool {
    let Some(origin) = origin else {
        return true;
    };
    let origin = origin.trim().trim_end_matches('/').to_ascii_lowercase();
    if APP_WEBVIEW_ORIGINS.contains(&origin.as_str()) {
        return true;
    }
    #[cfg(debug_assertions)]
    if DEV_SERVER_ORIGINS.contains(&origin.as_str()) || is_mobile_dev_server_origin(&origin) {
        return true;
    }
    false
}

/// The mobile companion's dev webview is served from `http://<dev host>:3001`
/// (apps/tauri-mobile devUrl with TAURI_DEV_HOST); debug builds only.
#[cfg(debug_assertions)]
fn is_mobile_dev_server_origin(origin: &str) -> bool {
    origin
        .strip_prefix("http://")
        .and_then(|host| host.strip_suffix(":3001"))
        .is_some_and(|host| !host.is_empty() && !host.contains('/'))
}

fn reject_control_ws_origin(
    origin: Option<&str>,
    remote_addr: Option<std::net::SocketAddr>,
) -> warp::reply::Response {
    log::warn!(
        "Rejected control WebSocket upgrade from {:?} with disallowed Origin {:?}",
        remote_addr.map(|addr| addr.ip()),
        origin
    );
    warp::http::Response::builder()
        .status(warp::http::StatusCode::FORBIDDEN)
        .header("Cache-Control", "no-store")
        .body(warp::hyper::Body::from("WebSocket origin not allowed"))
        .unwrap()
}

pub(super) async fn start_warp_server(
    bind_ips: Vec<std::net::IpAddr>,
    identity_ip: std::net::IpAddr,
    port: u16,
    icons_dir: std::path::PathBuf,
    app_handle: AppHandle,
    webhook_handler: Arc<vortideck_core::services::webhook::WebhookHandler>,
    overlay_access_token: String,
    metrics_access_token: String,
    ready: tokio::sync::oneshot::Sender<Result<(u16, Vec<std::net::IpAddr>), String>>,
) {
    use std::net::SocketAddr;
    use warp::{Filter, Reply};

    log::info!("🚀 Starting unified Warp server on {:?}:{}", bind_ips, port);

    // 📡 NEW: Type-specific WebSocket routes

    // Banner WebSocket route at /ws/banner
    let app_handle_banner = app_handle.clone();
    let banner_ws_route = warp::path!("ws" / "banner")
        .and(warp::ws())
        .and(warp::addr::remote())
        .map(move |ws: warp::ws::Ws, remote_addr: Option<SocketAddr>| {
            let app_handle = app_handle_banner.clone();
            ws.on_upgrade(move |websocket| async move {
                let peer_ip = remote_addr
                    .map(|addr| addr.ip().to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                log::info!("📋 Banner WebSocket connection from: {}", peer_ip);
                handle_banner_websocket(websocket, app_handle).await;
            })
        });

    // Overlay WebSocket route at /ws/overlay
    let app_handle_overlay = app_handle.clone();
    let overlay_ws_access_token = overlay_access_token.clone();
    let overlay_ws_route = warp::path!("ws" / "overlay")
        .and(warp::query::<std::collections::HashMap<String, String>>())
        .and(warp::ws())
        .and(warp::addr::remote())
        .map(
            move |query: std::collections::HashMap<String, String>,
                  ws: warp::ws::Ws,
                  remote_addr: Option<SocketAddr>| {
                let app_handle = app_handle_overlay.clone();
                let authorized =
                    overlay::overlay_query_authorized(&query, &overlay_ws_access_token);
                ws.on_upgrade(move |websocket| async move {
                    let peer_ip = remote_addr
                        .map(|addr| addr.ip().to_string())
                        .unwrap_or_else(|| "unknown".to_string());

                    log::info!("🎨 Overlay WebSocket connection from: {}", peer_ip);
                    handle_overlay_websocket(websocket, app_handle, authorized).await;
                })
            },
        );

    // WebSocket route at /ws
    let app_handle_ws = app_handle.clone();
    let server_ip_ws = identity_ip;
    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(warp::addr::remote())
        .and(warp::header::optional::<String>("origin"))
        .map(move |ws: warp::ws::Ws, remote_addr: Option<SocketAddr>, origin: Option<String>| {
            if !control_ws_origin_allowed(origin.as_deref()) {
                return reject_control_ws_origin(origin.as_deref(), remote_addr);
            }
            let app_handle = app_handle_ws.clone();
            let server_ip = server_ip_ws;
            ws.on_upgrade(move |websocket| async move {
                let peer_ip = remote_addr
                    .map(|addr| addr.ip().to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                log::info!("🌐 WebSocket connection from: {}", peer_ip);

                // Handle WebSocket connection (reuse existing logic)
                handle_websocket_connection(websocket, peer_ip, app_handle, server_ip).await;
            })
            .into_response()
        });

    // WebSocket route at root / for compatibility (since clients connect to root)
    let app_handle_root = app_handle.clone();
    let server_ip_root = identity_ip;
    let root_ws_route = warp::path::end()
        .and(warp::ws())
        .and(warp::addr::remote())
        .and(warp::header::optional::<String>("origin"))
        .map(move |ws: warp::ws::Ws, remote_addr: Option<SocketAddr>, origin: Option<String>| {
            if !control_ws_origin_allowed(origin.as_deref()) {
                return reject_control_ws_origin(origin.as_deref(), remote_addr);
            }
            let app_handle = app_handle_root.clone();
            let server_ip = server_ip_root;
            ws.on_upgrade(move |websocket| async move {
                let peer_ip = remote_addr
                    .map(|addr| addr.ip().to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                log::info!("🌐 Root WebSocket connection from: {}", peer_ip);

                // Handle WebSocket connection (reuse existing logic)
                handle_websocket_connection(websocket, peer_ip, app_handle, server_ip).await;
            })
            .into_response()
        });

    // Icons static file serving
    let icons_route = warp::path("icons").and(warp::fs::dir(icons_dir));

    // Overlay HTML route
    let app_handle_overlay = app_handle.clone();
    let overlay_html_access_token = overlay_access_token.clone();
    let overlay_route = warp::path("overlay.html")
        .and(warp::get())
        .and(warp::query::<std::collections::HashMap<String, String>>())
        .map(move |params: std::collections::HashMap<String, String>| {
            if !overlay::overlay_query_authorized(&params, &overlay_html_access_token) {
                return warp::http::Response::builder()
                    .status(warp::http::StatusCode::UNAUTHORIZED)
                    .header("Cache-Control", "no-store")
                    .body("Overlay access token is missing or invalid".to_string())
                    .unwrap();
            }
            let app_handle = app_handle_overlay.clone();

            // Get overlay state from app handle
            let html = if let Some(overlay_state) =
                app_handle.try_state::<Arc<tokio::sync::RwLock<overlay::OverlayState>>>()
            {
                // Use blocking call to get the HTML
                tokio::task::block_in_place(move || {
                    let runtime = tokio::runtime::Handle::current();
                    runtime.block_on(async {
                        let state_guard = overlay_state.read().await;
                        state_guard.generated_html.clone()
                    })
                })
            } else {
                // Fallback if overlay state not available
                log::warn!("Overlay state not available, using fallback HTML");
                "<html><body><h1>Overlay - State Not Available</h1></body></html>".to_string()
            };

            // Add cache-control headers to prevent caching
            warp::http::Response::builder()
                .header("Content-Type", "text/html")
                .header("Cache-Control", "no-cache, no-store, must-revalidate")
                .header("Pragma", "no-cache")
                .header("Expires", "0")
                .body(html)
                .unwrap()
        });

    // Check update route for overlay real-time updates
    let app_handle_check = app_handle.clone();
    let check_update_route = warp::path("check-update").and(warp::get()).map(move || {
        let app_handle = app_handle_check.clone();

        // Get overlay state to check last_updated timestamp
        let updated = if let Some(overlay_state) =
            app_handle.try_state::<Arc<tokio::sync::RwLock<overlay::OverlayState>>>()
        {
            tokio::task::block_in_place(move || {
                let runtime = tokio::runtime::Handle::current();
                runtime.block_on(async {
                    let state_guard = overlay_state.read().await;
                    state_guard.last_updated
                })
            })
        } else {
            0 // Default if overlay state not available
        };

        warp::reply::json(&serde_json::json!({
            "last_updated": updated,
            "timestamp": chrono::Utc::now().timestamp_millis()
        }))
    });

    // Banners route with WebSocket broadcast
    let app_handle_banners = app_handle.clone();
    let banners_route = warp::path("banners").and(warp::get()).map(move || {
        let html = banners::generate_banner_html();
        let timestamp = chrono::Utc::now().timestamp_millis();

        // Broadcast banner HTML update via WebSocket
        if let Some(state_manager) = app_handle_banners.try_state::<Arc<StateManager>>() {
            let state_manager = state_manager.inner().clone();
            tokio::spawn(async move {
                // Get the server sender to broadcast to all WebSocket clients
                if let Some(broadcast_tx) = state_manager.get_server_sender() {
                    let update_message = serde_json::json!({
                        "type": "BANNER_HTML_UPDATED",
                        "timestamp": timestamp
                    });

                    let message_str = update_message.to_string();
                    if let Err(_) = broadcast_tx.send(
                        tokio_tungstenite::tungstenite::Message::Text(message_str.into()),
                    ) {
                        // Ignore send errors (no clients connected)
                    }
                }
            });
        }

        warp::http::Response::builder()
            .header("Content-Type", "text/html")
            .header("Cache-Control", "no-cache, no-store, must-revalidate")
            .header("Pragma", "no-cache")
            .header("Expires", "0")
            .header("X-Banner-Timestamp", timestamp.to_string())
            .body(html)
            .unwrap()
    });

    // Capability-token protected Prometheus exposition. This credential is
    // deliberately separate from the capability delivered to OBS overlays.
    let metrics_app_handle = app_handle.clone();
    let metrics_route = warp::path("metrics")
        .and(warp::get())
        .and(warp::header::optional::<String>("authorization"))
        .and_then(move |authorization: Option<String>| {
            let app_handle = metrics_app_handle.clone();
            let expected_token = metrics_access_token.clone();
            async move {
                let response = if !overlay::bearer_authorized(
                    authorization.as_deref(),
                    &expected_token,
                ) {
                    warp::http::Response::builder()
                        .status(warp::http::StatusCode::UNAUTHORIZED)
                        .body("unauthorized\n".to_string())
                        .unwrap()
                } else if let Some(state) =
                    app_handle
                        .try_state::<vortideck_core::services::flow::commands::FlowExecutorState>()
                {
                    let guard = state.executor.lock().await;
                    match guard.as_ref() {
                        Some(executor) => match executor.inspect_workflow_runtime().await {
                            Ok(inspection) => warp::http::Response::builder()
                                .header("Content-Type", "text/plain; version=0.0.4")
                                .header("Cache-Control", "no-store")
                                .body(inspection.metrics.to_prometheus_text())
                                .unwrap(),
                            Err(error) => warp::http::Response::builder()
                                .status(warp::http::StatusCode::INTERNAL_SERVER_ERROR)
                                .body(format!("metrics unavailable: {}\n", error))
                                .unwrap(),
                        },
                        None => warp::http::Response::builder()
                            .status(warp::http::StatusCode::SERVICE_UNAVAILABLE)
                            .body("workflow runtime not initialized\n".to_string())
                            .unwrap(),
                    }
                } else {
                    warp::http::Response::builder()
                        .status(warp::http::StatusCode::SERVICE_UNAVAILABLE)
                        .body("workflow state unavailable\n".to_string())
                        .unwrap()
                };
                Ok::<_, std::convert::Infallible>(response)
            }
        });

    // CORS configuration for overlay routes
    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(vec!["GET", "POST"])
        .allow_headers(vec!["content-type", "authorization", "idempotency-key"]);

    // Combine all routes with CORS
    // Create webhook routes using the adapter
    let webhook_routes = crate::adapters::webhook_routes::create_webhook_routes(webhook_handler);

    let routes = banner_ws_route
        .or(overlay_ws_route)
        .or(root_ws_route)
        .or(ws_route)
        .or(icons_route)
        .or(overlay_route)
        .or(banners_route)
        .or(check_update_route)
        .or(metrics_route)
        .or(webhook_routes)
        .with(cors);

    log::info!("📍 Warp routes configured:");
    log::info!("   /ws              → WebSocket endpoint");
    log::info!("   /icons/*         → Static file serving");
    log::info!("   /overlay.html    → Overlay HTML");
    log::info!("   /banners         → Banners HTML");
    log::info!("   /check-update    → Update check");
    log::info!("   /metrics         → Authenticated Prometheus workflow metrics");
    log::info!("   /webhook/:id     → Production webhooks");
    log::info!("   /webhook-test/:id → Test mode webhooks");

    let preferred_addr = SocketAddr::new(identity_ip, port);
    let (primary_addr, primary_server) = match warp::serve(routes.clone())
        .try_bind_ephemeral(preferred_addr)
    {
        Ok(binding) => binding,
        Err(preferred_error) => {
            log::warn!(
                "Preferred VortiDeck port {} is unavailable on {}: {}. Asking the OS for a free port.",
                port,
                identity_ip,
                preferred_error
            );
            match warp::serve(routes.clone()).try_bind_ephemeral(SocketAddr::new(identity_ip, 0)) {
                Ok(binding) => binding,
                Err(fallback_error) => {
                    let message = format!(
                        "failed to bind preferred and OS-selected ports on {}: {}",
                        identity_ip, fallback_error
                    );
                    let _ = ready.send(Err(message));
                    return;
                }
            }
        }
    };

    let actual_port = primary_addr.port();
    let mut bound_ips = vec![identity_ip];
    let mut servers = vec![primary_server];
    for bind_ip in bind_ips
        .into_iter()
        .filter(|candidate| *candidate != identity_ip)
    {
        let addr = SocketAddr::new(bind_ip, actual_port);
        match warp::serve(routes.clone()).try_bind_ephemeral(addr) {
            Ok((bound_addr, server)) => {
                log::info!("Listening on {}", bound_addr);
                bound_ips.push(bind_ip);
                servers.push(server);
            }
            Err(error) => log::warn!(
                "Could not bind optional network endpoint {}: {}; continuing with other transports",
                addr,
                error
            ),
        }
    }
    log::info!("Unified server listening on {}", primary_addr);
    let _ = ready.send(Ok((actual_port, bound_ips)));
    futures_util::future::join_all(servers).await;
}

#[cfg(test)]
mod tests {
    use super::control_ws_origin_allowed;

    #[test]
    fn control_websocket_accepts_native_clients_and_app_webviews() {
        assert!(control_ws_origin_allowed(None));
        assert!(control_ws_origin_allowed(Some("tauri://localhost")));
        assert!(control_ws_origin_allowed(Some("http://tauri.localhost")));
        assert!(control_ws_origin_allowed(Some("https://tauri.localhost/")));
    }

    #[test]
    fn control_websocket_rejects_web_pages() {
        assert!(!control_ws_origin_allowed(Some("https://evil.example")));
        assert!(!control_ws_origin_allowed(Some("null")));
        assert!(!control_ws_origin_allowed(Some("http://192.168.1.20:9001")));
        assert!(!control_ws_origin_allowed(Some("http://tauri.localhost.evil.example")));
        assert!(!control_ws_origin_allowed(Some("")));
    }
}
