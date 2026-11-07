//! Service Plugin Runtime
//!
//! Manages external process-based plugins that communicate via JSON-RPC over stdin/stdout.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{oneshot, Mutex, RwLock};

use super::bundle::{PluginManifest, PluginPermission, ServiceEntry};
use super::interface::{FlowNodeMetadata, Plugin, PluginError};
use super::permissions::PermissionChecker;

const SERVICE_CALL_TIMEOUT: tokio::time::Duration = tokio::time::Duration::from_secs(30);
const MAX_RPC_LINE_BYTES: usize = 4 * 1024 * 1024;
const MAX_STDERR_LINE_BYTES: usize = 64 * 1024;
const MAX_HOST_PAYLOAD_BYTES: usize = 4 * 1024 * 1024;

async fn read_bounded_line<R>(
    reader: &mut BufReader<R>,
    line: &mut String,
    max_bytes: usize,
) -> std::io::Result<usize>
where
    R: AsyncRead + Unpin,
{
    line.clear();
    let mut bounded = reader.take((max_bytes + 1) as u64);
    bounded.read_line(line).await
}

async fn terminate_child(process: &Arc<Mutex<Child>>) -> Result<(), String> {
    let mut process = process.lock().await;
    if process
        .try_wait()
        .map_err(|error| format!("Failed to inspect plugin process: {error}"))?
        .is_some()
    {
        return Ok(());
    }
    process
        .kill()
        .await
        .map_err(|error| format!("Failed to terminate plugin process: {error}"))?;
    tokio::time::timeout(tokio::time::Duration::from_secs(2), process.wait())
        .await
        .map_err(|_| "Plugin process termination was not reaped within 2 seconds".to_string())?
        .map_err(|error| format!("Failed to reap plugin process: {error}"))?;
    Ok(())
}

/// Service JSON-RPC has no per-request cancellation primitive. Once a request
/// may be dispatched, dropping its owner therefore terminates the owned child
/// so the request cannot begin or continue invisibly after cancellation.
struct ServiceCallGuard {
    id: u64,
    active: bool,
    process: Arc<Mutex<Child>>,
    pending: Arc<RwLock<HashMap<u64, oneshot::Sender<Result<Value, String>>>>>,
}

impl ServiceCallGuard {
    fn disarm(&mut self) {
        self.active = false;
    }
}

impl Drop for ServiceCallGuard {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        let id = self.id;
        let process = self.process.clone();
        let pending = self.pending.clone();
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(async move {
                pending.write().await.remove(&id);
                let _ = terminate_child(&process).await;
            });
        }
    }
}

/// Resolve a manifest `cwd` against the plugin folder. Only canonical
/// package-relative paths are accepted so the process cannot start outside it.
pub(crate) fn service_working_directory(
    plugin_folder: &Path,
    cwd: Option<&str>,
) -> Result<PathBuf, String> {
    match cwd {
        None | Some("") | Some(".") => Ok(plugin_folder.to_path_buf()),
        Some(path) => super::bundle::validate_relative_artifact_path(path)
            .map(|relative| plugin_folder.join(relative))
            .map_err(|error| format!("Invalid service working directory: {error}")),
    }
}

/// JSON-RPC message types
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum JsonRpcMessage {
    Request(JsonRpcRequest),
    Response(JsonRpcResponse),
    Notification(JsonRpcNotification),
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcNotification {
    jsonrpc: String,
    method: String,
    params: Value,
}

/// Service plugin instance
pub struct ServicePlugin {
    plugin_id: String,
    manifest: PluginManifest,
    process: Arc<Mutex<Child>>,
    stdin: Arc<Mutex<BufWriter<ChildStdin>>>,
    request_id: Arc<AtomicU64>,
    pending_requests: Arc<RwLock<HashMap<u64, oneshot::Sender<Result<Value, String>>>>>,
    nodes: Arc<RwLock<Vec<FlowNodeMetadata>>>,
    permission_checker: Arc<PermissionChecker>,
}

impl ServicePlugin {
    /// Spawn a service plugin process
    pub async fn spawn(
        manifest: PluginManifest,
        plugin_folder: PathBuf,
        permission_checker: Arc<PermissionChecker>,
    ) -> Result<Self, String> {
        println!("📦 Starting service plugin: {}", manifest.name);

        let service_entry = manifest
            .entry
            .get("service")
            .ok_or("No service entry in manifest")?;

        let service_config: ServiceEntry = match service_entry {
            super::bundle::EntryConfig::Service(config) => config.clone(),
            _ => return Err("Invalid service entry config".to_string()),
        };

        // A service runs a native process with the user's full rights, so it
        // requires the explicitly consented ExecuteCommands grant.
        permission_checker
            .enforce_permission(&manifest.id, &PluginPermission::ExecuteCommands)
            .map_err(|error| {
                format!(
                    "Refusing to start service plugin '{}': {error}",
                    manifest.id
                )
            })?;

        // Determine working directory (must stay inside the plugin folder)
        let cwd = service_working_directory(&plugin_folder, service_config.cwd.as_deref())?;

        println!("  📂 Working directory: {:?}", cwd);
        println!(
            "  🚀 Command: {} {:?}",
            service_config.command, service_config.args
        );

        // Spawn process
        let mut child = Command::new(&service_config.command)
            .args(&service_config.args)
            .current_dir(&cwd)
            .envs(&service_config.env)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| format!("Failed to spawn plugin process: {}", e))?;

        // Get stdin/stdout handles
        let stdin = BufWriter::new(child.stdin.take().unwrap());
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let stderr = BufReader::new(child.stderr.take().unwrap());

        let plugin_id = manifest.id.clone();
        let plugin_name = manifest.name.clone();

        let plugin = Self {
            plugin_id: plugin_id.clone(),
            manifest,
            process: Arc::new(Mutex::new(child)),
            stdin: Arc::new(Mutex::new(stdin)),
            request_id: Arc::new(AtomicU64::new(0)),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            nodes: Arc::new(RwLock::new(Vec::new())),
            permission_checker,
        };

        // Spawn stdout reader task
        plugin.spawn_stdout_reader(stdout).await;

        // Spawn stderr reader task (for logging)
        plugin.spawn_stderr_reader(stderr, plugin_name).await;

        // Wait for plugin to register its nodes
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        println!("✅ Service plugin started: {}", plugin_id);

        Ok(plugin)
    }

    /// Spawn task to read stdout (JSON-RPC messages)
    async fn spawn_stdout_reader(&self, mut stdout: BufReader<ChildStdout>) {
        let pending = self.pending_requests.clone();
        let plugin_id = self.plugin_id.clone();
        let nodes = self.nodes.clone();
        let stdin = self.stdin.clone();
        let process = self.process.clone();
        let permission_checker = self.permission_checker.clone();

        tokio::spawn(async move {
            let mut line = String::new();

            loop {
                match read_bounded_line(&mut stdout, &mut line, MAX_RPC_LINE_BYTES).await {
                    Ok(0) => {
                        println!("⚠️ Plugin {} stdout closed", plugin_id);
                        break;
                    }
                    Ok(_) => {
                        if line.len() > MAX_RPC_LINE_BYTES {
                            eprintln!(
                                "❌ Plugin {} exceeded the {} byte JSON-RPC line limit",
                                plugin_id, MAX_RPC_LINE_BYTES
                            );
                            let _ = terminate_child(&process).await;
                            break;
                        }
                        if line.trim().is_empty() {
                            continue;
                        }

                        match serde_json::from_str::<JsonRpcMessage>(&line) {
                            Ok(JsonRpcMessage::Response(resp)) => {
                                // Handle response to our request
                                println!(
                                    "📥 Received response from plugin {}: id={} has_error={}",
                                    plugin_id,
                                    resp.id,
                                    resp.error.is_some()
                                );

                                let mut pending_map = pending.write().await;
                                match pending_map.remove(&resp.id) {
                                    Some(sender) => {
                                        let result = match resp.error {
                                            Some(error) => Err(format!(
                                                "Service plugin error {}: {}",
                                                error.code, error.message
                                            )),
                                            None => Ok(resp.result.unwrap_or(Value::Null)),
                                        };
                                        let _ = sender.send(result);
                                    }
                                    _ => {
                                        println!(
                                            "⚠️ No pending request found for response id={}",
                                            resp.id
                                        );
                                    }
                                }
                            }
                            Ok(JsonRpcMessage::Request(req)) => {
                                // Plugin calling VortiDeck API
                                Self::handle_plugin_request(
                                    req,
                                    &plugin_id,
                                    &stdin,
                                    &permission_checker,
                                )
                                .await;
                            }
                            Ok(JsonRpcMessage::Notification(notif)) => {
                                // Plugin notification (e.g., node registration)
                                if notif.method == "vortideck.registerNodes" {
                                    Self::handle_node_registration(notif.params, &nodes).await;
                                }
                            }
                            Err(e) => {
                                eprintln!(
                                    "❌ Failed to parse JSON-RPC from {}: {} (line: {})",
                                    plugin_id,
                                    e,
                                    line.trim()
                                );
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Plugin {} stdout error: {}", plugin_id, e);
                        break;
                    }
                }
            }
            // Wake every dispatched caller when the transport closes. They
            // must not remain pending until independent outer timeouts fire.
            pending.write().await.clear();
        });
    }

    /// Spawn task to read stderr (for logging)
    async fn spawn_stderr_reader(
        &self,
        mut stderr: BufReader<tokio::process::ChildStderr>,
        plugin_name: String,
    ) {
        tokio::spawn(async move {
            let mut line = String::new();

            loop {
                match read_bounded_line(&mut stderr, &mut line, MAX_STDERR_LINE_BYTES).await {
                    Ok(0) => break,
                    Ok(_) => {
                        if !line.trim().is_empty() {
                            if line.len() > MAX_STDERR_LINE_BYTES {
                                println!(
                                    "🔌 [{}] stderr line truncated at {} bytes",
                                    plugin_name, MAX_STDERR_LINE_BYTES
                                );
                            } else {
                                println!("🔌 [{}] {}", plugin_name, line.trim());
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });
    }

    /// Handle plugin calling VortiDeck API
    async fn handle_plugin_request(
        req: JsonRpcRequest,
        plugin_id: &str,
        stdin: &Arc<Mutex<BufWriter<ChildStdin>>>,
        permission_checker: &PermissionChecker,
    ) {
        println!("📞 Plugin {} calling: {}", plugin_id, req.method);

        // Route to appropriate handler
        let result = match req.method.as_str() {
            "vortideck.http.request" => {
                Self::handle_http_request(req.params, plugin_id, permission_checker).await
            }
            "vortideck.file.read" => {
                Self::handle_file_read(req.params, plugin_id, permission_checker).await
            }
            "vortideck.file.write" => {
                Self::handle_file_write(req.params, plugin_id, permission_checker).await
            }
            "vortideck.log" => Self::handle_log(req.params).await,
            _ => Err(format!("Unknown method: {}", req.method)),
        };

        // Send response back to plugin
        let response = match result {
            Ok(value) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: Some(value),
                error: None,
            },
            Err(err_msg) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32000,
                    message: err_msg,
                }),
            },
        };

        // Send response
        if let Ok(mut response_line) = serde_json::to_string(&response) {
            response_line.push('\n');

            println!(
                "📤 Sending response to plugin {}: {} (method: {})",
                plugin_id,
                if response.error.is_some() {
                    "ERROR"
                } else {
                    "OK"
                },
                req.method
            );

            let mut stdin_lock = stdin.lock().await;
            let _ = stdin_lock.write_all(response_line.as_bytes()).await;
            let _ = stdin_lock.flush().await;
        }
    }

    /// Handle HTTP request
    async fn handle_http_request(
        params: Value,
        plugin_id: &str,
        permission_checker: &PermissionChecker,
    ) -> Result<Value, String> {
        use reqwest::Client;

        let url = params
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'url' parameter")?;
        permission_checker.enforce_network_url(plugin_id, url)?;

        let method = params
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET");

        let headers = params.get("headers").and_then(|v| v.as_object());
        let body = params.get("body");

        println!("🌐 HTTP {} {}", method, url);

        // Redirects are deliberately not followed: a scoped grant authorizes
        // the requested URL, not an arbitrary redirect destination.
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| format!("Failed to create plugin HTTP client: {error}"))?;
        let mut request = match method.to_uppercase().as_str() {
            "GET" => client.get(url),
            "POST" => client.post(url),
            "PUT" => client.put(url),
            "DELETE" => client.delete(url),
            "PATCH" => client.patch(url),
            _ => return Err(format!("Unsupported HTTP method: {}", method)),
        };

        // Add headers
        if let Some(headers_obj) = headers {
            for (key, value) in headers_obj {
                if let Some(value_str) = value.as_str() {
                    request = request.header(key, value_str);
                }
            }
        }

        // Add body
        if let Some(body_value) = body {
            if let Some(body_str) = body_value.as_str() {
                request = request.body(body_str.to_string());
            } else {
                request = request.json(body_value);
            }
        }

        let mut response = request
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        let status = response.status().as_u16();
        let headers: serde_json::Map<String, Value> = response
            .headers()
            .iter()
            .map(|(k, v)| {
                (
                    k.to_string(),
                    Value::String(v.to_str().unwrap_or("").to_string()),
                )
            })
            .collect();

        if response
            .content_length()
            .is_some_and(|length| length > MAX_HOST_PAYLOAD_BYTES as u64)
        {
            return Err("Plugin HTTP responses are limited to 4 MiB".to_string());
        }
        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| format!("Failed to read response body: {e}"))?
        {
            if body.len().saturating_add(chunk.len()) > MAX_HOST_PAYLOAD_BYTES {
                return Err("Plugin HTTP responses are limited to 4 MiB".to_string());
            }
            body.extend_from_slice(&chunk);
        }
        let body = String::from_utf8_lossy(&body).into_owned();

        Ok(serde_json::json!({
            "status": status,
            "headers": headers,
            "body": body,
        }))
    }

    /// Handle file read
    async fn handle_file_read(
        params: Value,
        plugin_id: &str,
        permission_checker: &PermissionChecker,
    ) -> Result<Value, String> {
        use tokio::fs;

        let path = params
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'path' parameter")?;
        permission_checker
            .enforce_permission(plugin_id, &super::bundle::PluginPermission::FileSystem)?;
        let path = super::permissions::resolve_workspace_read_path(path)?;

        println!("📄 Reading file: {}", path.display());

        let metadata = fs::metadata(&path)
            .await
            .map_err(|e| format!("Failed to inspect file: {e}"))?;
        if metadata.len() > MAX_HOST_PAYLOAD_BYTES as u64 {
            return Err("Plugin file reads are limited to 4 MiB".to_string());
        }
        let content = fs::read_to_string(path)
            .await
            .map_err(|e| format!("Failed to read file: {}", e))?;

        Ok(Value::String(content))
    }

    /// Handle file write
    async fn handle_file_write(
        params: Value,
        plugin_id: &str,
        permission_checker: &PermissionChecker,
    ) -> Result<Value, String> {
        use tokio::fs;

        let path = params
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'path' parameter")?;

        let content = params
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'content' parameter")?;
        permission_checker
            .enforce_permission(plugin_id, &super::bundle::PluginPermission::FileSystem)?;
        if content.len() > MAX_HOST_PAYLOAD_BYTES {
            return Err("Plugin file writes are limited to 4 MiB".to_string());
        }
        let path = super::permissions::resolve_workspace_write_path(path)?;

        println!("📝 Writing file: {}", path.display());

        fs::write(path, content)
            .await
            .map_err(|e| format!("Failed to write file: {}", e))?;

        Ok(Value::Bool(true))
    }

    /// Handle log message
    async fn handle_log(params: Value) -> Result<Value, String> {
        let message = params
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'message' parameter")?;

        let level = params
            .get("level")
            .and_then(|v| v.as_str())
            .unwrap_or("info");

        match level {
            "error" => eprintln!("❌ Plugin: {}", message),
            "warn" => println!("⚠️ Plugin: {}", message),
            "debug" => println!("🐛 Plugin: {}", message),
            _ => println!("ℹ️ Plugin: {}", message),
        }

        Ok(Value::Bool(true))
    }

    /// Handle node registration from plugin
    async fn handle_node_registration(params: Value, nodes: &Arc<RwLock<Vec<FlowNodeMetadata>>>) {
        if let Ok(registered_nodes) = serde_json::from_value::<Vec<FlowNodeMetadata>>(params) {
            let mut nodes_lock = nodes.write().await;
            // registerNodes declares the complete current catalog. Appending
            // would retain removed nodes and duplicate unchanged ones.
            *nodes_lock = registered_nodes.clone();

            println!("📝 Registered {} nodes from plugin", registered_nodes.len());
        }
    }

    /// Call a method on the plugin
    pub async fn call_method(&self, method: &str, params: Value) -> Result<Value, String> {
        self.call_method_with_timeout(method, params, SERVICE_CALL_TIMEOUT)
            .await
    }

    async fn call_method_with_timeout(
        &self,
        method: &str,
        params: Value,
        call_timeout: tokio::time::Duration,
    ) -> Result<Value, String> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();

        // Create JSON-RPC request
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        };

        // Send to plugin
        let mut line = serde_json::to_string(&request)
            .map_err(|e| format!("Failed to serialize request: {}", e))?;
        line.push('\n');
        if line.len() > MAX_RPC_LINE_BYTES {
            return Err(format!(
                "Service plugin request exceeds the {} byte limit; no call was started",
                MAX_RPC_LINE_BYTES
            ));
        }

        // Insert immediately before dispatch so a fast response cannot race
        // registration, while serialization failures leave no stale entry.
        self.pending_requests.write().await.insert(id, tx);
        let mut call_guard = ServiceCallGuard {
            id,
            active: true,
            process: self.process.clone(),
            pending: self.pending_requests.clone(),
        };

        // Send request and RELEASE lock before waiting for response
        // This prevents deadlock when plugin calls back to VortiDeck (e.g., HTTP request)
        {
            let mut stdin = self.stdin.lock().await;
            if let Err(error) = stdin.write_all(line.as_bytes()).await {
                self.pending_requests.write().await.remove(&id);
                let termination = terminate_child(&self.process).await;
                call_guard.disarm();
                return Err(match termination {
                    Ok(()) => format!(
                        "Failed to write to plugin stdin ({error}); the process was terminated, but a partially dispatched effect is uncertain"
                    ),
                    Err(cleanup) => format!(
                        "Failed to write to plugin stdin ({error}); effect outcome is uncertain and cleanup failed: {cleanup}"
                    ),
                });
            }
            if let Err(error) = stdin.flush().await {
                self.pending_requests.write().await.remove(&id);
                let termination = terminate_child(&self.process).await;
                call_guard.disarm();
                return Err(match termination {
                    Ok(()) => format!(
                        "Failed to flush plugin stdin ({error}); the process was terminated, but a dispatched effect is uncertain"
                    ),
                    Err(cleanup) => format!(
                        "Failed to flush plugin stdin ({error}); effect outcome is uncertain and cleanup failed: {cleanup}"
                    ),
                });
            }
        } // Lock is dropped here

        match tokio::time::timeout(call_timeout, rx).await {
            Ok(Ok(result)) => {
                call_guard.disarm();
                result
            }
            Ok(Err(_)) => {
                self.pending_requests.write().await.remove(&id);
                let termination = terminate_child(&self.process).await;
                call_guard.disarm();
                Err(match termination {
                    Ok(()) => "Service plugin transport closed after dispatch; its process was terminated and the effect outcome is uncertain".to_string(),
                    Err(error) => format!("Service plugin transport closed after dispatch; effect outcome is uncertain and process cleanup failed: {error}"),
                })
            }
            Err(_) => {
                self.pending_requests.write().await.remove(&id);
                let termination = terminate_child(&self.process).await;
                call_guard.disarm();
                Err(match termination {
                    Ok(()) => format!(
                        "Service plugin call exceeded {} ms; the owned process was terminated and reaped, but already-started external effects may be irreversible",
                        call_timeout.as_millis()
                    ),
                    Err(error) => format!(
                        "Service plugin call exceeded {} ms; effect outcome is uncertain and process cleanup failed: {error}",
                        call_timeout.as_millis()
                    ),
                })
            }
        }
    }

    /// Get registered nodes
    pub async fn get_nodes(&self) -> Vec<FlowNodeMetadata> {
        self.nodes.read().await.clone()
    }

    /// Shutdown the plugin
    pub async fn shutdown(&self) -> Result<(), String> {
        println!("🛑 Shutting down service plugin: {}", self.plugin_id);

        terminate_child(&self.process).await
    }
}

#[async_trait::async_trait]
impl Plugin for ServicePlugin {
    fn id(&self) -> &str {
        &self.plugin_id
    }

    fn name(&self) -> &str {
        &self.manifest.name
    }

    fn version(&self) -> &str {
        &self.manifest.version
    }

    fn metadata(&self) -> FlowNodeMetadata {
        FlowNodeMetadata {
            id: self.plugin_id.clone(),
            name: self.manifest.name.clone(),
            version: self.manifest.version.clone(),
            description: self.manifest.description.clone(),
            author: self.manifest.author.clone(),
            icon_path: None,
            plugin_bundle: None,
            category: None,
            parameters_schema: None,
            output_schema: None,
            type_color: None,
        }
    }

    async fn initialize(&self) -> Result<(), PluginError> {
        println!("🚀 Initializing service plugin: {}", self.manifest.name);
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), PluginError> {
        self.shutdown().await.map_err(|e| PluginError::Error(e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn bounded_line_reader_never_accumulates_an_unbounded_frame() {
        let (mut writer, reader) = tokio::io::duplex(128);
        let write = tokio::spawn(async move {
            writer.write_all(&vec![b'x'; 65]).await.unwrap();
            writer.write_all(b"\n").await.unwrap();
        });
        let mut reader = BufReader::new(reader);
        let mut line = String::new();
        let bytes = read_bounded_line(&mut reader, &mut line, 64).await.unwrap();
        assert_eq!(bytes, 65);
        assert_eq!(line.len(), 65);
        write.await.unwrap();
    }

    // This test is also launched as a child test process by the fixture below.
    #[test]
    fn unresponsive_service_child_fixture() {
        if std::env::var_os("VORTIDECK_UNRESPONSIVE_SERVICE_FIXTURE").is_some() {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    }

    fn unresponsive_service_manifest(id: &str) -> PluginManifest {
        let executable = std::env::current_exe().unwrap();
        let exact_test = "plugins::service_plugin::tests::unresponsive_service_child_fixture";
        let service = ServiceEntry {
            command: executable.to_string_lossy().into_owned(),
            args: vec![
                "--exact".to_string(),
                exact_test.to_string(),
                "--nocapture".to_string(),
            ],
            cwd: None,
            env: HashMap::from([(
                "VORTIDECK_UNRESPONSIVE_SERVICE_FIXTURE".to_string(),
                "1".to_string(),
            )]),
        };
        PluginManifest {
            id: id.to_string(),
            name: "Unresponsive service fixture".to_string(),
            version: "1.0.0".to_string(),
            author: "VortiDeck tests".to_string(),
            description: "Unresponsive owned test child".to_string(),
            vortideck_version: ">=0.7.0".to_string(),
            schema_version: super::super::bundle::CURRENT_SCHEMA_VERSION.to_string(),
            permissions: Vec::new(),
            web_extensions: false,
            main: String::new(),
            runtime: super::super::bundle::PluginRuntime::Service,
            entry: HashMap::from([(
                "service".to_string(),
                super::super::bundle::EntryConfig::Service(service),
            )]),
            web: None,
            contributions: Vec::new(),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn timed_out_service_call_terminates_reaps_and_clears_pending_request() {
        let plugin = ServicePlugin::spawn(
            unresponsive_service_manifest("service-timeout-fixture"),
            std::env::temp_dir(),
            command_grant("service-timeout-fixture"),
        )
        .await
        .expect("fixture process starts");

        let error = plugin
            .call_method_with_timeout(
                "execute",
                serde_json::json!({}),
                tokio::time::Duration::from_millis(50),
            )
            .await
            .expect_err("unresponsive child must time out");
        assert!(error.contains("terminated and reaped"), "{error}");
        assert!(plugin.pending_requests.read().await.is_empty());
        assert!(plugin.process.lock().await.try_wait().unwrap().is_some());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cancelling_dispatched_service_call_stops_the_owned_child() {
        let plugin = Arc::new(
            ServicePlugin::spawn(
                unresponsive_service_manifest("service-cancel-fixture"),
                std::env::temp_dir(),
                command_grant("service-cancel-fixture"),
            )
            .await
            .expect("fixture process starts"),
        );
        let caller = plugin.clone();
        let task = tokio::spawn(async move {
            caller
                .call_method_with_timeout(
                    "execute",
                    serde_json::json!({}),
                    tokio::time::Duration::from_secs(30),
                )
                .await
        });

        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            while plugin.pending_requests.read().await.is_empty() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("request was dispatched");
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());

        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                if plugin.process.lock().await.try_wait().unwrap().is_some()
                    && plugin.pending_requests.read().await.is_empty()
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("drop guard terminated and reaped the child");
    }

    #[tokio::test]
    async fn service_host_rejects_network_without_a_durable_grant() {
        let checker = PermissionChecker::new();
        let error = ServicePlugin::handle_http_request(
            serde_json::json!({ "url": "https://example.test" }),
            "untrusted-service",
            &checker,
        )
        .await
        .expect_err("network call must fail before transport");
        assert!(error.contains("Permission denied"), "{error}");
    }

    #[tokio::test]
    async fn service_host_rejects_workspace_traversal_with_a_file_grant() {
        let checker = PermissionChecker::new();
        checker.grant_permissions(
            "file-service",
            vec![super::super::bundle::PluginPermission::FileSystem],
        );
        let error = ServicePlugin::handle_file_read(
            serde_json::json!({ "path": "../outside.txt" }),
            "file-service",
            &checker,
        )
        .await
        .expect_err("workspace traversal must fail before file IO");
        assert!(
            error.contains("relative")
                || error.contains("Security")
                || error.contains("Non-canonical"),
            "{error}"
        );
    }

    fn command_grant(plugin_id: &str) -> Arc<PermissionChecker> {
        let checker = PermissionChecker::new();
        checker.grant_permissions(plugin_id, vec![PluginPermission::ExecuteCommands]);
        Arc::new(checker)
    }

    #[tokio::test]
    async fn service_spawn_requires_execute_commands_grant() {
        let checker = PermissionChecker::new();
        checker.grant_permissions("service-no-exec", vec![PluginPermission::Network]);
        let error = match ServicePlugin::spawn(
            unresponsive_service_manifest("service-no-exec"),
            std::env::temp_dir(),
            Arc::new(checker),
        )
        .await
        {
            Ok(_) => panic!("service without ExecuteCommands must not start"),
            Err(error) => error,
        };
        assert!(error.contains("Permission denied"), "{error}");
        assert!(error.contains("ExecuteCommands"), "{error}");
    }

    #[tokio::test]
    async fn service_spawn_rejects_cwd_outside_plugin_folder() {
        let mut manifest = unresponsive_service_manifest("service-cwd-escape");
        if let Some(super::super::bundle::EntryConfig::Service(service)) =
            manifest.entry.get_mut("service")
        {
            service.cwd = Some("../outside".to_string());
        }
        let error = match ServicePlugin::spawn(
            manifest,
            std::env::temp_dir(),
            command_grant("service-cwd-escape"),
        )
        .await
        {
            Ok(_) => panic!("escaping cwd must not start"),
            Err(error) => error,
        };
        assert!(
            error.contains("Invalid service working directory"),
            "{error}"
        );
    }

    #[test]
    fn service_working_directory_stays_inside_plugin_folder() {
        let root = Path::new("plugin-root");
        assert_eq!(service_working_directory(root, None).unwrap(), root);
        assert_eq!(service_working_directory(root, Some(".")).unwrap(), root);
        assert_eq!(
            service_working_directory(root, Some("bin/service")).unwrap(),
            root.join("bin").join("service")
        );
        for escape in ["..", "../x", "bin/../..", "/etc", "a\\b"] {
            assert!(
                service_working_directory(root, Some(escape)).is_err(),
                "{escape} must be rejected"
            );
        }
    }
}
