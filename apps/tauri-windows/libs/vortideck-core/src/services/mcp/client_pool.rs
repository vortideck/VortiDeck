use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::services::mcp::{
    client::{ChatClient, OllamaClient, OpenAIClient},
    provider::AIProvider,
    AnthropicClient, // Re-exported from anthropic_client module
};

/// Unique identifier for a chat client
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClientKey {
    pub provider: String,
    pub api_key_hash: String, // SHA-256 hash for security
    pub base_url: String,
}

impl ClientKey {
    /// Create a ClientKey from provider configuration
    pub fn from_provider(provider: &AIProvider) -> Self {
        match provider {
            AIProvider::OpenAI { api_key, base_url } => Self {
                provider: "openai".to_string(),
                api_key_hash: Self::hash_api_key(api_key),
                base_url: base_url
                    .clone()
                    .unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
            },
            AIProvider::Anthropic { api_key, base_url } => Self {
                provider: "anthropic".to_string(),
                api_key_hash: Self::hash_api_key(api_key),
                base_url: base_url
                    .clone()
                    .unwrap_or_else(|| "https://api.anthropic.com".to_string()),
            },
            AIProvider::AzureOpenAI {
                api_key,
                endpoint,
                deployment,
            } => Self {
                provider: format!("azure-{}", deployment),
                api_key_hash: Self::hash_api_key(api_key),
                base_url: endpoint.clone(),
            },
            AIProvider::Ollama { base_url, .. } => {
                Self {
                    provider: "ollama".to_string(),
                    api_key_hash: String::new(), // Ollama doesn't use API keys
                    base_url: base_url.clone(),
                }
            }
            AIProvider::Custom {
                name,
                base_url,
                api_key,
                ..
            } => Self {
                provider: format!("custom-{}", name),
                api_key_hash: api_key
                    .as_ref()
                    .map(|k| Self::hash_api_key(k))
                    .unwrap_or_default(),
                base_url: base_url.clone(),
            },
        }
    }

    /// Hash API key for secure storage (we don't want to use API key as hash map key directly)
    fn hash_api_key(api_key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(api_key.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

/// Pool of reusable chat clients to avoid duplicate HTTP connections
pub struct ClientPool {
    clients: Arc<RwLock<HashMap<ClientKey, Arc<dyn ChatClient>>>>,
}

// Manual Debug impl since trait objects don't implement Debug
impl std::fmt::Debug for ClientPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientPool")
            .field("clients", &"<ClientMap>")
            .finish()
    }
}

impl ClientPool {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or create a client for the given provider
    pub async fn get_or_create(
        &self,
        provider: &AIProvider,
    ) -> Result<Arc<dyn ChatClient>, String> {
        let key = ClientKey::from_provider(provider);

        // Try to get existing client (read lock)
        {
            let clients = self.clients.read().await;
            if let Some(client) = clients.get(&key) {
                println!(
                    "♻️ Reusing existing client: provider={}, url={}",
                    key.provider, key.base_url
                );
                return Ok(Arc::clone(client));
            }
        }

        // Create new client (write lock)
        let mut clients = self.clients.write().await;

        // Double-check pattern: another task might have created it while we waited for write lock
        if let Some(client) = clients.get(&key) {
            println!(
                "♻️ Client created by another task: provider={}",
                key.provider
            );
            return Ok(Arc::clone(client));
        }

        println!(
            "🆕 Creating new client: provider={}, url={}",
            key.provider, key.base_url
        );

        let client: Arc<dyn ChatClient> = match provider {
            AIProvider::OpenAI { api_key, base_url } => {
                Arc::new(OpenAIClient::new(api_key.clone(), base_url.clone()))
            }
            AIProvider::Anthropic { api_key, base_url } => {
                Arc::new(AnthropicClient::new(api_key.clone(), base_url.clone()))
            }
            AIProvider::AzureOpenAI {
                api_key, endpoint, ..
            } => Arc::new(OpenAIClient::new(api_key.clone(), Some(endpoint.clone()))),
            AIProvider::Ollama { base_url, .. } => Arc::new(OllamaClient::new(base_url.clone())),
            AIProvider::Custom {
                base_url, api_key, ..
            } => Arc::new(OpenAIClient::new(
                api_key.clone().unwrap_or_default(),
                Some(base_url.clone()),
            )),
        };

        clients.insert(key.clone(), Arc::clone(&client));
        println!("✅ Client pool now has {} clients", clients.len());

        Ok(client)
    }

    /// Remove a client from the pool (useful for forced refresh)
    pub async fn remove(&self, provider: &AIProvider) {
        let key = ClientKey::from_provider(provider);
        let mut clients = self.clients.write().await;

        if clients.remove(&key).is_some() {
            println!("🗑️ Removed client from pool: provider={}", key.provider);
        }
    }

    /// Get pool statistics
    pub async fn stats(&self) -> (usize, Vec<String>) {
        let clients = self.clients.read().await;
        let count = clients.len();
        let providers: Vec<String> = clients.keys().map(|k| k.provider.clone()).collect();
        (count, providers)
    }

    /// Clear all clients (useful for testing or forced cleanup)
    pub async fn clear(&self) {
        let mut clients = self.clients.write().await;
        let count = clients.len();
        clients.clear();
        println!("🧹 Cleared {} clients from pool", count);
    }
}

impl Default for ClientPool {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ClientPool {
    fn clone(&self) -> Self {
        Self {
            clients: Arc::clone(&self.clients),
        }
    }
}
