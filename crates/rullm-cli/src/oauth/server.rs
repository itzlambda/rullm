//! Local HTTP server for OAuth callback handling.
//!
//! Starts a temporary server to receive the OAuth authorization code.

use anyhow::{Context, Result, anyhow};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;

/// Result of waiting for an OAuth callback.
#[derive(Debug)]
pub struct CallbackResult {
    /// The authorization code received
    pub code: String,
    /// The state parameter (for CSRF verification)
    pub state: Option<String>,
}

/// Local callback server for OAuth flows.
pub struct CallbackServer {
    listener: TcpListener,
    port: u16,
}

impl CallbackServer {
    /// Create a new callback server on the specified port.
    pub fn new(port: u16) -> Result<Self> {
        let addr = format!("127.0.0.1:{port}");
        let listener =
            TcpListener::bind(&addr).with_context(|| format!("Failed to bind to {addr}"))?;

        // Get the actual port if 0 was specified
        let actual_port = listener.local_addr()?.port();

        Ok(Self {
            listener,
            port: actual_port,
        })
    }

    /// Get the redirect URI for this callback server.
    pub fn redirect_uri(&self) -> String {
        format!("http://localhost:{}/callback", self.port)
    }

    /// Build a redirect URI using a custom path (must start with '/').
    pub fn redirect_uri_with_path(&self, path: &str) -> String {
        let normalized = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{}", path)
        };
        format!("http://localhost:{}{}", self.port, normalized)
    }

    /// Wait for the OAuth callback and extract the authorization code.
    ///
    /// This blocks until a request is received or the timeout is reached.
    pub fn wait_for_callback(&self, timeout: Duration) -> Result<CallbackResult> {
        // Set non-blocking mode on the listener and poll with timeout
        self.listener
            .set_nonblocking(true)
            .context("Failed to set non-blocking mode")?;

        let start = std::time::Instant::now();
        let poll_interval = Duration::from_millis(100);

        loop {
            match self.listener.accept() {
                Ok((mut stream, _addr)) => {
                    // Set the stream back to blocking for read/write
                    stream.set_nonblocking(false).ok();
                    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();

                    // Read the HTTP request
                    let mut buffer = [0u8; 4096];
                    let n = stream
                        .read(&mut buffer)
                        .context("Failed to read from connection")?;

                    let request = String::from_utf8_lossy(&buffer[..n]);

                    // Parse the request to extract code and state
                    let result = Self::parse_callback_request(&request)?;

                    // Send a success response
                    let response_body = r#"<!DOCTYPE html>
<html>
<head><title>Authentication Successful</title></head>
<body style="font-family: system-ui; display: flex; justify-content: center; align-items: center; height: 100vh; margin: 0;">
<div style="text-align: center;">
<h1 style="color: #10b981;">Authentication successful!</h1>
<p>You can close this window and return to the terminal.</p>
</div>
</body>
</html>"#;

                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        response_body.len(),
                        response_body
                    );

                    stream
                        .write_all(response.as_bytes())
                        .context("Failed to send response")?;

                    stream.flush().ok();

                    return Ok(result);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No connection yet, check timeout
                    if start.elapsed() >= timeout {
                        return Err(anyhow!("Timeout waiting for OAuth callback"));
                    }
                    std::thread::sleep(poll_interval);
                }
                Err(e) => {
                    return Err(e).context("Failed to accept connection");
                }
            }
        }
    }

    /// Parse the callback request to extract code and state parameters.
    fn parse_callback_request(request: &str) -> Result<CallbackResult> {
        // Extract the request line (GET /callback?code=xxx&state=yyy HTTP/1.1)
        let first_line = request
            .lines()
            .next()
            .ok_or_else(|| anyhow!("Empty request"))?;

        // Extract the path with query string
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(anyhow!("Invalid HTTP request line"));
        }

        let path = parts[1];

        // Check for error response
        if let Some(error) = Self::extract_query_param(path, "error") {
            let description = Self::extract_query_param(path, "error_description")
                .unwrap_or_else(|| "Unknown error".to_string());
            return Err(anyhow!("OAuth error: {} - {}", error, description));
        }

        // Extract the code
        let code = Self::extract_query_param(path, "code")
            .ok_or_else(|| anyhow!("No authorization code in callback"))?;

        // Extract state (optional)
        let state = Self::extract_query_param(path, "state");

        Ok(CallbackResult { code, state })
    }

    /// Extract a query parameter value from a URL path.
    fn extract_query_param(path: &str, param: &str) -> Option<String> {
        let query = path.split('?').nth(1)?;
        for pair in query.split('&') {
            let mut kv = pair.splitn(2, '=');
            let key = kv.next()?;
            let value = kv.next()?;
            if key == param {
                // URL decode the value
                return Some(urlencoding::decode(value).ok()?.into_owned());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_callback_success() {
        let request = "GET /callback?code=abc123&state=xyz789 HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let result = CallbackServer::parse_callback_request(request).unwrap();
        assert_eq!(result.code, "abc123");
        assert_eq!(result.state, Some("xyz789".to_string()));
    }

    #[test]
    fn test_parse_callback_no_state() {
        let request = "GET /callback?code=abc123 HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let result = CallbackServer::parse_callback_request(request).unwrap();
        assert_eq!(result.code, "abc123");
        assert_eq!(result.state, None);
    }

    #[test]
    fn test_parse_callback_error() {
        let request = "GET /callback?error=access_denied&error_description=User%20denied%20access HTTP/1.1\r\n";
        let result = CallbackServer::parse_callback_request(request);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("access_denied"));
    }

    #[test]
    fn test_parse_callback_no_code() {
        let request = "GET /callback?state=xyz HTTP/1.1\r\n";
        let result = CallbackServer::parse_callback_request(request);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_query_param() {
        let path = "/callback?code=abc&state=def&other=ghi";
        assert_eq!(
            CallbackServer::extract_query_param(path, "code"),
            Some("abc".to_string())
        );
        assert_eq!(
            CallbackServer::extract_query_param(path, "state"),
            Some("def".to_string())
        );
        assert_eq!(CallbackServer::extract_query_param(path, "missing"), None);
    }

    #[test]
    fn test_redirect_uri() {
        // Note: This test requires an available port
        if let Ok(server) = CallbackServer::new(0) {
            let uri = server.redirect_uri();
            assert!(uri.starts_with("http://localhost:"));
            assert!(uri.ends_with("/callback"));
        }
    }
}
