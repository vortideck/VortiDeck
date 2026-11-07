//! OBS-only pairing and per-connection integrity. This does not encrypt traffic.
use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use sha2::Sha256;
use tauri_plugin_store::StoreExt;

pub fn credential(app: &tauri::AppHandle) -> Result<String, String> {
    let store = app
        .store("obs_companion_pairing.json")
        .map_err(|e| e.to_string())?;
    store
        .get("key")
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or_else(|| "Create an OBS pairing key in Canvas Settings first".into())
}

#[tauri::command]
pub fn obs_companion_pairing_key(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    rotate: bool,
) -> Result<String, String> {
    super::super::commands::overlay_commands::ensure_trusted_overlay_window(window.label())?;
    if !rotate {
        if let Ok(key) = credential(&app) {
            return Ok(key);
        }
    }
    let key = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let store = app
        .store("obs_companion_pairing.json")
        .map_err(|e| e.to_string())?;
    store.set("key", json!(key));
    store.save().map_err(|e| e.to_string())?;
    Ok(key)
}

#[derive(Clone)]
pub struct Session {
    pub key: String,
    pub id: String,
    sequence: u64,
}
impl Session {
    pub fn new(key: String, id: String) -> Self {
        Self {
            key,
            id,
            sequence: 0,
        }
    }
    fn mac(&self, direction: &str, sequence: u64, body: &str) -> Hmac<Sha256> {
        let mut mac =
            Hmac::<Sha256>::new_from_slice(self.key.as_bytes()).expect("HMAC accepts any key size");
        mac.update(
            format!(
                "vortideck-obs-v1\n{}\n{direction}\n{sequence}\n{body}",
                self.id
            )
            .as_bytes(),
        );
        mac
    }
    pub fn proof(&self, direction: &str) -> String {
        self.mac(direction, 0, "")
            .finalize()
            .into_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
    fn verify(&self, direction: &str, sequence: u64, body: &str, signature: &str) -> bool {
        if signature.len() != 64 || !signature.is_ascii() {
            return false;
        }
        let Ok(bytes) = (0..32)
            .map(|i| u8::from_str_radix(&signature[i * 2..i * 2 + 2], 16))
            .collect::<Result<Vec<_>, _>>()
        else {
            return false;
        };
        self.mac(direction, sequence, body)
            .verify_slice(&bytes)
            .is_ok()
    }
    pub fn verify_proof(&self, signature: &str) -> bool {
        self.verify("client", 0, "", signature)
    }
    pub fn wrap(&mut self, body: &str) -> String {
        self.sequence += 1;
        let signature: String = self
            .mac("host-message", self.sequence, body)
            .finalize()
            .into_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        json!({"obs_session":self.id,"sequence":self.sequence,"body":body,"signature":signature})
            .to_string()
    }
    pub fn unwrap(&mut self, envelope: &Value) -> Result<Value, String> {
        let seq = envelope["sequence"].as_u64().ok_or("Missing sequence")?;
        let body = envelope["body"].as_str().ok_or("Missing body")?;
        if envelope["obs_session"].as_str() != Some(&self.id)
            || seq != self.sequence + 1
            || !self.verify(
                "client-message",
                seq,
                body,
                envelope["signature"].as_str().unwrap_or(""),
            )
        {
            return Err("Invalid OBS session message".into());
        }
        let value = serde_json::from_str(body).map_err(|_| "Invalid OBS message JSON")?;
        self.sequence = seq;
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn proofs_bind_session_and_role() {
        let a = Session::new("key".into(), "fresh".into());
        assert!(a.verify_proof(&a.proof("client")));
        assert!(!a.verify_proof(&a.proof("host")));
        assert!(!Session::new("key".into(), "other".into()).verify_proof(&a.proof("client")));
        assert!(!a.verify_proof(&"é".repeat(32)));
    }
    #[test]
    fn signed_messages_reject_replay_tamper_and_other_session() {
        let mut a = Session::new("key".into(), "fresh".into());
        let body = "{\"type\":\"OBS_CANVAS_SIZE_UPDATE\"}";
        let signature: String = a
            .mac("client-message", 1, body)
            .finalize()
            .into_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let envelope =
            json!({"obs_session":"fresh","sequence":1,"body":body,"signature":signature});
        let mut tamper = envelope.clone();
        tamper["body"] = json!("{}");
        assert!(a.unwrap(&tamper).is_err());
        assert!(a.unwrap(&envelope).is_ok());
        assert!(a.unwrap(&envelope).is_err());
        assert!(Session::new("key".into(), "other".into())
            .unwrap(&envelope)
            .is_err());
    }
}
