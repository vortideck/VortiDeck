use std::collections::HashMap;

const OVERLAY_TOKEN_QUERY_KEY: &str = "overlayToken";

#[derive(Clone, Debug)]
pub struct ServerAccessTokens {
    pub overlay: String,
    pub metrics: String,
}

impl ServerAccessTokens {
    pub fn generate() -> Self {
        Self {
            overlay: uuid::Uuid::new_v4().to_string(),
            metrics: uuid::Uuid::new_v4().to_string(),
        }
    }
}

pub fn overlay_query_authorized(query: &HashMap<String, String>, expected: &str) -> bool {
    query
        .get(OVERLAY_TOKEN_QUERY_KEY)
        .is_some_and(|token| token == expected)
}

pub fn bearer_authorized(authorization: Option<&str>, expected: &str) -> bool {
    authorization
        .and_then(|value| value.split_once(' '))
        .is_some_and(|(scheme, token)| scheme.eq_ignore_ascii_case("bearer") && token == expected)
}

/// Remove capability query values before a URL is written to diagnostics.
pub fn redact_capability_url(url: &str) -> String {
    let Some(query_start) = url.find('?') else {
        return url.to_string();
    };
    let (base, query) = url.split_at(query_start + 1);
    let redacted = query
        .split('&')
        .map(|part| {
            let Some((key, value)) = part.split_once('=') else {
                return part.to_string();
            };
            if key.eq_ignore_ascii_case(OVERLAY_TOKEN_QUERY_KEY) && !value.is_empty() {
                format!("{}=<redacted>", key)
            } else {
                part.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("&");
    format!("{}{}", base, redacted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_scoped_and_distinct() {
        let tokens = ServerAccessTokens::generate();
        assert_ne!(tokens.overlay, tokens.metrics);
        assert!(!tokens.overlay.is_empty());
        assert!(!tokens.metrics.is_empty());
    }

    #[test]
    fn overlay_query_requires_exact_token() {
        let mut query = HashMap::new();
        assert!(!overlay_query_authorized(&query, "secret"));
        query.insert(OVERLAY_TOKEN_QUERY_KEY.into(), "wrong".into());
        assert!(!overlay_query_authorized(&query, "secret"));
        query.insert(OVERLAY_TOKEN_QUERY_KEY.into(), "secret".into());
        assert!(overlay_query_authorized(&query, "secret"));
    }

    #[test]
    fn metrics_bearer_is_independent() {
        assert!(bearer_authorized(
            Some("Bearer metrics-secret"),
            "metrics-secret"
        ));
        assert!(bearer_authorized(
            Some("bearer metrics-secret"),
            "metrics-secret"
        ));
        assert!(!bearer_authorized(
            Some("Bearer overlay-secret"),
            "metrics-secret"
        ));
        assert!(!bearer_authorized(Some("metrics-secret"), "metrics-secret"));
        assert!(!bearer_authorized(None, "metrics-secret"));
    }

    #[test]
    fn capability_urls_are_redacted_without_damaging_other_parameters() {
        assert_eq!(
            redact_capability_url(
                "http://127.0.0.1:9001/overlay.html?overlayToken=secret&mode=obs"
            ),
            "http://127.0.0.1:9001/overlay.html?overlayToken=<redacted>&mode=obs"
        );
        assert_eq!(
            redact_capability_url("http://127.0.0.1:9001/overlay.html"),
            "http://127.0.0.1:9001/overlay.html"
        );
    }
}
