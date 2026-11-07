
use mdns_sd::{ServiceDaemon, ServiceEvent};
use serde::Serialize;
use std::collections::HashSet;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;


/// Represents a discovered WebSocket service.
#[derive(Debug, Serialize, Clone)]
pub struct ServiceInfo {
    pub name: String,           // mDNS fullname
    pub instance_name: String,  // Human-readable instance name
    pub hostname: String,       // Device hostname
    pub ip: String,             // IP address
    pub port: u16,              // Port number
    pub websocket_url: String,  // Full WebSocket URL
    pub desktop_id: Option<String>, // Desktop ID from TXT record
    pub os: Option<String>,     // Operating system from TXT record
}

/// Discovers WebSocket services using mDNS in the background.

#[tauri::command]
pub async fn discover_websocket() -> Result<Vec<ServiceInfo>, String> {
   
  

    // Create a channel to receive discovered services
    let (tx, mut rx) = mpsc::channel(10);

    // Spawn the discovery process in the background
    tokio::spawn(async move {

        let result = discover_services(tx, false).await;
        if let Err(e) = result {
            println!("Error during mDNS discovery: {}", e);
        }
    });

    // Collect services received from the channel
    let mut discovered_services = Vec::new();
    while let Some(service) = rx.recv().await {
        discovered_services.push(service);
    }

    if discovered_services.is_empty() {
        Err("No WebSocket services found.".to_string())
    } else {
        Ok(discovered_services)
    }
}

/// Handles the mDNS discovery process.
async fn discover_services(tx: mpsc::Sender<ServiceInfo>, tls_enabled: bool) -> Result<(), String> {
    let mdns = ServiceDaemon::new().map_err(|e| format!("Failed to create mDNS daemon: {}", e))?;
    let service_type = "_vortideck._tcp.local.";
    let receiver = mdns
        .browse(service_type)
        .map_err(|e| format!("Failed to browse services: {}", e))?;

    let timeout = Duration::from_secs(3); // 3 seconds - mDNS responses are fast
    let start_time = Instant::now();
    let mut discovered_set = HashSet::new(); // Deduplication using a HashSet

    println!("Starting mDNS discovery for type: {}", service_type);
    // log::info!("Starting mDNS discovery for type: {}", service_type);

    // Process events with proper cleanup handling
    let mut should_continue = true;
    while should_continue {
        // Use try_recv with timeout to allow for graceful shutdown
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(event) => {
                if start_time.elapsed() > timeout {
                    println!("Timeout reached, stopping mDNS discovery...");
                    break;
                }

                match event {
                    ServiceEvent::ServiceResolved(info) => {
                        println!("Service resolved: {:?}", info);

                        // Extract all service information
                        let ip = info.get_addresses()
                            .iter()
                            .next()
                            .map(|addr| addr.to_string())
                            .unwrap_or_else(|| "Unknown".to_string());

                        let port = info.get_port();
                        let service_name = info.get_fullname().to_string();
                        // Trim .local. suffix from hostname
                        let hostname = info.get_hostname()
                            .trim_end_matches(".local.")
                            .trim_end_matches(".local")
                            .trim_end_matches('.')
                            .to_string();

                        // Extract instance name from fullname (e.g., "vortideck_1._vortideck._tcp.local." -> "vortideck_1")
                        let instance_name = service_name
                            .split('.')
                            .next()
                            .unwrap_or("Unknown Desktop")
                            .to_string();

                        // Build the WebSocket URL
                        let websocket_url = if tls_enabled {
                            format!("wss://{}:{}/ws", ip, port)
                        } else {
                            format!("ws://{}:{}/ws", ip, port)
                        };

                        // Extract TXT properties (desktop_id, os, etc.)
                        let txt_properties = info.get_properties();
                        let desktop_id = txt_properties
                            .get("desktop_id")
                            .map(|v| v.val_str().to_string());
                        let os = txt_properties
                            .get("os")
                            .map(|v| v.val_str().to_string());

                        // Deduplicate using the fullname
                        if discovered_set.insert(service_name.clone()) {
                            println!("WebSocket service found: {} at {}", instance_name, websocket_url);
                            if let Some(ref id) = desktop_id {
                                println!("  └─ Desktop ID: {}", id);
                            }
                            if let Some(ref os_name) = os {
                                println!("  └─ OS: {}", os_name);
                            }

                            let service_info = ServiceInfo {
                                name: service_name,
                                instance_name,
                                hostname,
                                ip,
                                port,
                                websocket_url,
                                desktop_id,
                                os,
                            };

                            if tx.send(service_info).await.is_err() {
                                println!("Receiver dropped; stopping discovery...");
                                should_continue = false;
                            }
                            // 🔥 REMOVED: should_continue = false; // Now continues to find ALL services
                        }
                    }
                    ServiceEvent::ServiceRemoved(..) => {
                        println!("Service removed: {:?}", event);
                    }
                    _ => {
                        // println!("Received other mDNS event: {:?}", event);
                    }
                }
            }
            Err(_) => {
                // Timeout or error - check if we should continue
                if start_time.elapsed() > timeout {
                    println!("Timeout reached, stopping mDNS discovery...");
                    break;
                }
            }
        }
    }

    // Ensure proper cleanup order: stop browsing first
    if let Err(e) = mdns.stop_browse(service_type) {
        println!("Warning: Failed to stop browsing: {}", e);
    }

    // Explicitly drop the receiver before the daemon
    drop(receiver);

    // Small delay to allow cleanup
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Now drop the daemon
    drop(mdns);

    println!("mDNS discovery finished.");

    Ok(())
}
