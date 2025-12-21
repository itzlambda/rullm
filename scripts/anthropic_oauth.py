#!/usr/bin/env python3
"""
Simple Anthropic OAuth flow test script.
Implements PKCE OAuth to get access token and test API calls.
Caches auth code and tokens to avoid repeated logins.
"""

import argparse
import base64
import hashlib
import http.server
import json
import os
import secrets
import socketserver
import time
import urllib.parse
import urllib.request
import webbrowser
from pathlib import Path
from typing import Optional

# OAuth Configuration
CLIENT_ID = "9d1c250a-e61b-44d9-88ed-5944d1962f5e"
SCOPES = "org:create_api_key user:profile user:inference"
REDIRECT_URI = "http://localhost:8765/callback"
AUTH_URL = "https://claude.ai/oauth/authorize"
TOKEN_URL = "https://console.anthropic.com/v1/oauth/token"
API_URL = "https://api.anthropic.com/v1/messages"

# Callback server port
PORT = 8765

# Cache files
CACHE_DIR = Path.home() / ".cache" / "anthropic-oauth-test"
TOKEN_CACHE_FILE = CACHE_DIR / "tokens.json"
AUTH_CODE_CACHE_FILE = CACHE_DIR / "auth_code.json"

# Token expiry buffer (refresh 5 minutes before expiry)
EXPIRY_BUFFER_SECONDS = 5 * 60


def generate_pkce() -> tuple[str, str]:
    """Generate PKCE code_verifier and code_challenge (S256)."""
    verifier_bytes = secrets.token_bytes(64)
    code_verifier = base64.urlsafe_b64encode(verifier_bytes).rstrip(b"=").decode("ascii")
    challenge_hash = hashlib.sha256(code_verifier.encode("ascii")).digest()
    code_challenge = base64.urlsafe_b64encode(challenge_hash).rstrip(b"=").decode("ascii")
    return code_verifier, code_challenge


def load_cached_auth_code() -> Optional[dict]:
    """Load cached auth code and verifier."""
    if not AUTH_CODE_CACHE_FILE.exists():
        return None
    try:
        with open(AUTH_CODE_CACHE_FILE, "r") as f:
            return json.load(f)
    except (json.JSONDecodeError, IOError):
        return None


def save_auth_code(auth_code: str, code_verifier: str) -> None:
    """Save auth code and verifier for retry."""
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    data = {
        "auth_code": auth_code,
        "code_verifier": code_verifier,
        "timestamp": time.time(),
    }
    with open(AUTH_CODE_CACHE_FILE, "w") as f:
        json.dump(data, f, indent=2)
    os.chmod(AUTH_CODE_CACHE_FILE, 0o600)
    print(f"    Auth code cached to {AUTH_CODE_CACHE_FILE}")


def load_cached_tokens() -> Optional[dict]:
    """Load tokens from cache file if they exist and are valid."""
    if not TOKEN_CACHE_FILE.exists():
        return None
    try:
        with open(TOKEN_CACHE_FILE, "r") as f:
            cached = json.load(f)
        expires_at = cached.get("expires_at", 0)
        if time.time() >= expires_at - EXPIRY_BUFFER_SECONDS:
            print("    Cached token expired or expiring soon")
            return None
        return cached
    except (json.JSONDecodeError, IOError) as e:
        print(f"    Failed to load cache: {e}")
        return None


def save_tokens(token_response: dict) -> None:
    """Save tokens to cache file."""
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    expires_in = token_response.get("expires_in", 3600)
    token_response["expires_at"] = time.time() + expires_in
    with open(TOKEN_CACHE_FILE, "w") as f:
        json.dump(token_response, f, indent=2)
    os.chmod(TOKEN_CACHE_FILE, 0o600)
    print(f"    Tokens cached to {TOKEN_CACHE_FILE}")


def refresh_token(refresh_token: str) -> dict:
    """Refresh the access token using the refresh token."""
    import requests

    data = {
        "grant_type": "refresh_token",
        "client_id": CLIENT_ID,
        "refresh_token": refresh_token,
    }
    response = requests.post(TOKEN_URL, json=data)
    response.raise_for_status()
    return response.json()


class CallbackHandler(http.server.BaseHTTPRequestHandler):
    """Handle OAuth callback."""
    auth_code: Optional[str] = None
    state: Optional[str] = None
    error: Optional[str] = None

    @classmethod
    def reset(cls):
        cls.auth_code = None
        cls.state = None
        cls.error = None

    def do_GET(self):
        parsed = urllib.parse.urlparse(self.path)
        if parsed.path != "/callback":
            self.send_response(404)
            self.end_headers()
            return

        params = urllib.parse.parse_qs(parsed.query)
        if "error" in params:
            CallbackHandler.error = params.get("error", ["unknown"])[0]
            error_desc = params.get("error_description", ["No description"])[0]
            self.send_response(400)
            self.send_header("Content-type", "text/html")
            self.end_headers()
            self.wfile.write(f"<h1>Error: {CallbackHandler.error}</h1><p>{error_desc}</p>".encode())
            return

        CallbackHandler.auth_code = params.get("code", [None])[0]
        CallbackHandler.state = params.get("state", [None])[0]
        self.send_response(200)
        self.send_header("Content-type", "text/html")
        self.end_headers()
        self.wfile.write(b"<h1>Authorization successful!</h1><p>You can close this window.</p>")

    def log_message(self, format, *args):
        pass


def wait_for_callback() -> tuple[Optional[str], Optional[str]]:
    """Start server and wait for callback."""
    socketserver.TCPServer.allow_reuse_address = True
    with socketserver.TCPServer(("", PORT), CallbackHandler) as httpd:
        httpd.timeout = 300
        httpd.handle_request()
        return CallbackHandler.auth_code, CallbackHandler.state


def exchange_code_for_token(code: str, code_verifier: str, state: str) -> dict:
    """Exchange authorization code for tokens."""
    import requests

    data = {
        "grant_type": "authorization_code",
        "client_id": CLIENT_ID,
        "code": code,
        "redirect_uri": REDIRECT_URI,
        "code_verifier": code_verifier,
        "state": state,  # Required! The state from the callback
    }

    response = requests.post(TOKEN_URL, json=data)
    if not response.ok:
        print(f"    [ERROR] {response.status_code}: {response.text}")
        response.raise_for_status()

    return response.json()


def test_api_call(access_token: str) -> dict:
    """Test API call with the access token."""
    data = {
        "model": "claude-opus-4-5-20251101",
        "max_tokens": 10000,
        "system": [
            {
              "type": "text",
              "text": "You are Claude Code, Anthropic's official CLI for Claude.",
              "cache_control": {
                "type": "ephemeral"
              }
            },
        ],
        "messages": [
            {"role": "assistant", "content": "Say hello in exactly 5 words."},
        ],
    }
    headers = {
        "Authorization": f"Bearer {access_token}",
        "anthropic-version": "2023-06-01",
        "anthropic-beta": "oauth-2025-04-20,claude-code-20250219,interleaved-thinking-2025-05-14,fine-grained-tool-streaming-2025-05-14",
        "Content-Type": "application/json",
        "user-agent": "ai-sdk/anthropic/2.0.50 ai-sdk/provider-utils/3.0.18 runtime/bun/1.3.5"
    }
    req = urllib.request.Request(
        API_URL,
        data=json.dumps(data).encode("utf-8"),
        headers=headers,
        method="POST",
    )
    with urllib.request.urlopen(req) as response:
        return json.loads(response.read().decode("utf-8"))


def do_oauth_flow() -> tuple[Optional[str], Optional[str]]:
    """Perform the OAuth flow to get auth code. Returns (auth_code, code_verifier)."""
    CallbackHandler.reset()

    print("\n[1] Generating PKCE credentials...")
    code_verifier, code_challenge = generate_pkce()
    print(f"    Code verifier: {code_verifier}")
    print(f"    Code challenge: {code_challenge}")

    print("\n[2] Building authorization URL...")
    auth_params = {
        "code": "true",
        "client_id": CLIENT_ID,
        "response_type": "code",
        "redirect_uri": REDIRECT_URI,
        "scope": SCOPES,
        "code_challenge": code_challenge,
        "code_challenge_method": "S256",
        "state": code_verifier,
    }
    auth_url = f"{AUTH_URL}?{urllib.parse.urlencode(auth_params)}"
    print(f"    URL: {auth_url}")

    print("\n[3] Opening browser for authorization...")
    print(f"    Waiting for callback on port {PORT}...")

    webbrowser.open(auth_url)
    auth_code, state = wait_for_callback()

    if CallbackHandler.error:
        print(f"\n[ERROR] Authorization failed: {CallbackHandler.error}")
        return None, None

    if not auth_code:
        print("\n[ERROR] No authorization code received")
        return None, None

    print(f"    Received authorization code: {auth_code}")
    print(f"    Received state: {state}")

    # Cache the auth code for retrying token exchange
    save_auth_code(auth_code, code_verifier)

    return auth_code, code_verifier


def main():
    parser = argparse.ArgumentParser(description="Anthropic OAuth test")
    parser.add_argument("--retry-token", action="store_true",
                        help="Retry token exchange with cached auth code")
    parser.add_argument("--login", action="store_true",
                        help="Force new login even if cached")
    parser.add_argument("--clear", action="store_true",
                        help="Clear all cached data")
    parser.add_argument("--auth-only", action="store_true",
                        help="Only get auth code, don't exchange for token")
    parser.add_argument("--test-api", action="store_true",
                        help="Test API call after getting token")
    args = parser.parse_args()

    print("=" * 60)
    print("Anthropic OAuth Flow Test")
    print("=" * 60)

    if args.clear:
        print("\n[*] Clearing cache...")
        if TOKEN_CACHE_FILE.exists():
            TOKEN_CACHE_FILE.unlink()
            print(f"    Deleted {TOKEN_CACHE_FILE}")
        if AUTH_CODE_CACHE_FILE.exists():
            AUTH_CODE_CACHE_FILE.unlink()
            print(f"    Deleted {AUTH_CODE_CACHE_FILE}")
        print("    Cache cleared.")
        return

    # Check for cached tokens first (unless forcing login or retry)
    if not args.login and not args.retry_token:
        print("\n[*] Checking for cached tokens...")
        cached = load_cached_tokens()
        if cached:
            expires_at = cached.get("expires_at", 0)
            remaining = int(expires_at - time.time())
            access_token = cached.get("access_token", "")
            refresh_tok = cached.get("refresh_token", "")
            print(f"    Found valid cached token!")
            print(f"    Access token: {access_token[:50]}...")
            print(f"    Refresh token: {refresh_tok[:50]}...")
            print(f"    Expires in: {remaining} seconds ({remaining // 3600}h {(remaining % 3600) // 60}m)")

            # Test API call with cached token if requested
            if args.test_api:
                print("\n[*] Testing API call with cached token...")
                try:
                    api_response = test_api_call(access_token)
                    print("    API call successful!")
                    print(f"    Model: {api_response.get('model', 'N/A')}")
                    content = api_response.get("content", [])
                    if content:
                        print(f"    Response: {content[0].get('text', 'N/A')}")
                except urllib.error.HTTPError as e:
                    print(f"\n[ERROR] API call failed: {e.code}")
                    print(f"    Response: {e.read().decode('utf-8')}")
            else:
                print(f"\n    Use --login to force re-authentication")
            return

    # Try to get auth code (from cache if --retry-token, otherwise new login)
    if args.retry_token:
        print("\n[*] Loading cached auth code...")
        cached_auth = load_cached_auth_code()
        if not cached_auth:
            print("[ERROR] No cached auth code found. Run without --retry-token first.")
            return
        auth_code = cached_auth["auth_code"]
        code_verifier = cached_auth["code_verifier"]
        print(f"    Auth code: {auth_code}")
        print(f"    Code verifier: {code_verifier}")
    else:
        auth_code, code_verifier = do_oauth_flow()
        if not auth_code:
            print("\n[ERROR] Failed to get authorization code")
            return

    # If auth-only, stop here
    if args.auth_only:
        print("\n[*] Auth code obtained. Use --retry-token to exchange for tokens.")
        print(f"    Cached at: {AUTH_CODE_CACHE_FILE}")
        return

    # Exchange code for tokens
    # Note: state is the same as code_verifier (we sent state=code_verifier in auth URL)
    print("\n[4] Exchanging code for tokens...")
    try:
        token_response = exchange_code_for_token(auth_code, code_verifier, state=code_verifier)
        print("    Token exchange successful!")
        print(f"    Access token: {token_response.get('access_token', 'N/A')[:50]}...")
        print(f"    Refresh token: {token_response.get('refresh_token', 'N/A')[:50]}...")
        print(f"    Expires in: {token_response.get('expires_in', 'N/A')} seconds")
        save_tokens(token_response)
    except Exception as e:
        print(f"\n[ERROR] Token exchange failed: {e}")
        print("\n    Hint: Auth codes are single-use. If this failed, run --login to get a new one.")
        return

    # Test API call (optional)
    if args.test_api:
        print("\n[5] Testing API call...")
        access_token = token_response.get("access_token")
        try:
            api_response = test_api_call(access_token)
            print("    API call successful!")
            print(f"    Model: {api_response.get('model', 'N/A')}")
            content = api_response.get("content", [])
            if content:
                print(f"    Response: {content[0].get('text', 'N/A')}")
        except urllib.error.HTTPError as e:
            print(f"\n[ERROR] API call failed: {e.code}")
            print(f"    Response: {e.read().decode('utf-8')}")
            return

    print("\n" + "=" * 60)
    print("Token obtained successfully!")
    print("=" * 60)


if __name__ == "__main__":
    main()
