use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

pub type WebSocketSender = tokio::sync::mpsc::UnboundedSender<Message>;
pub type ClientId = String;

/// Central registry for managing different types of WebSocket clients
pub struct ClientRegistry {
    banner_clients: Arc<RwLock<HashMap<ClientId, WebSocketSender>>>,
    overlay_clients: Arc<RwLock<HashMap<ClientId, WebSocketSender>>>,
    mobile_clients: Arc<RwLock<HashMap<ClientId, WebSocketSender>>>,
}

impl ClientRegistry {
    pub fn new() -> Self {
        Self {
            banner_clients: Arc::new(RwLock::new(HashMap::new())),
            overlay_clients: Arc::new(RwLock::new(HashMap::new())),
            mobile_clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a banner client and return its ID for later removal
    pub async fn add_banner_client(&self, sender: WebSocketSender) -> ClientId {
        let client_id = Uuid::new_v4().to_string();
        let mut clients = self.banner_clients.write().await;
        clients.insert(client_id.clone(), sender);
        println!(
            "📋 Banner client {} added. Total banner clients: {}",
            client_id,
            clients.len()
        );
        client_id
    }

    /// Add an overlay client and return its ID for later removal
    pub async fn add_overlay_client(&self, sender: WebSocketSender) -> ClientId {
        let client_id = Uuid::new_v4().to_string();
        let mut clients = self.overlay_clients.write().await;
        clients.insert(client_id.clone(), sender);
        println!(
            "🎨 Overlay client {} added. Total overlay clients: {}",
            client_id,
            clients.len()
        );
        client_id
    }

    /// Add a mobile client (authenticated) and return its ID for later removal
    pub async fn add_mobile_client(&self, sender: WebSocketSender) -> ClientId {
        let client_id = Uuid::new_v4().to_string();
        let mut clients = self.mobile_clients.write().await;
        clients.insert(client_id.clone(), sender);
        println!(
            "📱 Mobile client {} added. Total mobile clients: {}",
            client_id,
            clients.len()
        );
        client_id
    }

    /// Send message to all banner clients, removing stale senders
    pub async fn send_to_banners(&self, message: &str) -> usize {
        let failed = {
            let clients = self.banner_clients.read().await;
            let mut failed_ids = Vec::new();
            let mut sent_count = 0;
            for (id, client) in clients.iter() {
                if client
                    .send(Message::Text(message.to_string().into()))
                    .is_ok()
                {
                    sent_count += 1;
                } else {
                    failed_ids.push(id.clone());
                }
            }
            if sent_count > 0 {
                println!("📋 Sent message to {} banner client(s)", sent_count);
            }
            (sent_count, failed_ids)
        };
        if !failed.1.is_empty() {
            let mut clients = self.banner_clients.write().await;
            for id in &failed.1 {
                clients.remove(id);
            }
            println!("🧹 Removed {} stale banner client(s)", failed.1.len());
        }
        failed.0
    }

    /// Send message to all overlay clients, removing stale senders
    pub async fn send_to_overlays(&self, message: &str) -> usize {
        let failed = {
            let clients = self.overlay_clients.read().await;
            let mut failed_ids = Vec::new();
            let mut sent_count = 0;
            for (id, client) in clients.iter() {
                if client
                    .send(Message::Text(message.to_string().into()))
                    .is_ok()
                {
                    sent_count += 1;
                } else {
                    failed_ids.push(id.clone());
                }
            }
            if sent_count > 0 {
                println!("🎨 Sent message to {} overlay client(s)", sent_count);
            }
            (sent_count, failed_ids)
        };
        if !failed.1.is_empty() {
            let mut clients = self.overlay_clients.write().await;
            for id in &failed.1 {
                clients.remove(id);
            }
            println!("🧹 Removed {} stale overlay client(s)", failed.1.len());
        }
        failed.0
    }

    /// Send message to all mobile clients, removing stale senders
    pub async fn send_to_mobile(&self, message: &str) -> usize {
        let failed = {
            let clients = self.mobile_clients.read().await;
            let mut failed_ids = Vec::new();
            let mut sent_count = 0;
            for (id, client) in clients.iter() {
                if client
                    .send(Message::Text(message.to_string().into()))
                    .is_ok()
                {
                    sent_count += 1;
                } else {
                    failed_ids.push(id.clone());
                }
            }
            if sent_count > 0 {
                println!("📱 Sent message to {} mobile client(s)", sent_count);
            }
            (sent_count, failed_ids)
        };
        if !failed.1.is_empty() {
            let mut clients = self.mobile_clients.write().await;
            for id in &failed.1 {
                clients.remove(id);
            }
            println!("🧹 Removed {} stale mobile client(s)", failed.1.len());
        }
        failed.0
    }

    /// Send message to all clients (broadcast)
    pub async fn broadcast_to_all(&self, message: &str) -> usize {
        let banner_count = self.send_to_banners(message).await;
        let overlay_count = self.send_to_overlays(message).await;
        let mobile_count = self.send_to_mobile(message).await;

        let total = banner_count + overlay_count + mobile_count;
        if total > 0 {
            println!("📡 Broadcast sent to {} total clients", total);
        }
        total
    }

    /// Remove a specific banner client by ID
    pub async fn remove_banner_client(&self, client_id: &ClientId) -> bool {
        let mut clients = self.banner_clients.write().await;
        if let Some(_) = clients.remove(client_id) {
            println!(
                "📋 Banner client {} removed. Total banner clients: {}",
                client_id,
                clients.len()
            );
            true
        } else {
            false
        }
    }

    /// Remove a specific overlay client by ID
    pub async fn remove_overlay_client(&self, client_id: &ClientId) -> bool {
        let mut clients = self.overlay_clients.write().await;
        if let Some(_) = clients.remove(client_id) {
            println!(
                "🎨 Overlay client {} removed. Total overlay clients: {}",
                client_id,
                clients.len()
            );
            true
        } else {
            false
        }
    }

    /// Remove a specific mobile client by ID
    pub async fn remove_mobile_client(&self, client_id: &ClientId) -> bool {
        let mut clients = self.mobile_clients.write().await;
        if let Some(_) = clients.remove(client_id) {
            println!(
                "📱 Mobile client {} removed. Total mobile clients: {}",
                client_id,
                clients.len()
            );
            true
        } else {
            false
        }
    }

    /// Clean up disconnected clients (fallback method)
    pub async fn cleanup_disconnected(&self) {
        // Remove closed channels from all client lists
        {
            let mut clients = self.banner_clients.write().await;
            let before_count = clients.len();
            clients.retain(|_id, client| !client.is_closed());
            let removed = before_count - clients.len();
            if removed > 0 {
                println!("🧹 Cleaned up {} disconnected banner client(s)", removed);
            }
        }
        {
            let mut clients = self.overlay_clients.write().await;
            let before_count = clients.len();
            clients.retain(|_id, client| !client.is_closed());
            let removed = before_count - clients.len();
            if removed > 0 {
                println!("🧹 Cleaned up {} disconnected overlay client(s)", removed);
            }
        }
        {
            let mut clients = self.mobile_clients.write().await;
            let before_count = clients.len();
            clients.retain(|_id, client| !client.is_closed());
            let removed = before_count - clients.len();
            if removed > 0 {
                println!("🧹 Cleaned up {} disconnected mobile client(s)", removed);
            }
        }
    }

    /// Get client counts for debugging
    pub async fn get_client_counts(&self) -> (usize, usize, usize) {
        let banner_count = self.banner_clients.read().await.len();
        let overlay_count = self.overlay_clients.read().await.len();
        let mobile_count = self.mobile_clients.read().await.len();
        (banner_count, overlay_count, mobile_count)
    }
}
