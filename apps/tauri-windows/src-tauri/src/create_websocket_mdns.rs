use mdns_sd::{ServiceDaemon, ServiceInfo};
use serde::Serialize;
use std::sync::Mutex;

const MDNS_SERVICE_TYPE: &str = "_vortideck._tcp.local.";

struct MdnsRuntimeState {
    daemon: Option<ServiceDaemon>,
    advertisement: Option<MdnsAdvertisement>,
    error: Option<String>,
}

static MDNS_STATE: Mutex<MdnsRuntimeState> = Mutex::new(MdnsRuntimeState {
    daemon: None,
    advertisement: None,
    error: None,
});

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MdnsAdvertisement {
    pub service_type: String,
    pub instance_name: String,
    pub hostname: String,
    pub display_hostname: String,
    pub ip: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MdnsStatus {
    pub active: bool,
    pub advertisement: Option<MdnsAdvertisement>,
    pub error: Option<String>,
}

fn trim_local_suffix(hostname: &str) -> &str {
    hostname
        .strip_suffix(".local.")
        .or_else(|| hostname.strip_suffix(".local"))
        .unwrap_or(hostname)
}

fn short_desktop_id(desktop_id: Option<&str>) -> Option<String> {
    let value = desktop_id?
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .take(8)
        .collect::<String>()
        .to_ascii_lowercase();
    (!value.is_empty()).then_some(value)
}

fn sanitize_dns_label(value: &str) -> String {
    let mut label = String::new();
    let mut previous_was_separator = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            label.push(character.to_ascii_lowercase());
            previous_was_separator = false;
        } else if !previous_was_separator && !label.is_empty() {
            label.push('-');
            previous_was_separator = true;
        }
    }
    label.trim_matches('-').to_string()
}

fn build_advertisement(
    ip: String,
    port: u16,
    desktop_id: Option<&str>,
    hostname: Option<&str>,
) -> MdnsAdvertisement {
    let display_hostname = hostname
        .map(trim_local_suffix)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("Desktop")
        .to_string();
    let suffix = short_desktop_id(desktop_id);
    let mut safe_hostname = sanitize_dns_label(&display_hostname);
    if safe_hostname.is_empty() {
        safe_hostname = "vortideck".to_string();
    }

    let suffix_length = suffix.as_ref().map_or(0, |value| value.len() + 1);
    safe_hostname.truncate(63usize.saturating_sub(suffix_length));
    safe_hostname = safe_hostname.trim_matches('-').to_string();
    if let Some(ref suffix) = suffix {
        safe_hostname.push('-');
        safe_hostname.push_str(suffix);
    }

    let instance_name = match suffix {
        Some(suffix) => format!("VortiDeck on {} [{}]", display_hostname, suffix),
        None => format!("VortiDeck on {}", display_hostname),
    };

    MdnsAdvertisement {
        service_type: MDNS_SERVICE_TYPE.to_string(),
        instance_name,
        hostname: format!("{}.local.", safe_hostname),
        display_hostname,
        ip,
        port,
    }
}

fn public_service_properties(
    desktop_id: Option<&str>,
    hostname: Option<&str>,
) -> Vec<(String, String)> {
    let mut properties = vec![("version".to_string(), "1.0".to_string())];
    if let Some(id) = desktop_id {
        properties.push(("desktop_id".to_string(), id.to_string()));
    }
    #[cfg(target_os = "windows")]
    properties.push(("os".to_string(), "Windows".to_string()));
    #[cfg(target_os = "macos")]
    properties.push(("os".to_string(), "macOS".to_string()));
    #[cfg(target_os = "linux")]
    properties.push(("os".to_string(), "Linux".to_string()));
    if let Some(name) = hostname {
        properties.push(("hostname".to_string(), name.to_string()));
    }
    properties
}

pub async fn create_websocket_mdns(
    ip: String,
    port: u16,
    desktop_id: Option<String>,
    hostname: Option<String>,
) -> Result<(), String> {
    let advertisement = build_advertisement(ip, port, desktop_id.as_deref(), hostname.as_deref());
    let result = register_advertisement(&advertisement, desktop_id.as_deref());

    match result {
        Ok(mdns) => {
            let mut state = MDNS_STATE
                .lock()
                .map_err(|_| "mDNS runtime state is unavailable".to_string())?;
            state.daemon = Some(mdns);
            state.advertisement = Some(advertisement.clone());
            state.error = None;
            log::info!(
                "mDNS service advertised as {} ({}) at {}:{}",
                advertisement.instance_name,
                advertisement.hostname,
                advertisement.ip,
                advertisement.port
            );
            Ok(())
        }
        Err(error) => {
            if let Ok(mut state) = MDNS_STATE.lock() {
                state.daemon = None;
                state.advertisement = Some(advertisement);
                state.error = Some(error.clone());
            }
            Err(error)
        }
    }
}

fn register_advertisement(
    advertisement: &MdnsAdvertisement,
    desktop_id: Option<&str>,
) -> Result<ServiceDaemon, String> {
    let mdns = ServiceDaemon::new().map_err(|e| format!("Failed to create mDNS daemon: {e}"))?;

    // Properties to be advertised with the service
    // 🔐 Include desktop_id and hostname so mobile can check if already paired
    if let Some(id) = desktop_id {
        println!("📡 Publishing mDNS with desktop_id: {}", id);
    } else {
        println!("⚠️ No desktop_id provided for mDNS broadcast");
    }

    // Discovery metadata is public multicast data. Capabilities and account
    // credentials must never be added to these properties.
    let properties =
        public_service_properties(desktop_id, Some(advertisement.display_hostname.as_str()));

    // Create the service info with dynamic properties
    let properties_slice: Vec<(&str, &str)> = properties
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    let my_service = ServiceInfo::new(
        &advertisement.service_type,
        &advertisement.instance_name,
        &advertisement.hostname,
        &advertisement.ip,
        advertisement.port,
        &properties_slice[..], // Properties with desktop_id
    )
    .map_err(|e| e.to_string())?;

    // Register the service
    mdns.register(my_service)
        .map_err(|e| format!("Failed to register mDNS service: {}", e))?;

    Ok(mdns)
}

pub fn mdns_status_snapshot() -> Result<MdnsStatus, String> {
    let state = MDNS_STATE
        .lock()
        .map_err(|_| "mDNS runtime state is unavailable".to_string())?;
    Ok(MdnsStatus {
        active: state.daemon.is_some() && state.error.is_none(),
        advertisement: state.advertisement.clone(),
        error: state.error.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::{build_advertisement, public_service_properties, sanitize_dns_label};

    #[test]
    fn mdns_properties_are_discovery_only() {
        let properties = public_service_properties(Some("desktop-id"), Some("host.local."));
        let keys = properties
            .iter()
            .map(|(key, _)| key.as_str())
            .collect::<Vec<_>>();
        assert!(keys.contains(&"version"));
        assert!(keys.contains(&"desktop_id"));
        assert!(keys.contains(&"hostname"));
        assert!(!keys.iter().any(|key| {
            let key = key.to_ascii_lowercase();
            key.contains("token") || key.contains("secret") || key.contains("capability")
        }));
    }

    #[test]
    fn dns_label_replaces_spaces_and_invalid_characters() {
        assert_eq!(
            sanitize_dns_label(" Alexis Streaming_PC! "),
            "alexis-streaming-pc"
        );
    }

    #[test]
    fn advertisement_separates_friendly_and_dns_names() {
        let advertisement = build_advertisement(
            "192.168.1.8".to_string(),
            9001,
            Some("A31F-55AA-0000"),
            Some("Alexis Streaming PC"),
        );
        assert_eq!(
            advertisement.instance_name,
            "VortiDeck on Alexis Streaming PC [a31f55aa]"
        );
        assert_eq!(
            advertisement.hostname,
            "alexis-streaming-pc-a31f55aa.local."
        );
        assert_eq!(advertisement.display_hostname, "Alexis Streaming PC");
    }
}
