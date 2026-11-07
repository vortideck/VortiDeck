# VortiDeck Integration Template

This is a template for creating new VortiDeck integrations. Use this as a starting point to build integrations with your favorite applications.

## 🚀 Quick Start

1. **Copy this template**:
   ```bash
   cp -r examples/integration-templates/simple-app-integration integrations/community/my-app-integration
   cd integrations/community/my-app-integration
   ```

2. **Customize the integration**:
   - Edit `Cargo.toml` with your integration details
   - Edit `integration.toml` with your app configuration
   - Modify `src/lib.rs` to implement your app's functionality

3. **Build the integration**:
   ```bash
   cargo build --release
   ```

4. **Test the integration**:
   ```bash
   cargo test
   ```

## 📁 File Structure

```
my-app-integration/
├── Cargo.toml          # Rust project configuration
├── integration.toml    # Integration metadata and settings
├── src/
│   └── lib.rs         # Main integration implementation
└── README.md          # This file
```

## 🔧 Implementation Guide

### 1. Integration Metadata (`integration.toml`)

Update the basic information about your integration:

```toml
[integration]
name = "Discord Integration"
version = "1.0.0"
author = "Your Name"
description = "Control Discord from VortiDeck"
app_name = "Discord"
supported_platforms = ["windows", "macos", "linux"]
category = "communication"
```

### 2. Define Actions

List the actions your integration supports:

```toml
[[actions.available]]
id = "mute_mic"
name = "Mute Microphone"
description = "Toggle microphone mute in Discord"
icon = "mic-off"
category = "control"
```

### 3. Implement the Integration (`src/lib.rs`)

The main integration logic goes in your Rust code:

```rust
#[async_trait]
impl AppIntegration for MyAppIntegration {
    async fn execute_action(&self, action_id: &str, parameters: HashMap<String, Value>) -> IntegrationResult<Value> {
        match action_id {
            "mute_mic" => {
                // Your implementation here
                self.toggle_microphone().await
            },
            _ => Err(IntegrationError::UnsupportedAction(action_id.to_string())),
        }
    }
}
```

## 🎯 Common Integration Patterns

### HTTP API Integration

```rust
async fn send_api_request(&self, endpoint: &str, data: Value) -> Result<Value, String> {
    let client = reqwest::Client::new();
    let response = client
        .post(&format!("http://localhost:8080{}", endpoint))
        .json(&data)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    response.json().await.map_err(|e| e.to_string())
}
```

### WebSocket Integration

```rust
use tokio_tungstenite::{connect_async, tungstenite::Message};

async fn connect_websocket(&self) -> Result<(), String> {
    let (ws_stream, _) = connect_async("ws://localhost:8080/ws")
        .await
        .map_err(|e| e.to_string())?;
    
    // Handle WebSocket messages
    Ok(())
}
```

### Windows Application Control

```rust
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{FindWindowA, SendMessageA};

#[cfg(windows)]
async fn send_windows_message(&self, message: u32) -> Result<(), String> {
    unsafe {
        let hwnd = FindWindowA(None, s!("My App"));
        if hwnd.0 != 0 {
            SendMessageA(hwnd, message, None, None);
        }
    }
    Ok(())
}
```

## 🔒 Permissions and Security

### Required Permissions

Declare what permissions your integration needs in `integration.toml`:

```toml
[permissions]
network = true           # For HTTP/WebSocket connections
filesystem = false       # For reading/writing files
system_control = false   # For system-level operations
window_management = true # For controlling app windows
```

### Premium Features

If your integration requires premium features:

```toml
[features]
premium_required = true

[features.usage_limits]
actions_per_day = 50  # Limit for free users
```

## 🧪 Testing Your Integration

Create unit tests for your integration:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_integration_initialization() {
        let mut integration = MyAppIntegration::new();
        assert!(integration.initialize().await.is_ok());
    }

    #[tokio::test]
    async fn test_action_execution() {
        let integration = MyAppIntegration::new();
        let result = integration.execute_action("get_status", HashMap::new()).await;
        assert!(result.is_ok());
    }
}
```

## 📦 Publishing Your Integration

### For Community Integrations

1. Place your integration in `integrations/community/`
2. Create a pull request to the VortiDeck repository
3. Include tests and documentation

### For Official Integrations

1. Contact the VortiDeck team
2. Follow the official integration guidelines
3. Pass security and quality reviews

### For Local/Private Integrations

1. Place your integration in `integrations/local/`
2. These won't be shared publicly but will work locally

## 🆘 Troubleshooting

### Common Issues

1. **Integration not loading**: Check that your `create_integration()` function is properly exported
2. **Actions not working**: Verify action IDs match between `integration.toml` and your Rust code
3. **Build errors**: Ensure all dependencies are correctly specified in `Cargo.toml`

### Debug Mode

Enable debug logging in your integration:

```rust
use log::{info, debug, error};

async fn execute_action(&self, action_id: &str, parameters: HashMap<String, Value>) -> IntegrationResult<Value> {
    debug!("Executing action: {} with params: {:?}", action_id, parameters);
    // ... implementation
}
```

## 📚 Additional Resources

- VortiDeck Integration API Documentation
- Example Integrations
- VortiDeck Core API Reference

## 💡 Tips for Success

1. **Start Simple**: Begin with basic actions and add complexity gradually
2. **Error Handling**: Always provide clear error messages
3. **Documentation**: Document your actions and parameters clearly
4. **Testing**: Write comprehensive tests for your integration
5. **Performance**: Consider caching and rate limiting for external APIs

## 🤝 Contributing

If you create a useful integration, consider sharing it with the community!

1. Fork the VortiDeck repository
2. Add your integration to `integrations/community/`
3. Create a pull request with your integration

---

Happy integrating! 🚀 