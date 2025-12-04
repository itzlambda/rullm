//! OpenAI OAuth flow implementation.
//!
//! Supports ChatGPT Plus/Pro subscription authentication via OAuth discovery.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::{CallbackServer, PkceChallenge};
use crate::auth::Credential;

/// OpenAI OAuth configuration.
pub struct OpenAIOAuth {
    /// Issuer URL for discovery
    pub issuer_url: &'static str,
    /// Public client ID for Codex CLI (OpenAI consumer OAuth)
    pub client_id: &'static str,
    /// Authorization scopes requested
    pub scopes: &'static [&'static str],
    /// Callback port
    pub callback_port: u16,
}

impl Default for OpenAIOAuth {
    fn default() -> Self {
        Self {
            issuer_url: "https://auth.openai.com",
            client_id: "app_EMoamEEZ73f0CkXaXp7hrann",
            scopes: &["openid", "profile", "email", "offline_access"],
            callback_port: 1455,
        }
    }
}

/// OpenID Connect discovery document (subset used).
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenIdConfiguration {
    authorization_endpoint: String,
    token_endpoint: String,
    issuer: String,
}

/// Token response from OpenAI OAuth.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
    #[allow(dead_code)]
    token_type: String,
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

/// Token refresh request body.
#[derive(Debug, Serialize)]
struct RefreshRequest<'a> {
    grant_type: &'static str,
    client_id: &'a str,
    refresh_token: &'a str,
}

impl OpenAIOAuth {
    /// Create a new OpenAI OAuth handler with default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Discover OAuth endpoints from the authorization server.
    async fn discover(&self) -> Result<OpenIdConfiguration> {
        let discovery_url = format!("{}/.well-known/openid-configuration", self.issuer_url);

        let client = reqwest::Client::new();
        let response = client
            .get(&discovery_url)
            .header("Accept", "application/json")
            .send()
            .await
            .with_context(|| format!("Failed to fetch OAuth discovery from {}", discovery_url))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("OAuth discovery failed: {} - {}", status, body);
        }

        response
            .json()
            .await
            .context("Failed to parse OAuth discovery response")
    }

    /// Build the authorization URL for the OAuth flow.
    fn build_authorization_url(
        &self,
        authorization_endpoint: &str,
        pkce: &PkceChallenge,
        state: &str,
        redirect_uri: &str,
    ) -> String {
        let scope = self.scopes.join(" ");
        format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&code_challenge={}&code_challenge_method={}&state={}&id_token_add_organizations=true&codex_cli_simplified_flow=true&originator=codex_cli_rs",
            authorization_endpoint,
            urlencoding::encode(self.client_id),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(&scope),
            urlencoding::encode(&pkce.challenge),
            pkce.method(),
            urlencoding::encode(state)
        )
    }

    /// Start the OAuth flow and return the credential on success.
    ///
    /// This will:
    /// 1. Discover OAuth endpoints
    /// 2. Start a local callback server
    /// 3. Open the browser to the authorization URL
    /// 4. Wait for the callback with the authorization code
    /// 5. Exchange the code for tokens
    pub async fn login(&self) -> Result<Credential> {
        // Discover OAuth endpoints
        println!("Discovering OpenAI OAuth endpoints...");
        let _metadata = self.discover().await?;

        // Generate PKCE challenge
        let pkce = PkceChallenge::generate();

        // Generate state for CSRF protection
        let state = generate_state();

        // Start callback server
        let server =
            CallbackServer::new(self.callback_port).context("Failed to start callback server")?;

        // OpenAI's public client is registered with /auth/callback for localhost.
        let redirect_uri = server.redirect_uri_with_path("/auth/callback");

        // Build and open authorization URL
        // The public web consumer OAuth lives at /oauth/authorize (different from OIDC discovery value).
        let authorization_endpoint = format!("{}/oauth/authorize", self.issuer_url);
        // The working token endpoint for consumer OAuth lives on auth.openai.com, not the discovery host.
        let token_endpoint = format!("{}/oauth/token", self.issuer_url);
        let auth_url =
            self.build_authorization_url(&authorization_endpoint, &pkce, &state, &redirect_uri);

        println!("Opening browser for OpenAI authentication...");
        webbrowser::open(&auth_url).context("Failed to open browser")?;

        println!("Waiting for authentication (timeout: 5 minutes)...");

        // Wait for callback
        let callback = server
            .wait_for_callback(Duration::from_secs(300))
            .context("Failed to receive OAuth callback")?;

        // Verify state
        if callback.state.as_deref() != Some(&state) {
            anyhow::bail!("State mismatch in OAuth callback (possible CSRF attack)");
        }

        // Exchange code for tokens
        let credential = self
            .exchange_code(
                &token_endpoint,
                &callback.code,
                &pkce.verifier,
                &redirect_uri,
            )
            .await?;

        Ok(credential)
    }

    /// Exchange authorization code for tokens.
    async fn exchange_code(
        &self,
        token_endpoint: &str,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
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
            .post(token_endpoint)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(serde_urlencoded::to_string(&request_body)?)
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

        // OpenAI might not always return a refresh token
        let refresh_token = token_response.refresh_token.unwrap_or_default();

        Ok(Credential::oauth(
            token_response.access_token,
            refresh_token,
            expires_at,
        ))
    }

    /// Refresh an expired OAuth token.
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<Credential> {
        // Use the known consumer token endpoint (discovery token endpoint points to auth0.openai.com and fails).
        let token_endpoint = format!("{}/oauth/token", self.issuer_url);

        let request_body = RefreshRequest {
            grant_type: "refresh_token",
            client_id: self.client_id,
            refresh_token,
        };

        let client = reqwest::Client::new();
        let response = client
            .post(&token_endpoint)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(serde_urlencoded::to_string(&request_body)?)
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

        let new_refresh_token = token_response
            .refresh_token
            .unwrap_or_else(|| refresh_token.to_string());

        Ok(Credential::oauth(
            token_response.access_token,
            new_refresh_token,
            expires_at,
        ))
    }
}

/// Generate a random state string for CSRF protection.
fn generate_state() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let oauth = OpenAIOAuth::new();
        assert_eq!(oauth.callback_port, 1455);
        assert_eq!(oauth.issuer_url, "https://auth.openai.com");
        assert_eq!(oauth.client_id, "app_EMoamEEZ73f0CkXaXp7hrann");
        assert_eq!(
            oauth.scopes,
            ["openid", "profile", "email", "offline_access"]
        );
    }

    #[test]
    fn test_build_authorization_url() {
        let oauth = OpenAIOAuth::new();
        let pkce = PkceChallenge::generate();
        let state = "test-state";
        let redirect_uri = "http://localhost:1455/auth/callback";

        let url = oauth.build_authorization_url(
            "https://auth.openai.com/oauth/authorize",
            &pkce,
            state,
            redirect_uri,
        );

        assert!(url.starts_with("https://auth.openai.com/oauth/authorize"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=app_EMoamEEZ73f0CkXaXp7hrann"));
        assert!(url.contains("scope=openid%20profile%20email%20offline_access"));
        assert!(url.contains("redirect_uri="));
        assert!(url.contains("code_challenge="));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("state=test-state"));
        assert!(url.contains("codex_cli_simplified_flow=true"));
        assert!(url.contains("originator=codex_cli_rs"));
    }
}
