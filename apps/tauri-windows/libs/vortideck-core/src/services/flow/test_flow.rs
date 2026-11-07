use serde_json::{json, Value};
use std::sync::Arc;
// Tauri dependencies removed - test moved to adapter layer
// use tauri::{AppHandle, Manager};
use crate::services::flow::commands::FlowRegistryState;

use crate::core::PlatformContext;
use crate::services::flow::{
    registry::FlowNodeRegistry,
    standard_nodes::StandardNodeProvider,
    system_adapter::SystemProviderAdapter,
    traits::{FlowNode, FlowNodeProvider},
};
use crate::services::system::flow_adapter::SystemActionProvider;

/// Simple function to test the flow functionality
#[cfg(feature = "tauri")]
pub async fn test_flow_execution(app_handle: AppHandle) -> Result<(), String> {
    println!("Starting flow test");

    // Get registry from app state
    let registry = match app_handle.try_state::<FlowRegistryState>() {
        Some(registry) => registry,
        None => {
            return Err("FlowRegistryState not available".to_string());
        }
    };

    // Register the standard nodes provider if not already registered

    // List all registered nodes
    let all_nodes = registry.get_all_node_info();
    println!("Registered nodes:");
    for node in &all_nodes {
        println!(
            "  - {} ({}): {}",
            node.name, node.type_name, node.description
        );
    }

    // Specifically check for system.volume.set
    println!("\nChecking if system.volume.set node exists:");

    // Get all node IDs for debugging
    let node_ids = registry.registry.lock().unwrap().get_all_node_ids();
    println!("All registered node IDs:");
    for id in &node_ids {
        println!("  - {}", id);
    }

    // Try to find the system volume node by ID
    if let Some(volume_node) = registry.get_node("system.volume.set") {
        println!("✅ Found system.volume.set node");
    } else {
        println!("❌ system.volume.set node not found!");

        // Try to find any system nodes
        let system_nodes: Vec<_> = node_ids
            .iter()
            .filter(|id| id.starts_with("system."))
            .collect();

        if !system_nodes.is_empty() {
            println!("Found other system nodes:");
            for id in system_nodes {
                println!("  - {}", id);
            }
        } else {
            println!("No system nodes found at all!");
        }
    }

    // Create a simple flow:
    // 1. Delay node (wait 500ms)
    // 2. Audio play node
    println!("\nExecuting simple flow:");

    // Step 1: Execute the delay node
    let delay_node_id = "standard.control.delay";
    let delay_params = json!({ "duration": 500 });

    println!("- Executing delay node with duration 500ms");
    let delay_node = registry
        .get_node(delay_node_id)
        .ok_or_else(|| format!("Node with ID '{}' not found", delay_node_id))?;

    let delay_result = delay_node.execute(delay_params).await?;
    println!("  Result: {:?}", delay_result);

    // Step 2: Execute the audio play node
    let audio_node_id = "standard.audio.play";
    let audio_params = json!({
        "fileLocation": "C:/path/to/sound.wav",
        "id": "test-audio",
        "volume": 10
    });

    println!("- Executing audio play node");
    let audio_node = registry
        .get_node(audio_node_id)
        .ok_or_else(|| format!("Node with ID '{}' not found", audio_node_id))?;

    let audio_result = audio_node.execute(audio_params).await?;
    println!("  Result: {:?}", audio_result);

    // Try a system action if available
    if let Some(volume_node) = registry.get_node("system.volume.set") {
        println!("- Executing system volume action");
        let volume_params = json!({ "volume": 50 });
        let volume_result = volume_node.execute(volume_params).await?;
        println!("  Result: {:?}", volume_result);
    } else {
        println!("System volume action not found (this is OK if not implemented)");
    }

    println!("\nFlow test completed successfully!");
    Ok(())
}

// Simple function to test a flow with branching
#[cfg(feature = "tauri")]
pub async fn test_flow_branching(app_handle: AppHandle) -> Result<(), String> {
    println!("Starting branching flow test");

    // Initialize the registry with providers
    let registry = match app_handle.try_state::<FlowRegistryState>() {
        Some(registry) => registry,
        None => {
            return Err("FlowRegistryState not available".to_string());
        }
    };

    // Create a flow with branching:
    // 1. Branch node (condition = true)
    // 2a. If true: Play audio
    // 2b. If false: Delay

    println!("\nExecuting branching flow:");

    // Step 1: Execute the branch node
    let branch_node_id = "standard.control.branch";
    let branch_node = registry
        .get_node(branch_node_id)
        .ok_or_else(|| format!("Node with ID '{}' not found", branch_node_id))?;

    // Test with condition = true
    println!("\n- Testing branch with condition = true");
    let branch_params_true = json!({ "condition": true });
    let branch_result_true = branch_node.execute(branch_params_true).await?;
    println!("  Result: {:?}", branch_result_true);

    let branch_value = branch_result_true["branch"].as_bool().unwrap_or(false);

    // Add a small delay before playing audio
    println!("Waiting a moment before next audio test...");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Execute the appropriate node based on the branch result
    if branch_value {
        // True path: Play audio
        let audio_node_id = "standard.audio.play";
        let audio_node = registry
            .get_node(audio_node_id)
            .ok_or_else(|| format!("Node with ID '{}' not found", audio_node_id))?;

        let audio_params = json!({
            "fileLocation": "C:/path/to/sound.wav",
            "id": "true-path-audio",
            "volume": 10
        });

        println!("- Following TRUE path: Play audio");
        let audio_result = audio_node.execute(audio_params).await?;
        println!("  Result: {:?}", audio_result);
    } else {
        // False path: Delay
        let delay_node_id = "standard.control.delay";
        let delay_node = registry
            .get_node(delay_node_id)
            .ok_or_else(|| format!("Node with ID '{}' not found", delay_node_id))?;

        let delay_params = json!({ "duration": 1000 });

        println!("- Following FALSE path: Delay");
        let delay_result = delay_node.execute(delay_params).await?;
        println!("  Result: {:?}", delay_result);
    }

    // Test with condition = false
    println!("\n- Testing branch with condition = false");
    let branch_params_false = json!({ "condition": false });
    let branch_result_false = branch_node.execute(branch_params_false).await?;
    println!("  Result: {:?}", branch_result_false);

    let branch_value = branch_result_false["branch"].as_bool().unwrap_or(false);

    // Add a small delay before the next test
    println!("Waiting a moment before next test...");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Execute the appropriate node based on the branch result
    if branch_value {
        // True path: Play audio
        let audio_node_id = "standard.audio.play";
        let audio_node = registry
            .get_node(audio_node_id)
            .ok_or_else(|| format!("Node with ID '{}' not found", audio_node_id))?;

        let audio_params = json!({
            "fileLocation": "C:/path/to/sound.wav",
            "id": "true-path-audio",
            "volume": 10
        });

        println!("- Following TRUE path: Play audio");
        let audio_result = audio_node.execute(audio_params).await?;
        println!("  Result: {:?}", audio_result);
    } else {
        // False path: Delay
        let delay_node_id = "standard.control.delay";
        let delay_node = registry
            .get_node(delay_node_id)
            .ok_or_else(|| format!("Node with ID '{}' not found", delay_node_id))?;

        let delay_params = json!({ "duration": 1000 });

        println!("- Following FALSE path: Delay");
        let delay_result = delay_node.execute(delay_params).await?;
        println!("  Result: {:?}", delay_result);
    }

    println!("\nBranching flow test completed successfully!");
    Ok(())
}

/// Universal test flow function that doesn't require Tauri
pub async fn run_test_flow_universal(platform_context: &PlatformContext) -> Result<String, String> {
    // Test events are now handled via EventBus (optional for this test)
    // In production, events would be emitted via platform_context.event_bus.emit(...)

    // Create a simple test flow execution
    println!("Running universal test flow...");

    // Test system interface
    let system_info = platform_context
        .system_interface
        .get_system_info()
        .await
        .map_err(|e| format!("Failed to get system info: {}", e))?;

    println!(
        "System info: platform={}, arch={}",
        system_info.platform, system_info.arch
    );

    // Test storage provider
    let test_key = "test_flow_execution";
    let test_value = serde_json::json!({"timestamp": chrono::Utc::now().timestamp()});

    platform_context
        .storage_provider
        .set(test_key, &test_value)
        .await
        .map_err(|e| format!("Failed to store test data: {}", e))?;

    let retrieved_value = platform_context
        .storage_provider
        .get(test_key)
        .await
        .map_err(|e| format!("Failed to retrieve test data: {}", e))?;

    println!("Storage test successful: {:?}", retrieved_value);

    // Test completed - events now via EventBus if needed
    // platform_context.event_bus.emit(...).ok();

    Ok("Universal test flow completed successfully".to_string())
}
