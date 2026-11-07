//! AI Provider abstraction for multi-provider support
//!
//! Supports OpenAI, Anthropic Claude, and extensible for future providers

use serde::{Deserialize, Serialize};

/// AI Provider type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AIProvider {
    /// OpenAI (GPT models)
    OpenAI {
        api_key: String,
        base_url: Option<String>,
    },
    /// Anthropic Claude
    Anthropic {
        api_key: String,
        base_url: Option<String>,
    },
    /// Azure OpenAI
    AzureOpenAI {
        api_key: String,
        endpoint: String,
        deployment: String,
    },
    /// Ollama - local LLM provider with native API
    Ollama {
        base_url: String,
        /// Available models from Ollama
        #[serde(default)]
        models: Vec<ModelInfo>,
    },
    /// Local/Custom provider with OpenAI-compatible API (LM Studio, etc.)
    Custom {
        name: String,
        base_url: String,
        api_key: Option<String>,
        /// Custom model capabilities from config
        #[serde(default)]
        models: Vec<ModelInfo>,
    },
}

impl AIProvider {
    /// Create AIProvider from ModelConfig
    pub fn from_model_config(
        config: &crate::services::mcp::config::ModelConfig,
    ) -> Result<Self, String> {
        let provider_type = config.provider.to_lowercase();

        match provider_type.as_str() {
            "openai" => Ok(AIProvider::OpenAI {
                api_key: config.api_key.clone(),
                base_url: if config.url.is_empty() {
                    None
                } else {
                    Some(config.url.clone())
                },
            }),
            "anthropic" => Ok(AIProvider::Anthropic {
                api_key: config.api_key.clone(),
                base_url: if config.url.is_empty() {
                    None
                } else {
                    Some(config.url.clone())
                },
            }),
            "azure" => {
                let deployment = config
                    .deployment
                    .clone()
                    .ok_or_else(|| "Azure provider requires deployment name".to_string())?;
                Ok(AIProvider::AzureOpenAI {
                    api_key: config.api_key.clone(),
                    endpoint: config.url.clone(),
                    deployment,
                })
            }
            "ollama" => Ok(AIProvider::Ollama {
                base_url: config.url.clone(),
                models: vec![],
            }),
            "custom" | _ => Ok(AIProvider::Custom {
                name: config.name.clone(),
                base_url: config.url.clone(),
                api_key: if config.api_key.is_empty() {
                    None
                } else {
                    Some(config.api_key.clone())
                },
                models: vec![],
            }),
        }
    }

    pub fn provider_name(&self) -> &str {
        match self {
            AIProvider::OpenAI { .. } => "OpenAI",
            AIProvider::Anthropic { .. } => "Anthropic",
            AIProvider::AzureOpenAI { .. } => "Azure OpenAI",
            AIProvider::Ollama { .. } => "Ollama",
            AIProvider::Custom { name, .. } => name,
        }
    }

    pub fn default_model(&self) -> &str {
        match self {
            AIProvider::OpenAI { .. } => "gpt-4",
            AIProvider::Ollama { .. } => "llama2",
            AIProvider::Anthropic { .. } => "claude-3-5-sonnet-20241022",
            AIProvider::AzureOpenAI { .. } => "gpt-4",
            AIProvider::Custom { .. } => "default",
        }
    }

    pub fn get_model_info(&self, model_id: &str) -> Option<ModelInfo> {
        self.available_models()
            .into_iter()
            .find(|m| m.id == model_id)
    }

    pub fn available_models(&self) -> Vec<ModelInfo> {
        match self {
            AIProvider::OpenAI { .. } => vec![
                ModelInfo {
                    id: "gpt-4".to_string(),
                    name: "GPT-4".to_string(),
                    description: "Most capable model, best for complex tasks".to_string(),
                    context_window: 8192,
                    supports_temperature: true,
                    supports_streaming: true,
                    supports_tools: true,
                },
                ModelInfo {
                    id: "gpt-4-turbo".to_string(),
                    name: "GPT-4 Turbo".to_string(),
                    description: "Faster GPT-4 with 128K context".to_string(),
                    context_window: 128000,
                    supports_temperature: true,
                    supports_streaming: true,
                    supports_tools: true,
                },
                ModelInfo {
                    id: "gpt-3.5-turbo".to_string(),
                    name: "GPT-3.5 Turbo".to_string(),
                    description: "Fast and efficient for simple tasks".to_string(),
                    context_window: 16385,
                    supports_temperature: true,
                    supports_streaming: true,
                    supports_tools: true,
                },
                ModelInfo {
                    id: "gpt-5".to_string(),
                    name: "GPT-5".to_string(),
                    description: "Reasoning model - no temperature control".to_string(),
                    context_window: 128000,
                    supports_temperature: false,
                    supports_streaming: false,
                    supports_tools: true,
                },
                ModelInfo {
                    id: "gpt-4.1".to_string(),
                    name: "GPT-4.1".to_string(),
                    description: "Latest GPT-4 with improved capabilities".to_string(),
                    context_window: 128000,
                    supports_temperature: true,
                    supports_streaming: true,
                    supports_tools: true,
                },
                ModelInfo {
                    id: "o1-preview".to_string(),
                    name: "o1-preview".to_string(),
                    description: "Reasoning model - no temperature control".to_string(),
                    context_window: 128000,
                    supports_temperature: false,
                    supports_streaming: false,
                    supports_tools: true,
                },
                ModelInfo {
                    id: "o1-mini".to_string(),
                    name: "o1-mini".to_string(),
                    description: "Faster reasoning model - no temperature control".to_string(),
                    context_window: 128000,
                    supports_temperature: false,
                    supports_streaming: false,
                    supports_tools: true,
                },
                ModelInfo {
                    id: "o1".to_string(),
                    name: "o1".to_string(),
                    description: "Full reasoning model - no temperature control".to_string(),
                    context_window: 200000,
                    supports_temperature: false,
                    supports_streaming: false,
                    supports_tools: true,
                },
            ],
            AIProvider::Anthropic { .. } => vec![
                ModelInfo {
                    id: "claude-3-5-sonnet-20241022".to_string(),
                    name: "Claude 3.5 Sonnet".to_string(),
                    description: "Most intelligent model, best for complex tasks".to_string(),
                    context_window: 200000,
                    supports_temperature: true,
                    supports_streaming: true,
                    supports_tools: true,
                },
                ModelInfo {
                    id: "claude-3-opus-20240229".to_string(),
                    name: "Claude 3 Opus".to_string(),
                    description: "Powerful model for demanding tasks".to_string(),
                    context_window: 200000,
                    supports_temperature: true,
                    supports_streaming: true,
                    supports_tools: true,
                },
                ModelInfo {
                    id: "claude-3-sonnet-20240229".to_string(),
                    name: "Claude 3 Sonnet".to_string(),
                    description: "Balanced performance and speed".to_string(),
                    context_window: 200000,
                    supports_temperature: true,
                    supports_streaming: true,
                    supports_tools: true,
                },
                ModelInfo {
                    id: "claude-3-haiku-20240307".to_string(),
                    name: "Claude 3 Haiku".to_string(),
                    description: "Fastest model for simple tasks".to_string(),
                    context_window: 200000,
                    supports_temperature: true,
                    supports_streaming: true,
                    supports_tools: true,
                },
            ],
            AIProvider::AzureOpenAI { .. } => vec![ModelInfo {
                id: "gpt-4".to_string(),
                name: "GPT-4 (Azure)".to_string(),
                description: "Azure-hosted GPT-4".to_string(),
                context_window: 8192,
                supports_temperature: true,
                supports_streaming: true,
                supports_tools: true,
            }],
            AIProvider::Ollama { models, .. } => {
                if models.is_empty() {
                    // Fallback if no models configured
                    vec![ModelInfo {
                        id: "llama2".to_string(),
                        name: "Llama 2".to_string(),
                        description: "Default Ollama model".to_string(),
                        context_window: 4096,
                        supports_temperature: true,
                        supports_streaming: true,
                        supports_tools: false,
                    }]
                } else {
                    models.clone()
                }
            }
            AIProvider::Custom { models, .. } => {
                if models.is_empty() {
                    // Fallback if no models configured
                    vec![ModelInfo {
                        id: "default".to_string(),
                        name: "Default Model".to_string(),
                        description: "Provider default model".to_string(),
                        context_window: 4096,
                        supports_temperature: true,
                        supports_streaming: true,
                        supports_tools: true,
                    }]
                } else {
                    models.clone()
                }
            }
        }
    }
}

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub context_window: usize,
    #[serde(default = "default_true")]
    pub supports_temperature: bool,
    #[serde(default = "default_true")]
    pub supports_streaming: bool,
    #[serde(default = "default_true")]
    pub supports_tools: bool,
}

fn default_true() -> bool {
    true
}

/// AI Configuration for assistant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantAIConfig {
    pub provider: AIProvider,
    pub model: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<usize>,
}

impl Default for AssistantAIConfig {
    fn default() -> Self {
        Self {
            provider: AIProvider::OpenAI {
                api_key: String::new(),
                base_url: None,
            },
            model: "gpt-4".to_string(),
            temperature: Some(0.7),
            max_tokens: Some(2000),
        }
    }
}

impl AssistantAIConfig {
    pub fn with_provider(provider: AIProvider) -> Self {
        let model = provider.default_model().to_string();
        Self {
            provider,
            model,
            ..Default::default()
        }
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }
}
