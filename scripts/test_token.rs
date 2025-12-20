//! Quick test of token exchange

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct TokenRequest<'a> {
    grant_type: &'static str,
    client_id: &'a str,
    code: &'a str,
    redirect_uri: &'a str,
    code_verifier: &'a str,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
    token_type: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Read from auth_code.json
    let cache_path = dirs::home_dir()
        .unwrap()
        .join(".cache/anthropic-oauth-test/auth_code.json");
    
    let cached: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&cache_path)?)?;
    
    let auth_code = cached["auth_code"].as_str().unwrap();
    let code_verifier = cached["code_verifier"].as_str().unwrap();
    
    println!("Auth code: {}", auth_code);
    println!("Code verifier: {}", code_verifier);
    
    let request_body = TokenRequest {
        grant_type: "authorization_code",
        client_id: "9d1c250a-e61b-44d9-88ed-5944d1962f5e",
        code: auth_code,
        redirect_uri: "http://localhost:8765/callback",
        code_verifier,
    };
    
    println!("Request JSON: {}", serde_json::to_string_pretty(&request_body)?);
    
    let client = reqwest::Client::new();
    let response = client
        .post("https://console.anthropic.com/v1/oauth/token")
        .json(&request_body)
        .send()
        .await?;
    
    println!("Response status: {}", response.status());
    println!("Response body: {}", response.text().await?);
    
    Ok(())
}
