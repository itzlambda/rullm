//! Anthropic OAuth flow implementation.
//!
//! Supports Claude Max/Pro subscription authentication.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::PkceChallenge;
use super::server::CallbackServer;
use crate::auth::Credential;

/// Anthropic OAuth configuration.
pub struct AnthropicOAuth {
    /// Authorization URL
    pub authorization_url: &'static str,
    /// Token URL
    pub token_url: &'static str,
    /// Client ID (Claude Code's public ID)
    pub client_id: &'static str,
    /// Required scopes
    pub scopes: &'static [&'static str],
    /// Local callback port
    pub callback_port: u16,
}

impl Default for AnthropicOAuth {
    fn default() -> Self {
        Self {
            authorization_url: "https://claude.ai/oauth/authorize",
            // The token endpoint lives on the console domain (not the public API)
            // and requires the `/v1` prefix; posting to the API host returns 404.
            token_url: "https://console.anthropic.com/v1/oauth/token",
            client_id: "9d1c250a-e61b-44d9-88ed-5944d1962f5e",
            scopes: &["org:create_api_key", "user:profile", "user:inference"],
            callback_port: 8765,
        }
    }
}

/// Token response from Anthropic OAuth.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
    #[allow(dead_code)]
    token_type: String,
}

/// Token refresh request body.
#[derive(Debug, Serialize)]
struct RefreshRequest<'a> {
    grant_type: &'static str,
    client_id: &'a str,
    refresh_token: &'a str,
}

/// Token exchange request body.
#[derive(Debug, Serialize)]
struct TokenRequest<'a> {
    grant_type: &'static str,
    client_id: &'a str,
    code: &'a str,
    redirect_uri: &'a str,
    code_verifier: &'a str,
}

impl AnthropicOAuth {
    /// Create a new Anthropic OAuth handler with default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build the authorization URL for the OAuth flow.
    fn build_authorization_url(&self, redirect_uri: &str, pkce: &PkceChallenge) -> String {
        let scope = self.scopes.join(" ");

        format!(
            "{}?code=true&client_id={}&response_type=code&redirect_uri={}&scope={}&code_challenge={}&code_challenge_method={}&state={}",
            self.authorization_url,
            urlencoding::encode(self.client_id),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(&scope),
            urlencoding::encode(&pkce.challenge),
            pkce.method(),
            urlencoding::encode(&pkce.verifier) // Use verifier as state (like OpenCode)
        )
    }

    /// Start the OAuth flow and return the credential on success.
    ///
    /// This opens a browser for the user to authorize, then captures
    /// the callback on a local server.
    pub async fn login(&self) -> Result<Credential> {
        // Start local callback server
        let server =
            CallbackServer::new(self.callback_port).context("Failed to start callback server")?;

        let redirect_uri = server.redirect_uri();

        // Generate PKCE challenge
        let pkce = PkceChallenge::generate();

        // Build authorization URL
        let auth_url = self.build_authorization_url(&redirect_uri, &pkce);

        println!("Opening browser for Anthropic authentication...");

        // Try to open browser, but don't fail if it doesn't work
        if let Err(e) = webbrowser::open(&auth_url) {
            println!("Could not open browser automatically: {}", e);
            println!("Please open this URL manually:");
            println!("{}", auth_url);
        }

        // Wait for the callback
        println!("Waiting for authorization callback...");
        let callback = server
            .wait_for_callback(Duration::from_secs(300))
            .context("Failed to receive OAuth callback")?;

        // Verify state matches (we use verifier as state)
        if let Some(state) = &callback.state {
            if state != &pkce.verifier {
                anyhow::bail!("State mismatch in OAuth callback");
            }
        }

        // Exchange code for tokens
        let credential = self
            .exchange_code(&callback.code, &redirect_uri, &pkce.verifier)
            .await?;

        println!("Authentication successful!");
        Ok(credential)
    }

    /// Exchange authorization code for tokens.
    async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
        code_verifier: &str,
    ) -> Result<Credential> {
        let request_body = TokenRequest {
            grant_type: "authorization_code",
            client_id: self.client_id,
            code,
            redirect_uri,
            code_verifier,
        };

        let client = reqwest::Client::new();
        let response = client
            .post(self.token_url)
            // Anthropic expects JSON payloads for the token exchange.
            .json(&request_body)
            .send()
            .await
            .context("Failed to send token request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Token exchange failed: {} - {}", status, body);
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .context("Failed to parse token response")?;

        // Calculate expiration timestamp
        let expires_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64 + token_response.expires_in * 1000)
            .unwrap_or(0);

        Ok(Credential::oauth(
            token_response.access_token,
            token_response.refresh_token,
            expires_at,
        ))
    }

    /// Refresh an expired OAuth token.
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<Credential> {
        let request_body = RefreshRequest {
            grant_type: "refresh_token",
            client_id: self.client_id,
            refresh_token,
        };

        let client = reqwest::Client::new();
        let response = client
            .post(self.token_url)
            .json(&request_body)
            .send()
            .await
            .context("Failed to send refresh request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Token refresh failed: {} - {}", status, body);
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .context("Failed to parse refresh response")?;

        let expires_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64 + token_response.expires_in * 1000)
            .unwrap_or(0);

        Ok(Credential::oauth(
            token_response.access_token,
            token_response.refresh_token,
            expires_at,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_authorization_url() {
        let oauth = AnthropicOAuth::new();
        let pkce = PkceChallenge::generate();

        let url = oauth.build_authorization_url("http://localhost:8765/callback", &pkce);

        assert!(url.starts_with("https://claude.ai/oauth/authorize"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id="));
        assert!(url.contains("redirect_uri="));
        assert!(url.contains("code_challenge="));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("state="));
    }

    #[test]
    fn test_default_config() {
        let oauth = AnthropicOAuth::new();
        assert!(oauth.scopes.contains(&"user:inference"));
    }
}
