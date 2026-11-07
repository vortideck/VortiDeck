use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordAccessToken {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
    pub discriminator: String,
    pub avatar: Option<String>,
    pub bot: Option<bool>,
    pub system: Option<bool>,
    pub mfa_enabled: Option<bool>,
    pub verified: Option<bool>,
    pub email: Option<String>,
}

impl DiscordOAuthConfig {
    pub fn new(client_id: String, client_secret: String, redirect_uri: String) -> Self {
        Self {
            client_id,
            client_secret,
            redirect_uri,
            scopes: vec![
                "identify".to_string(),
                "guilds".to_string(),
                "webhook.incoming".to_string(),
            ],
        }
    }

    /// Generate Discord OAuth2 authorization URL
    pub fn get_authorization_url(&self, state: Option<&str>) -> String {
        let scopes_joined = self.scopes.join(" ");
        let mut params = vec![
            ("client_id", self.client_id.as_str()),
            ("redirect_uri", self.redirect_uri.as_str()),
            ("response_type", "code"),
            ("scope", &scopes_joined),
        ];

        if let Some(state_value) = state {
            params.push(("state", state_value));
        }

        let query_string = params
            .iter()
            .map(|(key, value)| format!("{}={}", key, urlencoding::encode(value)))
            .collect::<Vec<_>>()
            .join("&");

        format!("https://discord.com/oauth2/authorize?{}", query_string)
    }

    /// Exchange authorization code for access token
    pub async fn exchange_code_for_token(
        &self,
        code: &str,
    ) -> Result<DiscordAccessToken, Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::new();

        let grant_type = "authorization_code".to_string();
        let code_string = code.to_string();

        let mut params = HashMap::new();
        params.insert("client_id", &self.client_id);
        params.insert("client_secret", &self.client_secret);
        params.insert("grant_type", &grant_type);
        params.insert("code", &code_string);
        params.insert("redirect_uri", &self.redirect_uri);

        let response = client
            .post("https://discord.com/api/oauth2/token")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?;

        if response.status().is_success() {
            let token: DiscordAccessToken = response.json().await?;
            Ok(token)
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(format!("Failed to exchange code for token: {}", error_text).into())
        }
    }

    /// Refresh access token using refresh token
    pub async fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<DiscordAccessToken, Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::new();

        let grant_type = "refresh_token".to_string();
        let refresh_token_string = refresh_token.to_string();

        let mut params = HashMap::new();
        params.insert("client_id", &self.client_id);
        params.insert("client_secret", &self.client_secret);
        params.insert("grant_type", &grant_type);
        params.insert("refresh_token", &refresh_token_string);

        let response = client
            .post("https://discord.com/api/oauth2/token")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&params)
            .send()
            .await?;

        if response.status().is_success() {
            let token: DiscordAccessToken = response.json().await?;
            Ok(token)
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(format!("Failed to refresh token: {}", error_text).into())
        }
    }

    /// Get user information using access token
    pub async fn get_user_info(
        &self,
        access_token: &str,
    ) -> Result<DiscordUser, Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::new();

        let response = client
            .get("https://discord.com/api/users/@me")
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        if response.status().is_success() {
            let user: DiscordUser = response.json().await?;
            Ok(user)
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(format!("Failed to get user info: {}", error_text).into())
        }
    }
}
