//! Authentication credential management for rullm.
//!
//! Supports multiple authentication methods per provider:
//! - OAuth (for Claude Max/Pro, ChatGPT Plus/Pro subscriptions)
//! - API keys (traditional method)

use crate::provider::Provider;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// File name for auth credentials
pub const AUTH_CONFIG_FILE: &str = "auth.toml";

/// Buffer time (in ms) before token expiration to trigger refresh
const TOKEN_EXPIRY_BUFFER_MS: u64 = 5 * 60 * 1000; // 5 minutes

/// A credential for authenticating with an LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Credential {
    /// OAuth credential with access/refresh tokens
    OAuth {
        access_token: String,
        refresh_token: String,
        /// Unix timestamp in milliseconds when the access token expires
        expires_at: u64,
    },
    /// API key credential
    Api { api_key: String },
}

impl Credential {
    /// Create a new OAuth credential
    pub fn oauth(access_token: String, refresh_token: String, expires_at: u64) -> Self {
        Self::OAuth {
            access_token,
            refresh_token,
            expires_at,
        }
    }

    /// Create a new API key credential
    pub fn api(api_key: String) -> Self {
        Self::Api { api_key }
    }

    /// Get the access token or API key for use in requests
    pub fn get_token(&self) -> &str {
        match self {
            Self::OAuth { access_token, .. } => access_token,
            Self::Api { api_key } => api_key,
        }
    }

    /// Check if an OAuth token is expired or about to expire
    pub fn is_expired(&self) -> bool {
        match self {
            Self::OAuth { expires_at, .. } => {
                let now_ms = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                *expires_at <= now_ms + TOKEN_EXPIRY_BUFFER_MS
            }
            Self::Api { .. } => false, // API keys don't expire
        }
    }

    /// Get the refresh token if this is an OAuth credential
    pub fn refresh_token(&self) -> Option<&str> {
        match self {
            Self::OAuth { refresh_token, .. } => Some(refresh_token),
            Self::Api { .. } => None,
        }
    }

    /// Return a display string for the credential type
    pub fn type_display(&self) -> &'static str {
        match self {
            Self::OAuth { .. } => "oauth",
            Self::Api { .. } => "api",
        }
    }
}

/// Authentication configuration containing credentials for all providers.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AuthConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anthropic: Option<Credential>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub openai: Option<Credential>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub groq: Option<Credential>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub openrouter: Option<Credential>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub google: Option<Credential>,
}

impl AuthConfig {
    /// Load auth config from the default location
    pub fn load(config_base_path: &Path) -> Result<Self> {
        let path = config_base_path.join(AUTH_CONFIG_FILE);
        Self::load_from_file(&path)
    }

    /// Load auth config from a specific file path
    pub fn load_from_file(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read auth config from {}", path.display()))?;

        if content.trim().is_empty() {
            return Ok(Self::default());
        }

        toml::from_str(&content)
            .with_context(|| format!("Failed to parse auth config from {}", path.display()))
    }

    /// Save auth config to the default location
    pub fn save(&self, config_base_path: &Path) -> Result<()> {
        let path = config_base_path.join(AUTH_CONFIG_FILE);
        self.save_to_file(&path)
    }

    /// Save auth config to a specific file path
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }

        let content =
            toml::to_string_pretty(self).with_context(|| "Failed to serialize auth config")?;

        fs::write(path, &content)
            .with_context(|| format!("Failed to write auth config to {}", path.display()))?;

        // Set file permissions to 0600 (owner read/write only) on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            fs::set_permissions(path, perms)
                .with_context(|| format!("Failed to set permissions on {}", path.display()))?;
        }

        Ok(())
    }

    /// Get credential for a provider from config
    pub fn get(&self, provider: &Provider) -> Option<&Credential> {
        match provider {
            Provider::Anthropic => self.anthropic.as_ref(),
            Provider::OpenAI => self.openai.as_ref(),
            Provider::Groq => self.groq.as_ref(),
            Provider::OpenRouter => self.openrouter.as_ref(),
            Provider::Google => self.google.as_ref(),
        }
    }

    /// Get mutable credential for a provider
    pub fn get_mut(&mut self, provider: &Provider) -> &mut Option<Credential> {
        match provider {
            Provider::Anthropic => &mut self.anthropic,
            Provider::OpenAI => &mut self.openai,
            Provider::Groq => &mut self.groq,
            Provider::OpenRouter => &mut self.openrouter,
            Provider::Google => &mut self.google,
        }
    }

    /// Set credential for a provider
    pub fn set(&mut self, provider: &Provider, credential: Credential) {
        *self.get_mut(provider) = Some(credential);
    }

    /// Remove credential for a provider
    pub fn remove(&mut self, provider: &Provider) {
        *self.get_mut(provider) = None;
    }
}

/// Get the auth config file path
pub fn auth_config_path(config_base_path: &Path) -> PathBuf {
    config_base_path.join(AUTH_CONFIG_FILE)
}

/// Source of a credential (file or environment variable)
#[derive(Debug, Clone, PartialEq)]
pub enum CredentialSource {
    File,
    Environment(String),
}

/// Result of credential lookup including the source
#[derive(Debug)]
pub struct CredentialInfo {
    pub credential: Credential,
    pub source: CredentialSource,
}

/// Get credential for a provider, checking file first, then environment variable.
///
/// File credentials take precedence over environment variables.
pub fn get_credential(provider: &Provider, auth_config: &AuthConfig) -> Option<CredentialInfo> {
    // Check file credentials first (higher precedence)
    if let Some(cred) = auth_config.get(provider) {
        return Some(CredentialInfo {
            credential: cred.clone(),
            source: CredentialSource::File,
        });
    }

    // Fall back to environment variable
    let env_key = provider.env_key();
    if let Ok(api_key) = std::env::var(env_key) {
        return Some(CredentialInfo {
            credential: Credential::api(api_key),
            source: CredentialSource::Environment(env_key.to_string()),
        });
    }

    None
}

/// Get token for a provider, automatically refreshing OAuth tokens if expired.
///
/// This is the preferred method for getting tokens as it handles expiration.
/// If the token is refreshed, the new credential is saved to the config file.
pub async fn get_or_refresh_token(
    provider: &Provider,
    auth_config: &mut AuthConfig,
    config_base_path: &Path,
) -> Result<String> {
    // Get credential info
    let info = get_credential(provider, auth_config)
        .ok_or_else(|| anyhow::anyhow!("No credential found for {}", provider))?;

    // If from environment, just return the token (can't refresh env vars)
    if matches!(info.source, CredentialSource::Environment(_)) {
        return Ok(info.credential.get_token().to_string());
    }

    // Check if OAuth token is expired
    if info.credential.is_expired() {
        if let Some(refresh_tok) = info.credential.refresh_token() {
            // Attempt to refresh
            eprintln!("OAuth token expired, refreshing...");
            match refresh_oauth_token(provider, refresh_tok).await {
                Ok(new_credential) => {
                    let token = new_credential.get_token().to_string();
                    auth_config.set(provider, new_credential);
                    auth_config.save(config_base_path)?;
                    eprintln!("Token refreshed successfully.");
                    return Ok(token);
                }
                Err(e) => {
                    // Refresh failed - user needs to re-authenticate
                    return Err(anyhow::anyhow!(
                        "OAuth token expired and refresh failed: {}. Please run 'rullm auth login {}'",
                        e,
                        provider
                    ));
                }
            }
        }
    }

    Ok(info.credential.get_token().to_string())
}

/// Refresh an OAuth token for a specific provider.
async fn refresh_oauth_token(provider: &Provider, refresh_token: &str) -> Result<Credential> {
    use crate::oauth::{anthropic::AnthropicOAuth, openai::OpenAIOAuth};

    match provider {
        Provider::Anthropic => {
            let oauth = AnthropicOAuth::new();
            oauth.refresh_token(refresh_token).await
        }
        Provider::OpenAI => {
            let oauth = OpenAIOAuth::new();
            oauth.refresh_token(refresh_token).await
        }
        _ => Err(anyhow::anyhow!(
            "Provider {} does not support OAuth token refresh",
            provider
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_credential_oauth() {
        let cred = Credential::oauth(
            "access".to_string(),
            "refresh".to_string(),
            u64::MAX, // Far future
        );
        assert!(matches!(cred, Credential::OAuth { .. }));
        assert_eq!(cred.get_token(), "access");
        assert_eq!(cred.refresh_token(), Some("refresh"));
        assert!(!cred.is_expired());
    }

    #[test]
    fn test_credential_api() {
        let cred = Credential::api("sk-test-key".to_string());
        assert!(matches!(cred, Credential::Api { .. }));
        assert_eq!(cred.get_token(), "sk-test-key");
        assert_eq!(cred.refresh_token(), None);
        assert!(!cred.is_expired());
    }

    #[test]
    fn test_credential_expired() {
        let cred = Credential::oauth(
            "access".to_string(),
            "refresh".to_string(),
            0, // Long expired
        );
        assert!(cred.is_expired());
    }

    #[test]
    fn test_auth_config_serialization() {
        let mut config = AuthConfig::default();
        config.anthropic = Some(Credential::oauth(
            "sk-ant-oat01-test".to_string(),
            "sk-ant-ort01-test".to_string(),
            1764813330304,
        ));
        config.openai = Some(Credential::api("sk-proj-test".to_string()));

        let toml_str = toml::to_string_pretty(&config).unwrap();

        // Verify it can be deserialized back
        let parsed: AuthConfig = toml::from_str(&toml_str).unwrap();
        assert!(parsed.anthropic.is_some());
        assert!(parsed.openai.is_some());
        assert!(parsed.groq.is_none());
    }

    #[test]
    fn test_auth_config_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path();

        let mut config = AuthConfig::default();
        config.groq = Some(Credential::api("test-groq-key".to_string()));

        config.save(config_path).unwrap();

        let loaded = AuthConfig::load(config_path).unwrap();
        assert_eq!(
            loaded.groq.as_ref().map(|c| c.get_token()),
            Some("test-groq-key")
        );
    }

    #[test]
    fn test_get_credential_file_precedence() {
        let mut config = AuthConfig::default();
        config.anthropic = Some(Credential::api("file-key".to_string()));

        // File credential should be returned
        let info = get_credential(&Provider::Anthropic, &config).unwrap();
        assert_eq!(info.source, CredentialSource::File);
        assert_eq!(info.credential.get_token(), "file-key");
    }
}
