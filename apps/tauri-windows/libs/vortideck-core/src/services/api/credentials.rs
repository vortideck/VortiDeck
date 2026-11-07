//! Opaque, immutable credential references in owner-authored HTTP headers.
//! Secret resolution occurs at the HTTP boundary, after admission snapshots.

use crate::services::auth::token_storage::TokenStorage;
use std::collections::HashMap;

pub const CREDENTIAL_PREFIX: &str = "vortideck-credential://";

pub fn is_credential_reference(value: &str) -> bool {
    value
        .strip_prefix(CREDENTIAL_PREFIX)
        .is_some_and(|identity| uuid::Uuid::parse_str(identity).is_ok())
}

pub fn requires_protection(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "authorization"
            | "proxy-authorization"
            | "cookie"
            | "x-api-key"
            | "api-key"
            | "x-auth-token"
    )
}

pub struct ProtectedHeaders {
    pub headers: HashMap<String, String>,
    created: Vec<String>,
}

impl ProtectedHeaders {
    /// Compensate only credentials created by this failed save. Existing shared
    /// references and historical generations must remain valid.
    pub fn rollback(self) {
        for identity in self.created {
            let _ = TokenStorage::remove_action_secret(&identity);
        }
    }
}

pub fn protect_headers(
    headers: HashMap<String, String>,
    additional: &[String],
) -> Result<ProtectedHeaders, String> {
    protect_headers_with(
        headers,
        additional,
        |value| TokenStorage::store_action_secret(value).map_err(|_| ()),
        |identity| {
            let _ = TokenStorage::remove_action_secret(identity);
        },
    )
}

fn protect_headers_with(
    mut headers: HashMap<String, String>,
    additional: &[String],
    mut store: impl FnMut(&str) -> Result<String, ()>,
    mut remove: impl FnMut(&str),
) -> Result<ProtectedHeaders, String> {
    let mut created = Vec::new();
    for (name, value) in &mut headers {
        if is_credential_reference(value) {
            continue;
        }
        if requires_protection(name)
            || additional
                .iter()
                .any(|secret| secret.eq_ignore_ascii_case(name))
        {
            match store(value) {
                Ok(identity) => {
                    *value = format!("{}{}", CREDENTIAL_PREFIX, identity);
                    created.push(identity);
                }
                Err(_) => {
                    for identity in created {
                        remove(&identity);
                    }
                    return Err(
                        "HTTP credentials could not be stored securely; request was not saved"
                            .into(),
                    );
                }
            }
        }
    }
    Ok(ProtectedHeaders { headers, created })
}

pub async fn resolve_headers(
    headers: HashMap<String, String>,
) -> Result<HashMap<String, String>, String> {
    tokio::task::spawn_blocking(move || {
        resolve_headers_with(headers, |identity| {
            TokenStorage::load_action_secret(identity)
                .map_err(|_| "HTTP credential is missing or inaccessible".into())
        })
    })
    .await
    .map_err(|_| "HTTP credential resolution was interrupted".to_string())?
}

pub async fn resolve_authentication(
    auth: super::http_request::AuthType,
) -> Result<super::http_request::AuthType, String> {
    use super::http_request::AuthType;
    tokio::task::spawn_blocking(move || {
        fn resolve(value: String) -> Result<String, String> {
            if let Some(identity) = value.strip_prefix(CREDENTIAL_PREFIX) {
                uuid::Uuid::parse_str(identity).map_err(|_| "Invalid HTTP credential reference")?;
                TokenStorage::load_action_secret(identity)
                    .map_err(|_| "HTTP credential is missing or inaccessible".into())
            } else {
                Ok(value)
            }
        }
        Ok(match auth {
            AuthType::None => AuthType::None,
            AuthType::BearerToken { token } => AuthType::BearerToken {
                token: resolve(token)?,
            },
            AuthType::BasicAuth { username, password } => AuthType::BasicAuth {
                username: resolve(username)?,
                password: resolve(password)?,
            },
            AuthType::ApiKey {
                header_name,
                api_key,
            } => AuthType::ApiKey {
                header_name,
                api_key: resolve(api_key)?,
            },
            AuthType::CustomHeaders { headers } => AuthType::CustomHeaders {
                headers: resolve_headers_with(headers, |identity| {
                    TokenStorage::load_action_secret(identity)
                        .map_err(|_| "HTTP credential is missing or inaccessible".into())
                })?,
            },
        })
    })
    .await
    .map_err(|_| "HTTP credential resolution was interrupted".to_string())?
}

fn resolve_headers_with(
    mut headers: HashMap<String, String>,
    mut load: impl FnMut(&str) -> Result<String, String>,
) -> Result<HashMap<String, String>, String> {
    for value in headers.values_mut() {
        if let Some(identity) = value.strip_prefix(CREDENTIAL_PREFIX) {
            uuid::Uuid::parse_str(identity).map_err(|_| "Invalid HTTP credential reference")?;
            *value = load(identity)?;
        }
    }
    Ok(headers)
}

pub fn redact_secrets(value: &mut serde_json::Value, secrets: &[String]) {
    match value {
        serde_json::Value::String(text) => {
            for secret in secrets.iter().filter(|secret| !secret.is_empty()) {
                *text = text.replace(secret, "[redacted]");
            }
        }
        serde_json::Value::Array(items) => items
            .iter_mut()
            .for_each(|item| redact_secrets(item, secrets)),
        serde_json::Value::Object(fields) => fields
            .values_mut()
            .for_each(|item| redact_secrets(item, secrets)),
        serde_json::Value::Number(number)
            if secrets.iter().any(|secret| *secret == number.to_string()) =>
        {
            *value = serde_json::Value::String("[redacted]".into());
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_protection_compensates_only_new_generations() {
        let previous = uuid::Uuid::new_v4().to_string();
        let newly_created = uuid::Uuid::new_v4().to_string();
        let headers = HashMap::from([
            (
                "Authorization".into(),
                format!("{CREDENTIAL_PREFIX}{previous}"),
            ),
            ("X-Api-Key".into(), "one-new-secret".into()),
            ("Cookie".into(), "another-new-secret".into()),
        ]);
        let mut calls = 0;
        let mut removed = Vec::new();
        let result = protect_headers_with(
            headers.clone(),
            &[],
            |_| {
                calls += 1;
                if calls == 1 {
                    Ok(newly_created.clone())
                } else {
                    Err(())
                }
            },
            |identity| removed.push(identity.to_string()),
        );
        assert!(result.is_err());
        assert_eq!(removed, [newly_created]);
        assert!(headers["Authorization"].ends_with(&previous));
        assert_eq!(headers["X-Api-Key"], "one-new-secret");
    }
    #[test]
    fn credential_values_are_loaded_only_at_the_provider_boundary() {
        let identity = uuid::Uuid::new_v4().to_string();
        let headers = HashMap::from([
            (
                "Authorization".into(),
                format!("{}{}", CREDENTIAL_PREFIX, identity),
            ),
            ("Content-Type".into(), "application/json".into()),
        ]);
        assert!(!serde_json::to_string(&headers)
            .unwrap()
            .contains("seeded-secret"));
        let resolved = resolve_headers_with(headers.clone(), |requested| {
            assert_eq!(requested, identity);
            Ok("Bearer seeded-secret".into())
        })
        .unwrap();
        assert_eq!(resolved["Authorization"], "Bearer seeded-secret");
        assert!(resolve_headers_with(headers, |_| Err("missing credential".into())).is_err());
    }
}
