use ashford_core::accounts::{AccountConfig, PubsubConfig};
use ashford_core::gmail::oauth::{OAuthTokens, TOKEN_ENDPOINT};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{Duration as ChronoDuration, Utc};
use rand::RngCore;
use rand::rngs::OsRng;
use reqwest::{Client, Url};
use serde::Deserialize;
use std::env;
use std::error::Error;
use std::io::{self, Write};
use std::process::Command;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time;

type AnyError = Box<dyn Error + Send + Sync>;

const AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const CALLBACK_PATH: &str = "/oauth2callback";
const SUCCESS_HTML: &str = r#"<!doctype html>
<html>
  <head><title>Gmail OAuth</title></head>
  <body style="font-family: sans-serif;">
    <h2>You can close this window</h2>
    <p>Return to the terminal to finish setup.</p>
  </body>
</html>
"#;

#[derive(Debug, Clone)]
struct Inputs {
    client_id: String,
    client_secret: String,
    scopes: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), AnyError> {
    let inputs = gather_inputs()?;
    let state = random_state();

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let redirect_uri = format!("http://127.0.0.1:{}{}", port, CALLBACK_PATH);

    let auth_url = build_auth_url(&inputs.client_id, &redirect_uri, &inputs.scopes, &state)?;
    println!("Opening browser for Google consent...");
    if let Err(err) = maybe_open_browser(auth_url.as_str()) {
        eprintln!("Could not open browser automatically: {err}. Please open the URL manually.");
    }
    println!(
        "If the browser did not open, paste this into a new tab:\n{}\n",
        auth_url
    );
    println!("Listening on {} for the OAuth callback...\n", redirect_uri);

    let code = match time::timeout(Duration::from_secs(300), wait_for_code(listener, state)).await {
        Ok(result) => result?,
        Err(_) => return Err("Timed out waiting for OAuth callback (5 minutes)".into()),
    };

    println!("Received authorization code, exchanging for tokens...");

    let client = Client::new();
    let tokens = exchange_code_for_tokens(
        &client,
        &inputs.client_id,
        &inputs.client_secret,
        &code,
        &redirect_uri,
    )
    .await?;

    let tokens_json = serde_json::to_string_pretty(&tokens)?;
    println!("\nOAuth tokens:");
    println!("{tokens_json}");

    let account_config = AccountConfig {
        client_id: inputs.client_id,
        client_secret: inputs.client_secret,
        oauth: tokens.clone(),
        pubsub: PubsubConfig::default(),
    };
    let config_json = serde_json::to_string_pretty(&account_config)?;
    println!("\nAccount config JSON (paste into accounts.config_json when creating an account):");
    println!("{config_json}");

    println!("\nDone. Keep your refresh token safe.");

    Ok(())
}

fn gather_inputs() -> Result<Inputs, AnyError> {
    let client_id = env_var_or_prompt("GMAIL_CLIENT_ID", "Google OAuth client ID")?;
    let client_secret = env_var_or_prompt("GMAIL_CLIENT_SECRET", "Google OAuth client secret")?;

    let scopes = match env::var("GMAIL_OAUTH_SCOPES") {
        Ok(value) if !value.trim().is_empty() => {
            value.split_whitespace().map(|s| s.to_string()).collect()
        }
        _ => {
            println!("Scopes to request (space-separated). Press enter to accept the default:");
            println!("  default: https://www.googleapis.com/auth/gmail.modify\n");
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let trimmed = input.trim();
            if trimmed.is_empty() {
                vec!["https://www.googleapis.com/auth/gmail.modify".to_string()]
            } else {
                trimmed.split_whitespace().map(|s| s.to_string()).collect()
            }
        }
    };

    Ok(Inputs {
        client_id,
        client_secret,
        scopes,
    })
}

fn env_var_or_prompt(key: &str, prompt: &str) -> Result<String, AnyError> {
    if let Ok(value) = env::var(key) {
        if !value.trim().is_empty() {
            return Ok(value);
        }
    }

    print!("{}: ", prompt);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim();
    if trimmed.is_empty() {
        Err(format!("{} is required", prompt).into())
    } else {
        Ok(trimmed.to_string())
    }
}

fn random_state() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn build_auth_url(
    client_id: &str,
    redirect_uri: &str,
    scopes: &[String],
    state: &str,
) -> Result<Url, AnyError> {
    let scope_value = scopes.join(" ");
    let url = Url::parse_with_params(
        AUTH_ENDPOINT,
        [
            ("client_id", client_id),
            ("redirect_uri", redirect_uri),
            ("response_type", "code"),
            ("scope", scope_value.as_str()),
            ("access_type", "offline"),
            ("prompt", "consent"),
            ("state", state),
            ("include_granted_scopes", "true"),
        ],
    )?;
    Ok(url)
}

async fn wait_for_code(listener: TcpListener, expected_state: String) -> Result<String, AnyError> {
    let (mut stream, _addr) = listener.accept().await?;

    let mut buf = Vec::new();
    let mut chunk = [0u8; 1024];

    // Read until we hit the end of headers or a reasonable limit.
    for _ in 0..16 {
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        if buf.len() > 8192 {
            break;
        }
    }

    let request = String::from_utf8_lossy(&buf);
    let request_line = request.lines().next().ok_or("Malformed HTTP request")?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("");

    if method != "GET" {
        send_response(&mut stream, 405, "Only GET is supported").await?;
        return Err("Unexpected HTTP method".into());
    }

    let url = match Url::parse(&format!("http://localhost{}", path)) {
        Ok(url) => url,
        Err(err) => {
            send_response(
                &mut stream,
                400,
                "Malformed OAuth callback URL. Please retry the OAuth flow.",
            )
            .await?;
            return Err(err.into());
        }
    };
    let mut code: Option<String> = None;
    let mut state: Option<String> = None;
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "code" => code = Some(value.to_string()),
            "state" => state = Some(value.to_string()),
            _ => {}
        }
    }

    let code = match code {
        Some(code) if !code.is_empty() => code,
        _ => {
            send_response(
                &mut stream,
                400,
                "Missing code in callback. Please retry the OAuth flow.",
            )
            .await?;
            return Err("Missing code in callback".into());
        }
    };

    if state.as_deref() != Some(expected_state.as_str()) {
        send_response(
            &mut stream,
            400,
            "State mismatch, please retry the OAuth flow.",
        )
        .await?;
        return Err("State mismatch".into());
    }

    send_response(&mut stream, 200, SUCCESS_HTML).await?;
    Ok(code)
}

async fn send_response(
    stream: &mut tokio::net::TcpStream,
    status: u16,
    body: &str,
) -> io::Result<()> {
    let status_line = match status {
        200 => "200 OK",
        400 => "400 Bad Request",
        405 => "405 Method Not Allowed",
        _ => "200 OK",
    };

    let response = format!(
        "HTTP/1.1 {status_line}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes()).await
}

#[derive(Debug, Deserialize)]
struct CodeTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
    #[allow(dead_code)]
    scope: Option<String>,
    #[allow(dead_code)]
    token_type: Option<String>,
}

async fn exchange_code_for_tokens(
    client: &Client,
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
) -> Result<OAuthTokens, AnyError> {
    exchange_code_for_tokens_with_endpoint(
        client,
        client_id,
        client_secret,
        code,
        redirect_uri,
        TOKEN_ENDPOINT,
    )
    .await
}

async fn exchange_code_for_tokens_with_endpoint(
    client: &Client,
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
    endpoint: &str,
) -> Result<OAuthTokens, AnyError> {
    let response = client
        .post(endpoint)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("redirect_uri", redirect_uri),
        ])
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(format!("Token endpoint returned {}: {}", status.as_u16(), body).into());
    }

    let payload: CodeTokenResponse = serde_json::from_str(&body)?;
    let refresh_token = payload.refresh_token.ok_or(
        "Token response missing refresh_token. Re-run with prompt=consent and offline access.",
    )?;

    if payload.expires_in <= 0 {
        return Err(format!("Invalid expires_in value: {}", payload.expires_in).into());
    }

    let expires_at = Utc::now() + ChronoDuration::seconds(payload.expires_in);

    Ok(OAuthTokens {
        access_token: payload.access_token,
        refresh_token,
        expires_at,
    })
}

fn maybe_open_browser(url: &str) -> Result<(), AnyError> {
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(target_os = "linux")]
    let mut command = Command::new("xdg-open");
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut cmd = Command::new("cmd");
        let escaped_url = url.replace('"', "\"\"");
        cmd.arg("/C").arg(format!("start \"\" \"{}\"", escaped_url));
        cmd
    };

    #[cfg(not(target_os = "windows"))]
    command.arg(url);
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("browser command exited with status {status}").into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn build_auth_url_includes_expected_params() {
        let url = build_auth_url(
            "client",
            "http://localhost:8080/callback",
            &["scope1".into(), "scope2".into()],
            "state123",
        )
        .expect("url builds");

        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("accounts.google.com"));
        let params: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(params.get("client_id"), Some(&"client".to_string()));
        assert_eq!(
            params.get("redirect_uri"),
            Some(&"http://localhost:8080/callback".to_string())
        );
        assert_eq!(params.get("response_type"), Some(&"code".to_string()));
        assert_eq!(params.get("scope"), Some(&"scope1 scope2".to_string()));
        assert_eq!(params.get("state"), Some(&"state123".to_string()));
        assert_eq!(params.get("access_type"), Some(&"offline".to_string()));
        assert_eq!(params.get("prompt"), Some(&"consent".to_string()));
    }

    #[test]
    fn random_state_is_urlsafe_and_correct_length() {
        let state = random_state();
        assert!(state.len() >= 43); // 32 bytes => 43 chars without padding
        let decoded = URL_SAFE_NO_PAD
            .decode(state.as_bytes())
            .expect("state decodes");
        assert_eq!(decoded.len(), 32);
    }

    #[tokio::test]
    async fn wait_for_code_returns_authorization_code() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let wait = tokio::spawn(wait_for_code(listener, "state".to_string()));

        let mut stream = TcpStream::connect(addr).await.unwrap();
        let request = format!(
            "GET {}?code=abc&state=state HTTP/1.1\r\nHost: localhost\r\n\r\n",
            CALLBACK_PATH
        );
        stream.write_all(request.as_bytes()).await.unwrap();
        stream.shutdown().await.unwrap();

        let code = wait.await.unwrap().expect("code returned");
        assert_eq!(code, "abc");
    }

    #[tokio::test]
    async fn wait_for_code_rejects_state_mismatch() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let wait = tokio::spawn(wait_for_code(listener, "expected".to_string()));

        let mut stream = TcpStream::connect(addr).await.unwrap();
        let request = format!(
            "GET {}?code=abc&state=wrong HTTP/1.1\r\nHost: localhost\r\n\r\n",
            CALLBACK_PATH
        );
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await.unwrap();
        let response = String::from_utf8_lossy(&buf);
        assert!(response.contains("400 Bad Request"));

        let err = wait.await.unwrap().expect_err("state mismatch");
        assert!(err.to_string().contains("State mismatch"));
    }

    #[tokio::test]
    async fn wait_for_code_returns_error_for_missing_code() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let wait = tokio::spawn(wait_for_code(listener, "state".to_string()));

        let mut stream = TcpStream::connect(addr).await.unwrap();
        let request = format!(
            "GET {}?state=state HTTP/1.1\r\nHost: localhost\r\n\r\n",
            CALLBACK_PATH
        );
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await.unwrap();
        let response = String::from_utf8_lossy(&buf);
        assert!(response.contains("400 Bad Request"));

        let err = wait.await.unwrap().expect_err("missing code");
        assert!(err.to_string().contains("Missing code"));
    }

    #[tokio::test]
    async fn wait_for_code_rejects_non_get() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let wait = tokio::spawn(wait_for_code(listener, "state".to_string()));

        let mut stream = TcpStream::connect(addr).await.unwrap();
        let request = format!(
            "POST {}?code=abc&state=state HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\n\r\n",
            CALLBACK_PATH
        );
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await.unwrap();
        let response = String::from_utf8_lossy(&buf);
        assert!(response.contains("405 Method Not Allowed"));

        let err = wait.await.unwrap().expect_err("method mismatch");
        assert!(err.to_string().contains("Unexpected HTTP method"));
    }

    #[tokio::test]
    async fn exchange_code_for_tokens_happy_path() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "access",
                "refresh_token": "refresh",
                "expires_in": 3600,
                "token_type": "Bearer"
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = Client::new();
        let tokens = exchange_code_for_tokens_with_endpoint(
            &client,
            "client",
            "secret",
            "code123",
            "http://localhost/callback",
            &format!("{}/token", server.uri()),
        )
        .await
        .expect("tokens returned");

        assert_eq!(tokens.access_token, "access");
        assert_eq!(tokens.refresh_token, "refresh");
        assert!(tokens.expires_at > Utc::now());
    }

    #[tokio::test]
    async fn exchange_code_for_tokens_errors_on_http_failure() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_string("bad"))
            .expect(1)
            .mount(&server)
            .await;

        let client = Client::new();
        let err = exchange_code_for_tokens_with_endpoint(
            &client,
            "client",
            "secret",
            "code",
            "http://localhost/callback",
            &format!("{}/token", server.uri()),
        )
        .await
        .expect_err("http error surfaces");

        assert!(err.to_string().contains("Token endpoint returned 400"));
    }

    #[tokio::test]
    async fn exchange_code_for_tokens_requires_refresh_token() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "access",
                "expires_in": 3600
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = Client::new();
        let err = exchange_code_for_tokens_with_endpoint(
            &client,
            "client",
            "secret",
            "code",
            "http://localhost/callback",
            &format!("{}/token", server.uri()),
        )
        .await
        .expect_err("missing refresh token");

        assert!(
            err.to_string()
                .contains("Token response missing refresh_token")
        );
    }

    #[tokio::test]
    async fn exchange_code_for_tokens_validates_expires() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "access",
                "refresh_token": "refresh",
                "expires_in": -1
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = Client::new();
        let err = exchange_code_for_tokens_with_endpoint(
            &client,
            "client",
            "secret",
            "code",
            "http://localhost/callback",
            &format!("{}/token", server.uri()),
        )
        .await
        .expect_err("invalid expires");

        assert!(err.to_string().contains("Invalid expires_in value"));
    }
}
