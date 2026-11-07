//! Authenticated loopback control plane for an optional background flow host.
//!
//! This transport deliberately exposes a small command surface over the same
//! [`FlowRuntime`] used by the desktop application. It does not construct a
//! second executor and it does not pretend that desktop-only capabilities are
//! available in a headless process.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use super::runtime::{FlowRuntime, FlowRuntimeStatus};

const MAX_REQUEST_BYTES: u64 = 64 * 1024;
const MIN_TOKEN_BYTES: usize = 32;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalHostRequest {
    request_id: String,
    token: String,
    #[serde(flatten)]
    command: LocalHostCommand,
}

#[derive(Debug, Deserialize)]
#[serde(
    tag = "operation",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum LocalHostCommand {
    Capabilities,
    ExecuteSavedFlow { flow_id: String },
    GetRun { run_id: String },
    InvokeDesktopCapability { capability: String },
    Shutdown,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalHostError {
    code: &'static str,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalHostResponse {
    request_id: String,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<LocalHostError>,
}

impl LocalHostResponse {
    fn success(request_id: String, result: Value) -> Self {
        Self {
            request_id,
            ok: true,
            result: Some(result),
            error: None,
        }
    }

    fn failure(request_id: String, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            request_id,
            ok: false,
            result: None,
            error: Some(LocalHostError {
                code,
                message: message.into(),
            }),
        }
    }
}

/// A bounded control server whose lifecycle owns the supplied runtime.
pub struct LocalFlowHost {
    runtime: FlowRuntime,
    token: String,
}

impl LocalFlowHost {
    pub fn new(runtime: FlowRuntime, token: impl Into<String>) -> Result<Self, String> {
        let token = token.into();
        if token.as_bytes().len() < MIN_TOKEN_BYTES {
            return Err(format!(
                "Local flow host token must contain at least {MIN_TOKEN_BYTES} bytes"
            ));
        }
        Ok(Self { runtime, token })
    }

    /// Serve one-request connections until an authenticated shutdown command.
    /// The listener must be loopback-bound; exposing this protocol to a LAN is
    /// rejected even when a token is configured.
    pub async fn serve(self, listener: TcpListener) -> Result<(), String> {
        let address = listener.local_addr().map_err(|error| error.to_string())?;
        if !address.ip().is_loopback() {
            return Err(format!(
                "Local flow host refuses non-loopback listener {address}"
            ));
        }

        match self.runtime.status() {
            FlowRuntimeStatus::Stopped => {
                self.runtime.start().await?;
            }
            FlowRuntimeStatus::Running => {}
            status => {
                return Err(format!(
                    "Local flow host cannot serve while runtime is {status:?}"
                ))
            }
        }

        let serve_result = self.serve_until_shutdown(listener).await;
        let stop_result = self.runtime.stop().await;
        serve_result.and(stop_result)
    }

    async fn serve_until_shutdown(&self, listener: TcpListener) -> Result<(), String> {
        loop {
            let (stream, peer) = listener.accept().await.map_err(|error| error.to_string())?;
            if !peer.ip().is_loopback() {
                log::warn!("Rejected non-loopback local-host peer {peer}");
                continue;
            }
            if self.handle_connection(stream).await? {
                return Ok(());
            }
        }
    }

    async fn handle_connection(&self, mut stream: TcpStream) -> Result<bool, String> {
        let mut bytes = Vec::new();
        (&mut stream)
            .take(MAX_REQUEST_BYTES + 1)
            .read_to_end(&mut bytes)
            .await
            .map_err(|error| format!("Failed to read local-host request: {error}"))?;

        let (response, shutdown) = if bytes.len() as u64 > MAX_REQUEST_BYTES {
            (
                LocalHostResponse::failure(
                    String::new(),
                    "request_too_large",
                    "Local-host request exceeds 64 KiB",
                ),
                false,
            )
        } else {
            match serde_json::from_slice::<LocalHostRequest>(&bytes) {
                Ok(request) => self.dispatch(request).await,
                Err(error) => (
                    LocalHostResponse::failure(
                        String::new(),
                        "invalid_request",
                        format!("Invalid local-host request: {error}"),
                    ),
                    false,
                ),
            }
        };

        let payload = serde_json::to_vec(&response).map_err(|error| error.to_string())?;
        stream
            .write_all(&payload)
            .await
            .map_err(|error| format!("Failed to write local-host response: {error}"))?;
        stream
            .shutdown()
            .await
            .map_err(|error| format!("Failed to close local-host response: {error}"))?;
        Ok(shutdown)
    }

    async fn dispatch(&self, request: LocalHostRequest) -> (LocalHostResponse, bool) {
        if !tokens_equal(request.token.as_bytes(), self.token.as_bytes()) {
            return (
                LocalHostResponse::failure(
                    request.request_id,
                    "unauthorized",
                    "Invalid local flow host token",
                ),
                false,
            );
        }

        let request_id = request.request_id;
        match request.command {
            LocalHostCommand::Capabilities => (
                LocalHostResponse::success(
                    request_id,
                    json!({
                        "flowExecution": true,
                        "durableRecovery": true,
                        "desktopSystemActions": false,
                        "windowControl": false,
                        "inputSimulation": false
                    }),
                ),
                false,
            ),
            LocalHostCommand::ExecuteSavedFlow { flow_id } => {
                let result = async {
                    let executor = self.runtime.executor().await?;
                    let flow = executor.load_flow(&flow_id).await?;
                    let run_id = executor.start_flow(flow).await?;
                    Ok::<_, String>(json!({ "flowId": flow_id, "runId": run_id }))
                }
                .await;
                (response_from_result(request_id, result), false)
            }
            LocalHostCommand::GetRun { run_id } => {
                let result = self.run_snapshot(&run_id).await;
                (response_from_result(request_id, result), false)
            }
            LocalHostCommand::InvokeDesktopCapability { capability } => (
                LocalHostResponse::failure(
                    request_id,
                    "capability_unavailable",
                    format!(
                        "Desktop capability '{capability}' is unavailable in the local background host"
                    ),
                ),
                false,
            ),
            LocalHostCommand::Shutdown => (
                LocalHostResponse::success(request_id, json!({ "shuttingDown": true })),
                true,
            ),
        }
    }

    async fn run_snapshot(&self, run_id: &str) -> Result<Value, String> {
        let executor = self.runtime.executor().await?;
        if let Ok(state) = executor.get_flow_status(run_id).await {
            return serde_json::to_value(state).map_err(|error| error.to_string());
        }
        let history = executor.list_execution_history(None, 500).await?;
        let entry = history
            .into_iter()
            .find(|entry| entry.run_id == run_id)
            .ok_or_else(|| format!("Workflow run '{run_id}' was not found"))?;
        serde_json::to_value(entry).map_err(|error| error.to_string())
    }
}

fn response_from_result(request_id: String, result: Result<Value, String>) -> LocalHostResponse {
    match result {
        Ok(value) => LocalHostResponse::success(request_id, value),
        Err(error) => LocalHostResponse::failure(request_id, "runtime_error", error),
    }
}

fn tokens_equal(left: &[u8], right: &[u8]) -> bool {
    let mut difference = left.len() ^ right.len();
    let length = left.len().max(right.len());
    for index in 0..length {
        difference |= left.get(index).copied().unwrap_or_default() as usize
            ^ right.get(index).copied().unwrap_or_default() as usize;
    }
    difference == 0
}

#[cfg(test)]
mod tests {
    use super::tokens_equal;

    #[test]
    fn token_comparison_rejects_value_and_length_changes() {
        assert!(tokens_equal(b"same", b"same"));
        assert!(!tokens_equal(b"same", b"some"));
        assert!(!tokens_equal(b"same", b"same-longer"));
    }
}
