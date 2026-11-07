use serde_json::Value;
use std::net::IpAddr;
use std::path::PathBuf;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TailscaleStatus {
    pub ips: Vec<IpAddr>,
    pub dns_name: Option<String>,
}

/// Read the local Tailscale daemon state without changing tailnet settings.
/// The CLI's JSON output is Tailscale's supported machine-readable interface.
pub async fn detect() -> Option<TailscaleStatus> {
    for executable in executable_candidates() {
        let mut command = Command::new(&executable);
        command.args(["status", "--json"]);
        #[cfg(windows)]
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

        let output = match timeout(Duration::from_secs(2), command.output()).await {
            Ok(Ok(output)) => output,
            Ok(Err(error)) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Ok(Err(error)) => {
                log::debug!("Unable to query Tailscale status: {}", error);
                return None;
            }
            Err(_) => {
                log::debug!("Tailscale status query timed out");
                return None;
            }
        };

        if !output.status.success() {
            log::debug!("Tailscale CLI is present but the daemon is not ready");
            return None;
        }

        return serde_json::from_slice::<Value>(&output.stdout)
            .ok()
            .and_then(|value| parse_status(&value));
    }
    None
}

fn executable_candidates() -> Vec<PathBuf> {
    let mut candidates = vec![PathBuf::from("tailscale")];
    #[cfg(windows)]
    for program_files_variable in ["ProgramW6432", "ProgramFiles"] {
        if let Some(program_files) = std::env::var_os(program_files_variable) {
            candidates.push(
                PathBuf::from(program_files)
                    .join("Tailscale")
                    .join("tailscale.exe"),
            );
        }
    }
    candidates.dedup();
    candidates
}

fn parse_status(value: &Value) -> Option<TailscaleStatus> {
    if value.get("BackendState").and_then(Value::as_str) != Some("Running") {
        return None;
    }

    let addresses = value
        .get("TailscaleIPs")
        .and_then(Value::as_array)
        .or_else(|| {
            value
                .get("Self")
                .and_then(|self_node| self_node.get("TailscaleIPs"))
                .and_then(Value::as_array)
        })?;

    let mut ips: Vec<IpAddr> = addresses
        .iter()
        .filter_map(Value::as_str)
        .filter_map(|address| address.parse().ok())
        .filter(|address: &IpAddr| !address.is_loopback())
        .collect();
    ips.sort_by_key(|address| if address.is_ipv4() { 0 } else { 1 });
    ips.dedup();
    if ips.is_empty() {
        return None;
    }

    let dns_name = value
        .get("Self")
        .and_then(|self_node| self_node.get("DNSName"))
        .and_then(Value::as_str)
        .map(|name| name.trim_end_matches('.').to_string())
        .filter(|name| !name.is_empty());

    Some(TailscaleStatus { ips, dns_name })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_running_status_and_normalizes_magic_dns_name() {
        let status = parse_status(&json!({
            "BackendState": "Running",
            "TailscaleIPs": ["100.90.80.70", "fd7a:115c:a1e0::1"],
            "Self": { "DNSName": "studio-pc.example.ts.net." }
        }))
        .unwrap();

        assert_eq!(status.ips[0], "100.90.80.70".parse::<IpAddr>().unwrap());
        assert_eq!(status.dns_name.as_deref(), Some("studio-pc.example.ts.net"));
    }

    #[test]
    fn ignores_a_non_running_daemon() {
        assert!(parse_status(&json!({
            "BackendState": "Stopped",
            "TailscaleIPs": ["100.90.80.70"]
        }))
        .is_none());
    }
}
