use crate::services::flow::executor::{TriggerContext, VariableScope};
use crate::services::flow::traits::{
    ExecutionContext, FlowNode, FlowNodeProvider, FlowTriggerNode, TriggerType,
};
use crate::services::webhook::presets;
use async_trait::async_trait;
use chrono::Utc;
use serde_json::{json, Value};

/// AudioPlayNode - plays an audio file
pub struct AudioPlayNode;

#[async_trait]
impl FlowNode for AudioPlayNode {
    fn id(&self) -> String {
        "audio-play".to_string()
    }

    fn name(&self) -> String {
        "Play Audio".to_string()
    }

    fn description(&self) -> String {
        "Plays an audio file".to_string()
    }

    fn category(&self) -> String {
        "AUDIO".to_string()
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "fileLocation": {
                    "type": "string",
                    "format": "file",
                    "description": "Path to the audio file",
                    "typeOptions": {
                        "acceptedTypes": [".mp3", ".wav", ".ogg", ".flac", ".m4a"]
                    }
                },
                "id": {
                    "type": "string",
                    "description": "Unique identifier for this audio"
                },
                "volume": {
                    "type": "number",
                    "description": "Volume level (0-100)",
                    "default": 100,
                    "minimum": 0,
                    "maximum": 100,
                    "multipleOf": 1
                }
            },
            "required": ["fileLocation", "id"]
        })
    }

    fn type_color(&self) -> String {
        "#4CAF50".to_string() // Green
    }

    async fn execute(&self, params: Value) -> Result<Value, String> {
        // Extract parameters
        let file_location = params["fileLocation"].as_str().unwrap_or("default.mp3");
        let volume = params["volume"].as_u64().unwrap_or(100);
        let id = params["id"].as_str().unwrap_or("default_audio");

        println!("======================================================");
        println!(
            "🔊 STARTING AUDIO: {} (ID: {}, Volume: {})",
            file_location, id, volume
        );
        println!("======================================================");

        // Use rodio for audio playback
        use rodio::{Decoder, OutputStream, Sink};
        use std::fs::File;
        use std::io::BufReader;
        use std::time::Duration;

        // Instead of using channels and complex communication,
        // we'll use a simpler approach with a blocking task
        let file_path = file_location.to_string();
        let vol = volume as f32 / 100.0;
        let audio_id = id.to_string();

        // This will execute the audio playback in a blocking thread
        // but we won't wait for it to complete - just let it play in background
        tokio::task::spawn_blocking(move || {
            println!(
                "🎵 Audio thread started for: {} (ID: {})",
                file_path, audio_id
            );

            // Try to get audio output device
            match OutputStream::try_default() {
                Ok((stream, stream_handle)) => {
                    // Try to open the file
                    match File::open(&file_path) {
                        Ok(file) => {
                            let reader = BufReader::new(file);

                            // Try to decode the audio file
                            match Decoder::new(reader) {
                                Ok(source) => {
                                    // Try to create a sink for playback
                                    match Sink::try_new(&stream_handle) {
                                        Ok(sink) => {
                                            // Set volume
                                            sink.set_volume(vol);

                                            // Add source to sink
                                            sink.append(source);

                                            // Start playing
                                            println!(
                                                "▶️ Now playing audio: {} (ID: {})",
                                                file_path, audio_id
                                            );
                                            sink.play();

                                            // Wait for the audio to complete playback
                                            println!("⏳ Waiting for audio to complete...");
                                            sink.sleep_until_end();

                                            println!(
                                                "✅ Audio playback completed: {} (ID: {})",
                                                file_path, audio_id
                                            );
                                            println!("======================================================");
                                        }
                                        Err(e) => {
                                            println!("❌ Failed to create audio sink: {:?}", e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    println!("❌ Failed to decode audio file: {:?}", e);
                                }
                            }
                        }
                        Err(e) => {
                            println!("❌ Failed to open file ({}): {:?}", file_path, e);
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Failed to open audio output stream: {:?}", e);
                }
            }
        });

        // We're launching audio in the background and returning immediately
        // This is necessary since we're in an async function but rodio's sleep_until_end is blocking
        println!("🔈 Audio playback launched in background for ID '{}'", id);

        // Sleep for just a moment to let the audio thread start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        Ok(json!({ "success": true, "fileLocation": file_location, "volume": volume, "id": id }))
    }
}

/// AudioStopNode - stops an audio file
pub struct AudioStopNode;

#[async_trait]
impl FlowNode for AudioStopNode {
    fn id(&self) -> String {
        "audio-stop".to_string()
    }

    fn name(&self) -> String {
        "Stop Audio".to_string()
    }

    fn description(&self) -> String {
        "Stops playing an audio file".to_string()
    }

    fn category(&self) -> String {
        "AUDIO".to_string()
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "ID of the audio to stop"
                }
            },
            "required": ["id"]
        })
    }

    fn type_color(&self) -> String {
        "#f44336".to_string() // Red
    }

    async fn execute(&self, params: Value) -> Result<Value, String> {
        // For now, just log that we'd stop the audio
        let id = params["id"].as_str().unwrap_or("unknown");
        println!("Stopping audio with id: {}", id);

        // In a real implementation, we would track playing sources by ID
        // and stop the correct one

        Ok(json!({ "success": true, "id": id }))
    }
}

/// DelayNode - waits for a specified duration
pub struct DelayNode;

#[async_trait]
impl FlowNode for DelayNode {
    fn id(&self) -> String {
        "delay".to_string()
    }

    fn name(&self) -> String {
        "Delay".to_string()
    }

    fn description(&self) -> String {
        "Waits for a specified duration before continuing".to_string()
    }

    fn category(&self) -> String {
        "CONTROL".to_string()
    }

    fn category_display_name(&self) -> Option<String> {
        Some("Control Flow".to_string())
    }

    fn subcategory(&self) -> Option<String> {
        Some("Timing".to_string())
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "duration": {
                    "type": "number",
                    "description": "Duration in milliseconds",
                    "default": 1000
                }
            },
            "required": ["duration"]
        })
    }

    fn type_color(&self) -> String {
        "#FF9800".to_string() // Orange
    }

    async fn execute(&self, params: Value) -> Result<Value, String> {
        // Default implementation for backward compatibility
        let mut runtime_vars = crate::services::flow::executor::RuntimeVariables::new();
        let mut ctx = crate::services::flow::traits::ExecutionContext {
            flow_id: "",
            node_id: "",
            variables: &mut runtime_vars,
            progress_callback: None,
        };
        self.execute_with_context(params, &mut ctx).await
    }

    async fn execute_with_context(
        &self,
        params: Value,
        ctx: &mut crate::services::flow::traits::ExecutionContext<'_>,
    ) -> Result<Value, String> {
        let duration = params["duration"].as_u64().unwrap_or(1000);

        println!("⏳ DELAY NODE: Starting delay for {}ms", duration);

        // For longer delays, provide progress updates
        if duration > 1000 {
            let start_time = std::time::Instant::now();
            let total_duration = std::time::Duration::from_millis(duration);

            // Break into smaller chunks for progress reporting
            // Update every 200ms minimum to avoid UI lag (matching download throttle)
            let chunk_size = std::cmp::max(200, std::cmp::min(500, duration / 10));
            let mut remaining = duration;

            while remaining > 0 {
                let sleep_time = std::cmp::min(chunk_size, remaining);
                tokio::time::sleep(tokio::time::Duration::from_millis(sleep_time)).await;
                remaining -= sleep_time;

                let elapsed = start_time.elapsed();
                let progress =
                    (elapsed.as_millis() as f64 / total_duration.as_millis() as f64) * 100.0;

                // Report progress via callback
                if let Some(callback) = &ctx.progress_callback {
                    let elapsed_secs = elapsed.as_secs_f64();
                    let total_secs = total_duration.as_secs_f64();
                    let message = format!("{:.1}s / {:.1}s", elapsed_secs, total_secs);
                    callback(progress, message);
                }
            }

            println!("✅ DELAY COMPLETED: {}ms delay finished", duration);
        } else {
            // For short delays, just wait
            tokio::time::sleep(tokio::time::Duration::from_millis(duration)).await;
            println!("✅ DELAY COMPLETED: {}ms delay finished", duration);
        }

        Ok(
            json!({ "success": true, "duration": duration, "completed_at": chrono::Utc::now().to_rfc3339() }),
        )
    }
}

/// ApprovalNode is a registry descriptor for the executor's durable human
/// approval primitive. Its execution is intercepted by FlowExecutor so the
/// wait is persisted and can be resolved after a process restart.
pub struct ApprovalNode;

#[async_trait]
impl FlowNode for ApprovalNode {
    fn id(&self) -> String {
        "approval".to_string()
    }

    fn name(&self) -> String {
        "Human Approval".to_string()
    }

    fn description(&self) -> String {
        "Pauses the workflow until an operator approves or denies it".to_string()
    }

    fn category(&self) -> String {
        "CONTROL".to_string()
    }

    fn category_display_name(&self) -> Option<String> {
        Some("Control Flow".to_string())
    }

    fn subcategory(&self) -> Option<String> {
        Some("Human Interaction".to_string())
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "Short title shown to the operator",
                    "default": "Workflow approval"
                },
                "message": {
                    "type": "string",
                    "description": "Decision context shown to the operator"
                },
                "timeoutMs": {
                    "type": "number",
                    "description": "Optional fail-closed timeout in milliseconds; 0 waits indefinitely",
                    "minimum": 0,
                    "default": 0
                },
                "severity": {
                    "type": "string",
                    "description": "Risk level shown to approvers",
                    "enum": ["normal", "high", "critical"],
                    "default": "normal"
                },
                "channels": {
                    "type": "array",
                    "description": "Installed notification adapters to request. Email and webhook adapters appear here only when configured.",
                    "items": { "type": "string", "enum": ["desktop", "mobile"] },
                    "default": ["desktop", "mobile"]
                },
                "allowedDecisionChannels": {
                    "type": "array",
                    "description": "Authenticated transports allowed to submit the human decision",
                    "items": { "type": "string", "enum": ["desktop", "mobile_direct", "mobile_relay"] },
                    "default": ["desktop", "mobile_direct", "mobile_relay"]
                },
                "requestSnapshot": {
                    "type": "object",
                    "description": "Non-secret structured summary of the protected operation"
                }
            }
        })
    }

    fn type_color(&self) -> String {
        "#F59E0B".to_string()
    }

    fn outputs_schema(&self) -> Vec<crate::services::flow::traits::NodeOutput> {
        vec![
            crate::services::flow::traits::NodeOutput {
                id: "approved".to_string(),
                name: "Approved".to_string(),
                output_type: "flow".to_string(),
                description: Some("Continues only after explicit approval".to_string()),
                ..Default::default()
            },
            crate::services::flow::traits::NodeOutput {
                id: "denied".to_string(),
                name: "Denied".to_string(),
                output_type: "flow".to_string(),
                description: Some("Continues after an explicit denial".to_string()),
                ..Default::default()
            },
            crate::services::flow::traits::NodeOutput {
                id: "timed_out".to_string(),
                name: "Timed out".to_string(),
                output_type: "flow".to_string(),
                description: Some(
                    "Continues when no decision arrived before the deadline".to_string(),
                ),
                ..Default::default()
            },
            crate::services::flow::traits::NodeOutput {
                id: "decision".to_string(),
                name: "Decision".to_string(),
                output_type: "string".to_string(),
                description: Some("approved, denied, timed_out, or cancelled".to_string()),
                ..Default::default()
            },
            crate::services::flow::traits::NodeOutput {
                id: "isApproved".to_string(),
                name: "Is Approved".to_string(),
                output_type: "boolean".to_string(),
                description: Some("True only for an explicit approval".to_string()),
                ..Default::default()
            },
            crate::services::flow::traits::NodeOutput {
                id: "wakeId".to_string(),
                name: "Wake ID".to_string(),
                output_type: "string".to_string(),
                description: Some("Durable approval receipt identifier".to_string()),
                ..Default::default()
            },
            crate::services::flow::traits::NodeOutput {
                id: "decidedAt".to_string(),
                name: "Decided At".to_string(),
                output_type: "string".to_string(),
                description: Some("RFC 3339 decision time".to_string()),
                ..Default::default()
            },
            crate::services::flow::traits::NodeOutput {
                id: "decidedBy".to_string(),
                name: "Decided By".to_string(),
                output_type: "object".to_string(),
                description: Some("Authenticated approver identity and channel".to_string()),
                ..Default::default()
            },
            crate::services::flow::traits::NodeOutput {
                id: "comment".to_string(),
                name: "Comment".to_string(),
                output_type: "string".to_string(),
                description: Some("Optional approver comment".to_string()),
                ..Default::default()
            },
        ]
    }

    async fn execute(&self, _params: Value) -> Result<Value, String> {
        Err("Human Approval must be executed by the durable workflow runtime".to_string())
    }
}

/// WebhookTriggerNode - triggered by webhook
pub struct WebhookTriggerNode;

#[async_trait]
impl FlowNode for WebhookTriggerNode {
    fn id(&self) -> String {
        "webhook-trigger".to_string()
    }

    fn name(&self) -> String {
        "Webhook Trigger".to_string()
    }

    fn description(&self) -> String {
        "Triggers the flow when a webhook is received".to_string()
    }

    fn category(&self) -> String {
        "TRIGGER".to_string()
    }

    fn parameters_schema(&self) -> Value {
        // Get available services from presets
        let presets = presets::get_webhook_presets();
        let services: Vec<String> = presets.keys().cloned().collect();

        json!({
            "type": "object",
            "properties": {
                "service": {
                    "type": "string",
                    "enum": services,
                    "default": "custom",
                    "description": "Webhook service provider"
                },
                "event_type": {
                    "type": "string",
                    "description": "Type of event to listen for (depends on service)"
                },
                "config": {
                    "type": "object",
                    "description": "Service-specific configuration",
                    "properties": {
                        "broadcaster_id": {
                            "type": "string",
                            "description": "Twitch broadcaster ID (for Twitch)"
                        },
                        "client_id": {
                            "type": "string",
                            "description": "API client ID (for services with OAuth)"
                        },
                        "access_token": {
                            "type": "string",
                            "description": "Access token for API authentication"
                        },
                        "secret": {
                            "type": "string",
                            "description": "Webhook secret for signature verification"
                        },
                        "owner": {
                            "type": "string",
                            "description": "Repository owner (for GitHub)"
                        },
                        "repo": {
                            "type": "string",
                            "description": "Repository name (for GitHub)"
                        }
                    }
                },
                "auto_register": {
                    "type": "boolean",
                    "default": false,
                    "description": "Automatically register webhook with service API (requires credentials)"
                }
            },
            "required": ["service", "event_type"]
        })
    }

    fn type_color(&self) -> String {
        "#FF6B6B".to_string()
    }

    async fn execute(&self, params: Value) -> Result<Value, String> {
        // Extract webhook data if it was injected by trigger_flow_from_webhook
        let webhook_data = params.get("webhook_data").cloned();

        Ok(json!({
            "triggered": true,
            "type": "webhook",
            "data": webhook_data
        }))
    }
}

// Implement FlowTriggerNode for WebhookTriggerNode
#[async_trait]
impl FlowTriggerNode for WebhookTriggerNode {
    async fn on_activate(&self, context: &TriggerContext) -> Result<(), String> {
        log::info!(
            "🔗 Activating webhook trigger for flow: {}, node: {}",
            context.flow_id,
            context.node_id
        );

        // Generate webhook URL path
        let webhook_path = format!("/webhook/{}-{}", context.flow_id, context.node_id);

        // Save webhook path to trigger state
        context
            .set_state("webhook_path", json!(webhook_path))
            .await?;

        log::info!("✅ Webhook activated: {}", webhook_path);
        log::info!("   Full URL: http://localhost:3000{}", webhook_path);

        Ok(())
    }

    async fn on_deactivate(&self, context: &TriggerContext) -> Result<(), String> {
        log::info!(
            "⏸️ Deactivating webhook trigger for flow: {}, node: {}",
            context.flow_id,
            context.node_id
        );

        // Get webhook path from state
        if let Some(webhook_path) = context.get_state("webhook_path").await? {
            log::info!("🗑️ Webhook deactivated: {}", webhook_path);
        }

        Ok(())
    }

    fn trigger_type(&self) -> TriggerType {
        TriggerType::Webhook
    }

    fn natural_source(&self) -> crate::services::flow::executor::NaturalSource {
        use crate::services::flow::executor::NaturalSource;

        // Determine natural source based on service type
        // This would ideally be based on node parameters, but for now we default to Any
        // In a real implementation, you'd check the service parameter:
        //
        // - Twitch, Kick, YouTube → BackendOnly (streaming events)
        // - Discord, GitHub → LocalOnly (personal integrations)
        // - Custom webhooks → Any (either with de-duplication)

        NaturalSource::Any
    }
}

/// ButtonTriggerNode - triggered by button press
pub struct ButtonTriggerNode;

#[async_trait]
impl FlowNode for ButtonTriggerNode {
    fn id(&self) -> String {
        "button-trigger".to_string()
    }

    fn name(&self) -> String {
        "Button Trigger".to_string()
    }

    fn description(&self) -> String {
        "Triggers the flow when a button is pressed".to_string()
    }

    fn category(&self) -> String {
        "TRIGGER".to_string()
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    fn type_color(&self) -> String {
        "#4ECDC4".to_string()
    }

    async fn execute(&self, _params: Value) -> Result<Value, String> {
        Ok(json!({ "triggered": true, "type": "button" }))
    }
}

/// BranchNode - conditional branching
pub struct BranchNode;

#[async_trait]
impl FlowNode for BranchNode {
    fn id(&self) -> String {
        "branch".to_string()
    }

    fn name(&self) -> String {
        "Branch Condition".to_string()
    }

    fn description(&self) -> String {
        "Branches flow based on a condition".to_string()
    }

    fn category(&self) -> String {
        "CONTROL".to_string()
    }

    fn category_display_name(&self) -> Option<String> {
        Some("Control Flow".to_string())
    }

    fn subcategory(&self) -> Option<String> {
        Some("Routing".to_string())
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "condition": {
                    "type": "boolean",
                    "description": "Condition to evaluate",
                    "default": false
                }
            },
            "required": ["condition"]
        })
    }

    fn type_color(&self) -> String {
        "#9C27B0".to_string() // Purple
    }

    fn outputs_schema(&self) -> Vec<crate::services::flow::traits::NodeOutput> {
        vec![
            crate::services::flow::traits::NodeOutput {
                id: "true_branch".to_string(),
                name: "True Branch".to_string(),
                output_type: "flow".to_string(),
                description: Some("Executes when condition is true".to_string()),
                ..Default::default()
            },
            crate::services::flow::traits::NodeOutput {
                id: "false_branch".to_string(),
                name: "False Branch".to_string(),
                output_type: "flow".to_string(),
                description: Some("Executes when condition is false".to_string()),
                ..Default::default()
            },
        ]
    }

    async fn execute(&self, params: Value) -> Result<Value, String> {
        let condition = params["condition"].as_bool().unwrap_or(false);
        Ok(json!({
            "success": true,
            "branch": condition,
            "selectedRoute": if condition { "true_branch" } else { "false_branch" }
        }))
    }
}

/// AI Text Generator Node
pub struct AiTextGeneratorNode;

#[async_trait]
impl FlowNode for AiTextGeneratorNode {
    fn id(&self) -> String {
        "ai-text-generator".to_string()
    }

    fn name(&self) -> String {
        "AI Text Generator".to_string()
    }

    fn description(&self) -> String {
        "Generates text using AI".to_string()
    }

    fn category(&self) -> String {
        "AI".to_string()
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "user_message": {
                    "type": "string",
                    "description": "User message for text generation"
                },
                "context": {
                    "type": "object",
                    "description": "Additional context",
                    "default": {}
                }
            },
            "required": ["user_message"]
        })
    }

    fn type_color(&self) -> String {
        "#8b5cf6".to_string()
    }

    fn outputs_schema(&self) -> Vec<crate::services::flow::traits::NodeOutput> {
        vec![crate::services::flow::traits::NodeOutput {
            id: "text_output".to_string(),
            name: "Text Output".to_string(),
            output_type: "string".to_string(),
            description: Some("Generated text content".to_string()),
            ..Default::default()
        }]
    }

    async fn execute(&self, _params: Value) -> Result<Value, String> {
        // This should never be called - AI nodes go through execute_ai_node()
        Err("AI nodes should be executed through the AI executor, not directly".to_string())
    }
}

/// AI Free Agent Node  
pub struct AiFreeAgentNode;

#[async_trait]
impl FlowNode for AiFreeAgentNode {
    fn id(&self) -> String {
        "hybrid-llm-free".to_string()
    }

    fn name(&self) -> String {
        "AI Agent (Free)".to_string()
    }

    fn description(&self) -> String {
        "AI agent that can use tools freely".to_string()
    }

    fn category(&self) -> String {
        "AI".to_string()
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "user_message": {
                    "type": "string",
                    "description": "Message for the AI agent"
                },
                "context": {
                    "type": "object",
                    "description": "Context for the agent",
                    "default": {}
                }
            },
            "required": ["user_message"]
        })
    }

    fn type_color(&self) -> String {
        "#8b5cf6".to_string()
    }

    fn outputs_schema(&self) -> Vec<crate::services::flow::traits::NodeOutput> {
        vec![crate::services::flow::traits::NodeOutput {
            id: "result".to_string(),
            name: "Result".to_string(),
            output_type: "string".to_string(),
            description: Some("Agent execution result and response".to_string()),
            ..Default::default()
        }]
    }

    async fn execute(&self, _params: Value) -> Result<Value, String> {
        // This should never be called - AI nodes go through execute_ai_node()
        Err("AI nodes should be executed through the AI executor, not directly".to_string())
    }
}

/// AI Orchestrator Node
pub struct AiOrchestratorNode;

#[async_trait]
impl FlowNode for AiOrchestratorNode {
    fn id(&self) -> String {
        "ai-orchestrator".to_string()
    }

    fn name(&self) -> String {
        "AI Orchestrator".to_string()
    }

    fn description(&self) -> String {
        "AI orchestrator for complex workflows".to_string()
    }

    fn category(&self) -> String {
        "AI".to_string()
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "user_message": {
                    "type": "string",
                    "description": "Goal for the orchestrator"
                },
                "previous_data": {
                    "type": "object",
                    "description": "Previous execution data",
                    "default": {}
                }
            },
            "required": ["user_message"]
        })
    }

    fn type_color(&self) -> String {
        "#3b82f6".to_string()
    }

    fn outputs_schema(&self) -> Vec<crate::services::flow::traits::NodeOutput> {
        vec![crate::services::flow::traits::NodeOutput {
            id: "result".to_string(),
            name: "Result".to_string(),
            output_type: "string".to_string(),
            description: Some("Orchestrator execution result".to_string()),
            ..Default::default()
        }]
    }

    async fn execute(&self, _params: Value) -> Result<Value, String> {
        // This should never be called - AI nodes go through execute_ai_node()
        Err("AI nodes should be executed through the AI executor, not directly".to_string())
    }
}

/// WidgetDemoNode - demonstrates all available widget types
pub struct WidgetDemoNode;

#[async_trait]
impl FlowNode for WidgetDemoNode {
    fn id(&self) -> String {
        "widget-demo".to_string()
    }

    fn name(&self) -> String {
        "Widget Demo".to_string()
    }

    fn description(&self) -> String {
        "Demonstrates all widget types, displayOptions (conditional visibility), and forceInput (connection-only ports)".to_string()
    }

    fn category(&self) -> String {
        "DEVELOPMENT".to_string()
    }

    fn category_display_name(&self) -> Option<String> {
        Some("Development".to_string())
    }

    fn subcategory(&self) -> Option<String> {
        Some("Schema Fixtures".to_string())
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "opacity": {
                    "type": "number",
                    "description": "Opacity level using slider widget",
                    "default": 50,
                    "minimum": 0,
                    "maximum": 100,
                    "multipleOf": 1
                },
                "backgroundColor": {
                    "type": "string",
                    "format": "color",
                    "description": "Background color using color picker",
                    "default": "#3B82F6",
                    "optional": true
                },
                "enabled": {
                    "type": "boolean",
                    "description": "Enable/disable using toggle widget",
                    "default": true
                },
                "notes": {
                    "type": "string",
                    "description": "Multi-line notes using textarea widget",
                    "maxLength": 500,
                    "default": "Enter your notes here...",
                    "optional": true
                },
                "mode": {
                    "type": "string",
                    "description": "Mode selection using dropdown",
                    "enum": ["normal", "advanced", "expert"],
                    "default": "normal"
                },
                "audioFile": {
                    "type": "string",
                    "format": "file",
                    "description": "Audio file using file picker widget",
                    "default": "",
                    "optional": true,
                    "typeOptions": {
                        "acceptedTypes": [".mp3", ".wav", ".ogg", ".flac"]
                    }
                },
                "tags": {
                    "type": "array",
                    "description": "Tags using multi-select widget",
                    "optional": true,
                    "items": {
                        "type": "string",
                        "enum": ["urgent", "important", "review", "done", "archived"]
                    },
                    "default": ["important"]
                },
                "authMode": {
                    "type": "string",
                    "description": "Authentication mode (demonstrates displayOptions)",
                    "enum": ["none", "apiKey", "oauth"],
                    "default": "none"
                },
                "apiKey": {
                    "type": "string",
                    "description": "API Key (only shows when authMode=apiKey)",
                    "default": "",
                    "optional": true,
                    "displayOptions": {
                        "show": {
                            "authMode": ["apiKey"]
                        }
                    }
                },
                "oauthClientId": {
                    "type": "string",
                    "description": "OAuth Client ID (only shows when authMode=oauth)",
                    "default": "",
                    "optional": true,
                    "displayOptions": {
                        "show": {
                            "authMode": ["oauth"]
                        }
                    }
                },
                "dataInput": {
                    "type": "object",
                    "description": "Data input (connection-only, demonstrates forceInput)",
                    "default": {},
                    "optional": true,
                    "forceInput": true
                }
            }
        })
    }

    fn type_color(&self) -> String {
        "#8b5cf6".to_string() // Purple
    }

    fn outputs_schema(&self) -> Vec<crate::services::flow::traits::NodeOutput> {
        use crate::services::flow::traits::{default_result_output, NodeOutput};
        vec![
            NodeOutput {
                id: "status".to_owned(),
                name: "status".to_owned(),
                output_type: "string".to_owned(),
                description: Some("Execution status".to_owned()),
                ..Default::default()
            },
            NodeOutput {
                id: "settings".to_owned(),
                name: "settings".to_owned(),
                output_type: "object".to_owned(),
                description: Some("All widget settings combined".to_owned()),
                ..Default::default()
            },
            NodeOutput {
                id: "message".to_owned(),
                name: "message".to_owned(),
                output_type: "string".to_owned(),
                description: Some("Summary message of widget values".to_owned()),
                ..Default::default()
            },
            default_result_output(),
        ]
    }

    async fn execute(&self, params: Value) -> Result<Value, String> {
        let opacity = params["opacity"].as_f64().unwrap_or(50.0);
        let bg_color = params
            .get("backgroundColor")
            .and_then(|v| v.as_str())
            .unwrap_or("#3B82F6");
        let enabled = params["enabled"].as_bool().unwrap_or(true);
        let notes = params.get("notes").and_then(|v| v.as_str()).unwrap_or("");
        let mode = params["mode"].as_str().unwrap_or("normal");
        let audio_file = params
            .get("audioFile")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let tags = params
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_else(|| "none".to_string());

        // New fields
        let auth_mode = params["authMode"].as_str().unwrap_or("none");
        let api_key = params.get("apiKey").and_then(|v| v.as_str()).unwrap_or("");
        let oauth_client_id = params
            .get("oauthClientId")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let data_input = params.get("dataInput").cloned().unwrap_or(json!({}));

        Ok(json!({
            "status": "success",
            "settings": {
                "opacity": opacity,
                "backgroundColor": bg_color,
                "enabled": enabled,
                "notes": notes,
                "mode": mode,
                "audioFile": audio_file,
                "tags": params.get("tags").cloned().unwrap_or(json!([])),
                "authMode": auth_mode,
                "apiKey": api_key,
                "oauthClientId": oauth_client_id,
                "dataInput": data_input
            },
            "message": format!(
                "Widget Demo: opacity={}, color={}, enabled={}, mode={}, auth={}, file={}, tags=[{}]",
                opacity, bg_color, enabled, mode, auth_mode,
                if audio_file.is_empty() { "none" } else { audio_file },
                tags
            )
        }))
    }
}

/// StandardNodeProvider - provides all standard nodes
pub struct StandardNodeProvider;

impl StandardNodeProvider {
    pub fn new() -> Self {
        Self {}
    }
}

/// RespondToWebhookNode - sends custom HTTP response for ResponseNode mode webhooks
pub struct RespondToWebhookNode;

#[async_trait]
impl FlowNode for RespondToWebhookNode {
    fn id(&self) -> String {
        "respond-to-webhook".to_string()
    }

    fn name(&self) -> String {
        "Respond to Webhook".to_string()
    }

    fn description(&self) -> String {
        "Sends a custom HTTP response for webhooks in ResponseNode mode. First ResponseNode wins!"
            .to_string()
    }

    fn category(&self) -> String {
        "WEBHOOK".to_string()
    }

    fn type_color(&self) -> String {
        "#9333ea".to_string() // Purple color for webhook nodes
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "statusCode": {
                    "type": "number",
                    "description": "HTTP status code (200, 201, 400, etc.)",
                    "default": 200,
                    "minimum": 100,
                    "maximum": 599
                },
                "body": {
                    "type": "object",
                    "description": "Response body as JSON",
                    "default": {"message": "Success"}
                },
                "headers": {
                    "type": "object",
                    "description": "Custom HTTP headers",
                    "default": {}
                }
            }
        })
    }

    async fn execute(&self, parameters: Value) -> Result<Value, String> {
        let status_code = parameters["statusCode"].as_u64().unwrap_or(200) as u16;
        let body = parameters["body"].clone();
        let headers = parameters["headers"]
            .as_object()
            .map(|obj| {
                obj.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                    .collect::<std::collections::HashMap<String, String>>()
            })
            .unwrap_or_default();

        // Note: The actual response sending is handled by the executor
        // This node just prepares the response data
        log::info!(
            "📤 RespondToWebhookNode: Preparing response with status {}",
            status_code
        );

        Ok(json!({
            "status_code": status_code,
            "body": body,
            "headers": headers,
            "response_sent": true,
            "timestamp": Utc::now().to_rfc3339()
        }))
    }
}

// ============================================================================
// VARIABLE NODES
// ============================================================================

/// GetVariableNode - retrieves a variable's value
/// Variables can store ANY value: strings, numbers, objects, arrays
pub struct GetVariableNode;

#[async_trait]
impl FlowNode for GetVariableNode {
    fn id(&self) -> String {
        "get-variable".to_string()
    }

    fn name(&self) -> String {
        "Get Variable".to_string()
    }

    fn description(&self) -> String {
        "Retrieves a variable's current value. Variables can store any data type.".to_string()
    }

    fn category(&self) -> String {
        "VARIABLES".to_string()
    }

    fn type_color(&self) -> String {
        "#f59e0b".to_string() // Amber/yellow for variables
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "variableName": {
                    "type": "string",
                    "description": "Variable"
                },
                "defaultValue": {
                    "type": "any",
                    "description": "Default (if not found)",
                    "optional": true
                }
            },
            "required": ["variableName"]
        })
    }

    fn outputs_schema(&self) -> Vec<crate::services::flow::traits::NodeOutput> {
        vec![
            crate::services::flow::traits::NodeOutput {
                id: "value".to_string(),
                name: "Value".to_string(),
                output_type: "any".to_string(),
                description: Some("The variable's current value".to_string()),
                is_dynamic_type: Some(true), // Type adapts based on the variable's value
            },
            crate::services::flow::traits::NodeOutput {
                id: "exists".to_string(),
                name: "Exists".to_string(),
                output_type: "boolean".to_string(),
                description: Some("Whether the variable exists".to_string()),
                is_dynamic_type: None,
            },
        ]
    }

    async fn execute(&self, _params: Value) -> Result<Value, String> {
        // Not used - executor calls execute_with_context which overrides this
        unreachable!("GetVariable uses execute_with_context")
    }

    async fn execute_with_context(
        &self,
        params: Value,
        ctx: &mut ExecutionContext<'_>,
    ) -> Result<Value, String> {
        let variable_name = params["variableName"]
            .as_str()
            .ok_or("Missing variableName parameter")?;
        let default_value = params.get("defaultValue").cloned();

        // Actually look up the variable from runtime context
        let (value, exists) = match ctx.variables.get(variable_name) {
            Some(v) => (v.clone(), true),
            None => (default_value.unwrap_or(Value::Null), false),
        };

        Ok(json!({
            "value": value,
            "exists": exists
        }))
    }
}

/// SetVariableNode - stores a value in a variable
/// Variables can store ANY value: strings, numbers, objects, arrays
pub struct SetVariableNode;

impl SetVariableNode {
    fn coerce_value_for_type(value: Value, value_type: &str) -> Result<Value, String> {
        match value_type {
            "string" => {
                let text = match value {
                    Value::String(s) => s,
                    Value::Null => String::new(),
                    other => other.to_string(),
                };
                Ok(Value::String(text))
            }
            "number" => {
                let number = if let Some(n) = value.as_f64() {
                    n
                } else if let Some(s) = value.as_str() {
                    s.trim()
                        .parse::<f64>()
                        .map_err(|_| format!("Cannot convert '{}' to number", s))?
                } else if let Some(b) = value.as_bool() {
                    if b {
                        1.0
                    } else {
                        0.0
                    }
                } else {
                    return Err("Cannot convert value to number".to_string());
                };

                serde_json::Number::from_f64(number)
                    .map(Value::Number)
                    .ok_or_else(|| "Invalid numeric value".to_string())
            }
            "boolean" => {
                let bool_value = if let Some(b) = value.as_bool() {
                    b
                } else if let Some(n) = value.as_f64() {
                    n != 0.0
                } else if let Some(s) = value.as_str() {
                    matches!(
                        s.trim().to_lowercase().as_str(),
                        "true" | "1" | "yes" | "on"
                    )
                } else {
                    return Err("Cannot convert value to boolean".to_string());
                };
                Ok(Value::Bool(bool_value))
            }
            "object" => {
                if value.is_object() {
                    return Ok(value);
                }
                if let Some(s) = value.as_str() {
                    let parsed: Value = serde_json::from_str(s)
                        .map_err(|_| "Object type expects valid JSON object".to_string())?;
                    if parsed.is_object() {
                        return Ok(parsed);
                    }
                    return Err("Object type expects a JSON object".to_string());
                }
                if value.is_null() {
                    return Ok(json!({}));
                }
                Err("Cannot convert value to object".to_string())
            }
            "array" => {
                if value.is_array() {
                    return Ok(value);
                }
                if let Some(s) = value.as_str() {
                    let parsed: Value = serde_json::from_str(s)
                        .map_err(|_| "Array type expects valid JSON array".to_string())?;
                    if parsed.is_array() {
                        return Ok(parsed);
                    }
                    return Err("Array type expects a JSON array".to_string());
                }
                if value.is_null() {
                    return Ok(json!([]));
                }
                Err("Cannot convert value to array".to_string())
            }
            "any" | "" => Ok(value),
            other => Err(format!("Unsupported valueType '{}'", other)),
        }
    }
}

#[async_trait]
impl FlowNode for SetVariableNode {
    fn id(&self) -> String {
        "set-variable".to_string()
    }

    fn name(&self) -> String {
        "Set Variable".to_string()
    }

    fn description(&self) -> String {
        "Stores a value in a variable. Connect any output to the 'value' input.".to_string()
    }

    fn category(&self) -> String {
        "VARIABLES".to_string()
    }

    fn type_color(&self) -> String {
        "#f59e0b".to_string() // Amber/yellow for variables
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "variableName": {
                    "type": "string",
                    "description": "Variable Name"
                },
                "value": {
                    "type": "any",
                    "description": "Value",
                    "optional": true
                },
                "valueType": {
                    "type": "string",
                    "enum": ["any", "string", "number", "boolean", "object", "array"],
                    "default": "any",
                    "description": "Type coercion for manual values"
                },
                "scope": {
                    "type": "string",
                    "enum": ["local"],
                    "default": "local",
                    "description": "Scope"
                    // TODO: Add "session" and "global" when implemented
                }
            },
            "required": ["variableName"]
        })
    }

    fn outputs_schema(&self) -> Vec<crate::services::flow::traits::NodeOutput> {
        vec![
            crate::services::flow::traits::NodeOutput {
                id: "value".to_string(),
                name: "Value".to_string(),
                output_type: "any".to_string(),
                description: Some("Passthrough of the stored value (for chaining)".to_string()),
                is_dynamic_type: Some(true), // Type adapts based on the input value
            },
            crate::services::flow::traits::NodeOutput {
                id: "previousValue".to_string(),
                name: "Previous Value".to_string(),
                output_type: "any".to_string(),
                description: Some("The variable's previous value (if it existed)".to_string()),
                is_dynamic_type: Some(true), // Type adapts based on previous value
            },
        ]
    }

    async fn execute(&self, _params: Value) -> Result<Value, String> {
        // Not used - executor calls execute_with_context which overrides this
        unreachable!("SetVariable uses execute_with_context")
    }

    async fn execute_with_context(
        &self,
        params: Value,
        ctx: &mut ExecutionContext<'_>,
    ) -> Result<Value, String> {
        let variable_name = params["variableName"]
            .as_str()
            .ok_or("Missing variableName parameter")?;
        let raw_value = params.get("value").cloned().unwrap_or(Value::Null);
        let value_type = params
            .get("valueType")
            .and_then(|v| v.as_str())
            .unwrap_or("any");
        let value = Self::coerce_value_for_type(raw_value, value_type)?;
        let scope_str = params["scope"].as_str().unwrap_or("local");

        // Parse scope (for now only local is implemented)
        let scope = match scope_str {
            "session" => VariableScope::Session,
            "global" => VariableScope::Global,
            _ => VariableScope::Local, // default
        };

        // Get previous value before setting new one
        if ctx.variables.is_read_only(variable_name) {
            return Err(
                "Invocation inputs are read-only; store derived values in another variable".into(),
            );
        }
        let previous_value = ctx.variables.get(variable_name).cloned();

        // Actually store the variable in runtime context
        ctx.variables.set(variable_name, value.clone(), scope);

        Ok(json!({
            "value": value,
            "previousValue": previous_value,
            "success": true
        }))
    }
}

/// Display Text Node - Shows text content in the UI
pub struct DisplayTextNode;

#[async_trait]
impl FlowNode for DisplayTextNode {
    fn id(&self) -> String {
        "display-text".to_owned()
    }

    fn name(&self) -> String {
        "Display Text".to_owned()
    }

    fn description(&self) -> String {
        "Display text content in the UI with formatting options".to_owned()
    }

    fn category(&self) -> String {
        "UI".to_owned()
    }

    fn type_color(&self) -> String {
        "#10b981".to_owned() // Green
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "Text to display",
                    "default": ""
                },
                "fontSize": {
                    "type": "number",
                    "description": "Font size in pixels",
                    "default": 14,
                    "minimum": 8,
                    "maximum": 72
                },
                "color": {
                    "type": "string",
                    "format": "color",
                    "description": "Text color",
                    "default": "#ffffff"
                },
                "backgroundColor": {
                    "type": "string",
                    "format": "color",
                    "description": "Background color",
                    "default": "#1f2937"
                }
            },
            "required": ["text"]
        })
    }

    // No outputs - this is a UI display node only
    fn outputs_schema(&self) -> Vec<crate::services::flow::traits::NodeOutput> {
        vec![]
    }

    async fn execute(&self, params: Value) -> Result<Value, String> {
        let text = params["text"].as_str().unwrap_or("");
        let font_size = params["fontSize"].as_f64().unwrap_or(14.0);
        let color = params["color"].as_str().unwrap_or("#ffffff");
        let bg_color = params["backgroundColor"].as_str().unwrap_or("#1f2937");

        // UI nodes just return display data, no outputs for connection
        Ok(json!({
            "display": {
                "text": text,
                "fontSize": font_size,
                "color": color,
                "backgroundColor": bg_color
            },
            "success": true
        }))
    }
}

// ============================================================================
// FILE OPERATION NODES
// ============================================================================

/// Save File Node - Saves content to local filesystem
pub struct SaveFileNode;

#[async_trait]
impl FlowNode for SaveFileNode {
    fn id(&self) -> String {
        "save-file".to_string()
    }

    fn name(&self) -> String {
        "Save File".to_string()
    }

    fn description(&self) -> String {
        "Save content to a local file".to_string()
    }

    fn category(&self) -> String {
        "SYSTEM".to_string()
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "content": {
                    "type": "string",
                    "description": "Content to save (text, JSON, base64, etc.)"
                },
                "filePath": {
                    "type": "string",
                    "format": "file",
                    "description": "Destination file path",
                    "typeOptions": {
                        "mode": "save",
                        "acceptedTypes": ["*"]
                    }
                },
                "overwrite": {
                    "type": "boolean",
                    "description": "Overwrite if file exists",
                    "default": false
                },
                "createDirectories": {
                    "type": "boolean",
                    "description": "Create parent directories if they don't exist",
                    "default": true
                }
            },
            "required": ["content", "filePath"]
        })
    }

    fn outputs_schema(&self) -> Vec<crate::services::flow::traits::NodeOutput> {
        use crate::services::flow::traits::NodeOutput;
        vec![
            NodeOutput {
                id: "filePath".to_owned(),
                name: "File Path".to_owned(),
                output_type: "string".to_owned(),
                description: Some("Absolute path to saved file".to_owned()),
                ..Default::default()
            },
            NodeOutput {
                id: "fileSize".to_owned(),
                name: "File Size".to_owned(),
                output_type: "number".to_owned(),
                description: Some("Size of saved file in bytes".to_owned()),
                ..Default::default()
            },
            NodeOutput {
                id: "success".to_owned(),
                name: "Success".to_owned(),
                output_type: "boolean".to_owned(),
                description: Some("Whether the file was saved successfully".to_owned()),
                ..Default::default()
            },
        ]
    }

    fn type_color(&self) -> String {
        "#2196F3".to_string() // Blue
    }

    async fn execute(&self, params: Value) -> Result<Value, String> {
        use std::fs;
        use std::path::Path;

        let content = params
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or("Content is required")?;

        let file_path = params
            .get("filePath")
            .and_then(|v| v.as_str())
            .ok_or("File path is required")?;

        let overwrite = params
            .get("overwrite")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let create_directories = params
            .get("createDirectories")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let path = Path::new(file_path);

        // 🔒 SECURITY: Validate path is within workspace
        // TODO: Get workspace config from context and validate path
        // For now, we allow all paths but this needs workspace validation

        // Check if file exists and overwrite is false
        if path.exists() && !overwrite {
            return Err(format!("File already exists: {}", file_path));
        }

        // Create parent directories if needed
        if create_directories {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create directories: {}", e))?;
            }
        }

        // Write file
        fs::write(path, content).map_err(|e| format!("Failed to write file: {}", e))?;

        // Get file size
        let metadata =
            fs::metadata(path).map_err(|e| format!("Failed to get file metadata: {}", e))?;
        let file_size = metadata.len();

        // Get absolute path
        let absolute_path = path
            .canonicalize()
            .map_err(|e| format!("Failed to get absolute path: {}", e))?;

        Ok(json!({
            "filePath": absolute_path.to_string_lossy(),
            "fileSize": file_size,
            "success": true
        }))
    }
}

/// Load File Node - Loads content from local filesystem
pub struct LoadFileNode;

#[async_trait]
impl FlowNode for LoadFileNode {
    fn id(&self) -> String {
        "load-file".to_string()
    }

    fn name(&self) -> String {
        "Load File".to_string()
    }

    fn description(&self) -> String {
        "Load content from a local file or multiple files".to_string()
    }

    fn category(&self) -> String {
        "SYSTEM".to_string()
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "filePath": {
                    "type": "string",
                    "format": "file",
                    "description": "File path or glob pattern (e.g., *.txt)",
                    "typeOptions": {
                        "mode": "open",
                        "acceptedTypes": ["*"]
                    }
                },
                "encoding": {
                    "type": "string",
                    "enum": ["utf8", "base64", "binary"],
                    "description": "File encoding",
                    "default": "utf8"
                },
                "batch": {
                    "type": "boolean",
                    "description": "Load multiple files (supports glob patterns)",
                    "default": false
                }
            },
            "required": ["filePath"]
        })
    }

    fn outputs_schema(&self) -> Vec<crate::services::flow::traits::NodeOutput> {
        use crate::services::flow::traits::NodeOutput;
        vec![
            NodeOutput {
                id: "content".to_owned(),
                name: "Content".to_owned(),
                output_type: "string".to_owned(),
                description: Some("File content (string or array if batch mode)".to_owned()),
                is_dynamic_type: Some(true),
            },
            NodeOutput {
                id: "fileName".to_owned(),
                name: "File Name".to_owned(),
                output_type: "string".to_owned(),
                description: Some("Name of the loaded file".to_owned()),
                ..Default::default()
            },
            NodeOutput {
                id: "fileSize".to_owned(),
                name: "File Size".to_owned(),
                output_type: "number".to_owned(),
                description: Some("Size in bytes".to_owned()),
                ..Default::default()
            },
            NodeOutput {
                id: "fileCount".to_owned(),
                name: "File Count".to_owned(),
                output_type: "number".to_owned(),
                description: Some("Number of files loaded (batch mode)".to_owned()),
                ..Default::default()
            },
        ]
    }

    fn type_color(&self) -> String {
        "#2196F3".to_string() // Blue
    }

    async fn execute(&self, params: Value) -> Result<Value, String> {
        use glob::glob;
        use std::fs;
        use std::path::Path;

        let file_path = params
            .get("filePath")
            .and_then(|v| v.as_str())
            .ok_or("File path is required")?;

        let encoding = params
            .get("encoding")
            .and_then(|v| v.as_str())
            .unwrap_or("utf8");

        let batch = params
            .get("batch")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if batch {
            // Batch mode: use glob pattern
            let mut files = Vec::new();
            let mut total_size = 0u64;

            for entry in glob(file_path).map_err(|e| format!("Invalid glob pattern: {}", e))? {
                match entry {
                    Ok(path) => {
                        if path.is_file() {
                            let content = match encoding {
                                "base64" => {
                                    let bytes = fs::read(&path)
                                        .map_err(|e| format!("Failed to read file: {}", e))?;
                                    use base64::{engine::general_purpose, Engine as _};
                                    general_purpose::STANDARD.encode(&bytes)
                                }
                                "binary" => {
                                    let bytes = fs::read(&path)
                                        .map_err(|e| format!("Failed to read file: {}", e))?;
                                    String::from_utf8_lossy(&bytes).to_string()
                                }
                                _ => {
                                    // utf8
                                    fs::read_to_string(&path)
                                        .map_err(|e| format!("Failed to read file: {}", e))?
                                }
                            };

                            let metadata = fs::metadata(&path)
                                .map_err(|e| format!("Failed to get metadata: {}", e))?;
                            total_size += metadata.len();

                            files.push(json!({
                                "fileName": path.file_name().unwrap().to_string_lossy(),
                                "filePath": path.to_string_lossy(),
                                "content": content,
                                "fileSize": metadata.len()
                            }));
                        }
                    }
                    Err(e) => println!("Error reading file: {:?}", e),
                }
            }

            if files.is_empty() {
                return Err(format!("No files found matching pattern: {}", file_path));
            }

            Ok(json!({
                "content": files,
                "fileName": format!("{} files", files.len()),
                "fileSize": total_size,
                "fileCount": files.len()
            }))
        } else {
            // Single file mode
            let path = Path::new(file_path);

            if !path.exists() {
                return Err(format!("File not found: {}", file_path));
            }

            let content = match encoding {
                "base64" => {
                    let bytes =
                        fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;
                    use base64::{engine::general_purpose, Engine as _};
                    general_purpose::STANDARD.encode(&bytes)
                }
                "binary" => {
                    let bytes =
                        fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;
                    String::from_utf8_lossy(&bytes).to_string()
                }
                _ => {
                    // utf8
                    fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?
                }
            };

            let metadata =
                fs::metadata(path).map_err(|e| format!("Failed to get metadata: {}", e))?;

            Ok(json!({
                "content": content,
                "fileName": path.file_name().unwrap().to_string_lossy(),
                "fileSize": metadata.len(),
                "fileCount": 1
            }))
        }
    }
}

/// Download File Node - Downloads file from HTTP/HTTPS URL
pub struct DownloadFileNode;

#[async_trait]
impl FlowNode for DownloadFileNode {
    fn id(&self) -> String {
        "download-file".to_string()
    }

    fn name(&self) -> String {
        "Download File".to_string()
    }

    fn description(&self) -> String {
        "Download a file from HTTP/HTTPS URL".to_string()
    }

    fn category(&self) -> String {
        "SYSTEM".to_string()
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "format": "uri",
                    "description": "HTTP/HTTPS URL to download from"
                },
                "saveDirectory": {
                    "type": "string",
                    "format": "file",
                    "description": "Directory to save the file (filename auto-extracted from URL)",
                    "typeOptions": {
                        "mode": "directory"
                    }
                },
                "timeout": {
                    "type": "number",
                    "description": "Download timeout in seconds (0 = no timeout)",
                    "default": 0,
                    "minimum": 0,
                    "maximum": 3599999,
                    "ui:widget": "duration"
                }
            },
            "required": ["url", "saveDirectory"]
        })
    }

    fn outputs_schema(&self) -> Vec<crate::services::flow::traits::NodeOutput> {
        use crate::services::flow::traits::NodeOutput;
        vec![
            NodeOutput {
                id: "filePath".to_owned(),
                name: "File Path".to_owned(),
                output_type: "string".to_owned(),
                description: Some("Path to downloaded file".to_owned()),
                ..Default::default()
            },
            NodeOutput {
                id: "fileSize".to_owned(),
                name: "File Size".to_owned(),
                output_type: "number".to_owned(),
                description: Some("Size of downloaded file in bytes".to_owned()),
                ..Default::default()
            },
            NodeOutput {
                id: "httpStatus".to_owned(),
                name: "HTTP Status".to_owned(),
                output_type: "number".to_owned(),
                description: Some("HTTP status code (200, 404, etc.)".to_owned()),
                ..Default::default()
            },
            NodeOutput {
                id: "success".to_owned(),
                name: "Success".to_owned(),
                output_type: "boolean".to_owned(),
                description: Some("Whether download succeeded".to_owned()),
                ..Default::default()
            },
        ]
    }

    fn type_color(&self) -> String {
        "#2196F3".to_string() // Blue
    }

    async fn execute(&self, params: Value) -> Result<Value, String> {
        // Create dummy context for backward compatibility
        let mut runtime_vars = crate::services::flow::executor::RuntimeVariables::new();
        let mut ctx = crate::services::flow::traits::ExecutionContext {
            flow_id: "",
            node_id: "",
            variables: &mut runtime_vars,
            progress_callback: None,
        };
        self.execute_with_context(params, &mut ctx).await
    }

    async fn execute_with_context(
        &self,
        params: Value,
        ctx: &mut crate::services::flow::traits::ExecutionContext<'_>,
    ) -> Result<Value, String> {
        use futures_util::StreamExt;
        use std::fs;
        use std::path::PathBuf;
        use tokio::io::AsyncWriteExt;

        let url = params
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or("URL is required")?;

        let save_directory = params
            .get("saveDirectory")
            .and_then(|v| v.as_str())
            .ok_or("Save directory is required")?;

        let timeout = params.get("timeout").and_then(|v| v.as_u64()).unwrap_or(0);

        // Extract filename from URL
        let filename = url
            .split('/')
            .last()
            .and_then(|s| {
                // Remove query params if present
                let without_query = s.split('?').next()?;
                if without_query.is_empty() {
                    None
                } else {
                    Some(without_query)
                }
            })
            .ok_or("Could not extract filename from URL")?;

        // Combine directory + filename
        let save_path = PathBuf::from(save_directory).join(filename);

        // Ensure directory exists
        fs::create_dir_all(save_directory)
            .map_err(|e| format!("Failed to create directory: {}", e))?;

        // Download file with optional timeout
        let client = reqwest::Client::new();
        let mut request = client.get(url);

        // Only set timeout if > 0 (0 = no timeout)
        if timeout > 0 {
            request = request.timeout(std::time::Duration::from_secs(timeout));
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Failed to download file: {}", e))?;

        let status = response.status().as_u16() as u32;

        if !response.status().is_success() {
            return Ok(json!({
                "filePath": save_path.to_string_lossy().to_string(),
                "fileSize": 0,
                "httpStatus": status,
                "success": false,
                "error": format!("HTTP {}: {}", status, response.status().canonical_reason().unwrap_or("Unknown"))
            }));
        }

        // Get content length if available
        let content_length = response.content_length();

        // Download with progress tracking
        let mut stream = response.bytes_stream();
        let mut file = tokio::fs::File::create(&save_path)
            .await
            .map_err(|e| format!("Failed to create file: {}", e))?;

        let mut downloaded: u64 = 0;
        let mut last_progress_report = std::time::Instant::now();
        const PROGRESS_THROTTLE_MS: u64 = 150; // Report progress at most every 150ms

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| format!("Failed to read chunk: {}", e))?;

            file.write_all(&chunk)
                .await
                .map_err(|e| format!("Failed to write chunk: {}", e))?;

            downloaded += chunk.len() as u64;

            // Throttle progress reporting to avoid performance issues
            let now = std::time::Instant::now();
            let should_report = now.duration_since(last_progress_report).as_millis()
                >= PROGRESS_THROTTLE_MS as u128;

            if should_report {
                if let Some(callback) = &ctx.progress_callback {
                    let progress = if let Some(total) = content_length {
                        (downloaded as f64 / total as f64) * 100.0
                    } else {
                        0.0 // Unknown size, can't show percentage
                    };

                    let message = if let Some(total) = content_length {
                        format!(
                            "{:.1} MB / {:.1} MB",
                            downloaded as f64 / 1_048_576.0,
                            total as f64 / 1_048_576.0
                        )
                    } else {
                        format!("{:.1} MB", downloaded as f64 / 1_048_576.0)
                    };

                    callback(progress, message);
                    last_progress_report = now;
                }
            }
        }

        // Report final progress (100%)
        if let Some(callback) = &ctx.progress_callback {
            if let Some(total) = content_length {
                callback(
                    100.0,
                    format!(
                        "{:.1} MB / {:.1} MB",
                        downloaded as f64 / 1_048_576.0,
                        total as f64 / 1_048_576.0
                    ),
                );
            }
        }

        file.flush()
            .await
            .map_err(|e| format!("Failed to flush file: {}", e))?;

        Ok(json!({
            "filePath": save_path.canonicalize().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|_| save_path.to_string_lossy().to_string()),
            "fileSize": downloaded,
            "httpStatus": status,
            "success": true
        }))
    }
}

/// Upload File Node - Uploads file to HTTP/HTTPS endpoint
pub struct UploadFileNode;

#[async_trait]
impl FlowNode for UploadFileNode {
    fn id(&self) -> String {
        "upload-file".to_string()
    }

    fn name(&self) -> String {
        "Upload File".to_string()
    }

    fn description(&self) -> String {
        "Upload a file to HTTP/HTTPS endpoint (multipart form-data)".to_string()
    }

    fn category(&self) -> String {
        "SYSTEM".to_string()
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "format": "uri",
                    "description": "HTTP/HTTPS endpoint URL"
                },
                "filePath": {
                    "type": "string",
                    "format": "file",
                    "description": "Local file path to upload",
                    "typeOptions": {
                        "mode": "open",
                        "acceptedTypes": ["*"]
                    }
                },
                "fieldName": {
                    "type": "string",
                    "description": "Form field name for the file",
                    "default": "file"
                },
                "timeout": {
                    "type": "number",
                    "description": "Upload timeout in seconds (0 = no timeout)",
                    "default": 0,
                    "minimum": 0,
                    "maximum": 3599999,
                    "ui:widget": "duration"
                }
            },
            "required": ["url", "filePath"]
        })
    }

    fn outputs_schema(&self) -> Vec<crate::services::flow::traits::NodeOutput> {
        use crate::services::flow::traits::NodeOutput;
        vec![
            NodeOutput {
                id: "httpStatus".to_owned(),
                name: "HTTP Status".to_owned(),
                output_type: "number".to_owned(),
                description: Some("HTTP response status code".to_owned()),
                ..Default::default()
            },
            NodeOutput {
                id: "responseBody".to_owned(),
                name: "Response Body".to_owned(),
                output_type: "string".to_owned(),
                description: Some("Server response body".to_owned()),
                ..Default::default()
            },
            NodeOutput {
                id: "success".to_owned(),
                name: "Success".to_owned(),
                output_type: "boolean".to_owned(),
                description: Some("Whether upload succeeded".to_owned()),
                ..Default::default()
            },
        ]
    }

    fn type_color(&self) -> String {
        "#2196F3".to_string() // Blue
    }

    async fn execute(&self, params: Value) -> Result<Value, String> {
        // Create dummy context for backward compatibility
        let mut runtime_vars = crate::services::flow::executor::RuntimeVariables::new();
        let mut ctx = crate::services::flow::traits::ExecutionContext {
            flow_id: "",
            node_id: "",
            variables: &mut runtime_vars,
            progress_callback: None,
        };
        self.execute_with_context(params, &mut ctx).await
    }

    async fn execute_with_context(
        &self,
        params: Value,
        ctx: &mut crate::services::flow::traits::ExecutionContext<'_>,
    ) -> Result<Value, String> {
        use std::fs;
        use std::path::Path;

        let url = params
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or("URL is required")?
            .to_string();

        let file_path = params
            .get("filePath")
            .and_then(|v| v.as_str())
            .ok_or("File path is required")?
            .to_string();

        let field_name = params
            .get("fieldName")
            .and_then(|v| v.as_str())
            .unwrap_or("file")
            .to_string();

        let timeout = params.get("timeout").and_then(|v| v.as_u64()).unwrap_or(0);

        // Check if file exists
        let path = Path::new(&file_path);
        if !path.exists() {
            return Err(format!("File not found: {}", file_path));
        }

        // Get file size for progress reporting
        let file_size = fs::metadata(path)
            .map_err(|e| format!("Failed to get file size: {}", e))?
            .len();

        // Report start
        if let Some(callback) = &ctx.progress_callback {
            callback(
                0.0,
                format!(
                    "Preparing upload ({:.1} MB)",
                    file_size as f64 / 1_048_576.0
                ),
            );
        }

        // Read file
        let file_bytes = fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_string();

        // Report upload starting
        if let Some(callback) = &ctx.progress_callback {
            callback(
                25.0,
                format!("Uploading {:.1} MB...", file_size as f64 / 1_048_576.0),
            );
        }

        // Upload file with optional timeout
        let client = reqwest::Client::new();

        let part = reqwest::multipart::Part::bytes(file_bytes).file_name(file_name);

        let form = reqwest::multipart::Form::new().part(field_name, part);

        let mut request = client.post(&url).multipart(form);

        // Only set timeout if > 0 (0 = no timeout)
        if timeout > 0 {
            request = request.timeout(std::time::Duration::from_secs(timeout));
        }

        // Report sending
        if let Some(callback) = &ctx.progress_callback {
            callback(50.0, "Sending data...".to_string());
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Failed to upload file: {}", e))?;

        // Report receiving response
        if let Some(callback) = &ctx.progress_callback {
            callback(75.0, "Receiving response...".to_string());
        }

        let status = response.status().as_u16() as u32;
        let response_body = response.text().await.unwrap_or_else(|_| String::new());

        // Report complete
        if let Some(callback) = &ctx.progress_callback {
            callback(100.0, "Upload complete".to_string());
        }

        Ok(json!({
            "httpStatus": status,
            "responseBody": response_body,
            "success": (200..300).contains(&status)
        }))
    }
}

/// Preview File Node - Generalized file preview (UI-only)
/// Auto-detects file type and displays appropriately
pub struct PreviewFileNode;

#[async_trait]
impl FlowNode for PreviewFileNode {
    fn id(&self) -> String {
        "preview-file".to_string()
    }

    fn name(&self) -> String {
        "Preview File".to_string()
    }

    fn description(&self) -> String {
        "Display file preview (auto-detects type: image, text, JSON, PDF, etc.)".to_string()
    }

    fn category(&self) -> String {
        "SYSTEM".to_string()
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "fileData": {
                    "type": "string",
                    "description": "File path or base64-encoded content"
                },
                "fileName": {
                    "type": "string",
                    "description": "File name (for type detection)"
                }
            },
            "required": ["fileData"]
        })
    }

    fn outputs_schema(&self) -> Vec<crate::services::flow::traits::NodeOutput> {
        use crate::services::flow::traits::NodeOutput;
        vec![NodeOutput {
            id: "result".to_owned(),
            name: "Result".to_owned(),
            output_type: "object".to_owned(),
            description: Some("Preview result with type and data".to_owned()),
            is_dynamic_type: Some(true),
        }]
    }

    fn type_color(&self) -> String {
        "#2196F3".to_string() // Blue
    }

    async fn execute(&self, params: Value) -> Result<Value, String> {
        let file_path = params
            .get("fileData")
            .and_then(|v| v.as_str())
            .ok_or("File path is required")?;

        let mut file_name = params
            .get("fileName")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Check if fileData is a file path
        let path = std::path::Path::new(file_path);

        if path.exists() && path.is_file() {
            // Extract filename if not provided
            if file_name.is_empty() {
                file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();
            }

            // Detect file type by reading only first 8KB (works even for multi-GB files!)
            use std::io::Read;
            let mut file =
                std::fs::File::open(&path).map_err(|e| format!("Failed to open file: {}", e))?;

            let mut buf = [0u8; 8192]; // 8KB buffer
            let bytes_read = file
                .read(&mut buf)
                .map_err(|e| format!("Failed to read file: {}", e))?;

            // Detect MIME type from magic bytes
            let file_type = infer::get(&buf[..bytes_read])
                .map(|k| k.mime_type().to_string())
                .unwrap_or_else(|| "application/octet-stream".to_string());

            // Normalize the file path (remove Windows extended-length prefix \\?\ if present)
            let normalized_path = file_path
                .strip_prefix(r"\\?\")
                .unwrap_or(file_path)
                .to_string();

            Ok(json!({
                "filePath": normalized_path,
                "fileName": file_name,
                "fileType": file_type,
                "success": true
            }))
        } else {
            Err(format!("File not found: {}", file_path))
        }
    }
}

/// Preview Image Node - Displays image from base64 or URL
pub struct PreviewImageNode;

#[async_trait]
impl FlowNode for PreviewImageNode {
    fn id(&self) -> String {
        "preview-image".to_owned()
    }

    fn name(&self) -> String {
        "Preview Image".to_owned()
    }

    fn description(&self) -> String {
        "Displays image data. Resize the node to adjust preview size.".to_owned()
    }

    fn category(&self) -> String {
        "MEDIA".to_owned()
    }

    fn type_color(&self) -> String {
        "#8B5CF6".to_owned() // Violet (matches image type)
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "imageData": {
                    "type": "string",
                    "description": "Image data (base64 with data URI or URL)",
                    "default": "",
                    "forceInput": true,  // Connection only - no manual input
                    "optional": false
                }
            }
        })
    }

    // No outputs - this is a UI display node only
    fn outputs_schema(&self) -> Vec<crate::services::flow::traits::NodeOutput> {
        vec![]
    }

    async fn execute(&self, params: Value) -> Result<Value, String> {
        let image_data = params
            .get("imageData")
            .and_then(|v| v.as_str())
            .ok_or("Image data is required")?;

        if image_data.is_empty() {
            return Err("Image data cannot be empty".to_owned());
        }

        // UI nodes just return display data, no outputs for connection
        Ok(json!({
            "display": {
                "imageData": image_data
            },
            "success": true
        }))
    }
}

impl FlowNodeProvider for StandardNodeProvider {
    fn get_flow_nodes(&self) -> Vec<Box<dyn FlowNode>> {
        vec![
            Box::new(AudioPlayNode),
            Box::new(AudioStopNode),
            Box::new(DelayNode),
            Box::new(ApprovalNode),
            Box::new(BranchNode),
            Box::new(WebhookTriggerNode),
            Box::new(ButtonTriggerNode),
            Box::new(AiTextGeneratorNode),
            Box::new(AiFreeAgentNode),
            Box::new(AiOrchestratorNode),
            Box::new(RespondToWebhookNode),
            // Variable nodes
            Box::new(GetVariableNode),
            Box::new(SetVariableNode),
            // Development-only schema fixture; never publish it in release catalogs.
            #[cfg(debug_assertions)]
            Box::new(WidgetDemoNode),
            // File operation nodes
            Box::new(SaveFileNode),
            Box::new(LoadFileNode),
            Box::new(DownloadFileNode),
            Box::new(UploadFileNode),
            // Display nodes
            Box::new(DisplayTextNode),
            Box::new(PreviewFileNode),
            Box::new(PreviewImageNode),
        ]
    }

    fn provider_name(&self) -> &'static str {
        "StandardNodeProvider"
    }
}

#[cfg(test)]
mod catalog_tests {
    use super::*;

    #[test]
    fn catalog_does_not_publish_mock_error_wrapper_nodes() {
        let nodes = StandardNodeProvider.get_flow_nodes();
        let ids: Vec<String> = nodes.iter().map(|node| node.id()).collect();

        for retired_id in ["try-catch", "retry", "fallback", "timeout"] {
            assert!(
                !ids.iter().any(|id| id == retired_id),
                "{retired_id} must be a scheduler policy or graph route, not a mock node"
            );
        }
    }

    #[test]
    fn debug_schema_fixture_is_separated_from_user_control_nodes() {
        let nodes = StandardNodeProvider.get_flow_nodes();
        if let Some(widget_demo) = nodes.iter().find(|node| node.id() == "widget-demo") {
            assert_eq!(widget_demo.category(), "DEVELOPMENT");
            assert_eq!(
                widget_demo.subcategory().as_deref(),
                Some("Schema Fixtures")
            );
        }
    }
}
