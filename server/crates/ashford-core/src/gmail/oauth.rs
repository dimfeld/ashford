use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json;
use std::convert::Infallible;
use thiserror::Error;

pub const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
pub const DEFAULT_REFRESH_BUFFER: Duration = Duration::minutes(5);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
}

impl OAuthTokens {
    pub fn needs_refresh(&self, now: DateTime<Utc>, buffer: Duration) -> bool {
        now + buffer >= self.expires_at
    }
}

#[derive(Debug, Error)]
pub enum OAuthError {
    #[error("missing refresh token")]
    MissingRefreshToken,
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("token response decode error: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("token endpoint error {status}: {body}")]
    TokenEndpoint { status: u16, body: String },
    #[error("invalid expires_in value: {0}")]
    InvalidExpires(i64),
}

#[async_trait]
pub trait TokenStore: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn save_tokens(&self, tokens: &OAuthTokens) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone, Default)]
pub struct NoopTokenStore;

#[async_trait]
impl TokenStore for NoopTokenStore {
    type Error = Infallible;

    async fn save_tokens(&self, _tokens: &OAuthTokens) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct RefreshResponse {
    access_token: String,
    expires_in: i64,
    #[serde(default)]
    refresh_token: Option<String>,
    #[allow(dead_code)]
    token_type: Option<String>,
}

pub async fn refresh_access_token(
    client: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
    tokens: &OAuthTokens,
) -> Result<OAuthTokens, OAuthError> {
    refresh_access_token_with_endpoint(client, client_id, client_secret, tokens, TOKEN_ENDPOINT)
        .await
}

pub async fn refresh_access_token_with_endpoint(
    client: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
    tokens: &OAuthTokens,
    endpoint: &str,
) -> Result<OAuthTokens, OAuthError> {
    if tokens.refresh_token.is_empty() {
        return Err(OAuthError::MissingRefreshToken);
    }

    let response = client
        .post(endpoint)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("refresh_token", tokens.refresh_token.as_str()),
        ])
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(OAuthError::TokenEndpoint {
            status: status.as_u16(),
            body,
        });
    }

    let body = response.text().await?;
    let payload: RefreshResponse = serde_json::from_str(&body).map_err(OAuthError::Decode)?;
    if payload.expires_in <= 0 {
        return Err(OAuthError::InvalidExpires(payload.expires_in));
    }

    let refresh_token = payload
        .refresh_token
        .unwrap_or_else(|| tokens.refresh_token.clone());
    let expires_at = Utc::now() + Duration::seconds(payload.expires_in.into());

    Ok(OAuthTokens {
        access_token: payload.access_token,
        refresh_token,
        expires_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn needs_refresh_respects_buffer() {
        let tokens = OAuthTokens {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: Utc::now() + Duration::minutes(4),
        };

        assert!(tokens.needs_refresh(Utc::now(), Duration::minutes(5)));
        assert!(!tokens.needs_refresh(Utc::now(), Duration::minutes(1)));
    }

    #[tokio::test]
    async fn refresh_access_token_updates_tokens() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "new_access",
                "refresh_token": "new_refresh",
                "expires_in": 3600,
                "token_type": "Bearer",
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let tokens = OAuthTokens {
            access_token: "old".into(),
            refresh_token: "old_refresh".into(),
            expires_at: Utc::now(),
        };

        let refreshed = refresh_access_token_with_endpoint(
            &client,
            "client",
            "secret",
            &tokens,
            &format!("{}/token", server.uri()),
        )
        .await
        .expect("refresh succeeds");

        assert_eq!(refreshed.access_token, "new_access");
        assert_eq!(refreshed.refresh_token, "new_refresh");
        assert!(refreshed.expires_at > tokens.expires_at);
    }

    #[tokio::test]
    async fn refresh_access_token_errors_on_bad_status() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_string("nope"))
            .expect(1)
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let tokens = OAuthTokens {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: Utc::now(),
        };

        let err = refresh_access_token_with_endpoint(
            &client,
            "client",
            "secret",
            &tokens,
            &format!("{}/token", server.uri()),
        )
        .await
        .expect_err("should fail on non-200");

        assert!(matches!(err, OAuthError::TokenEndpoint { .. }));
    }

    #[tokio::test]
    async fn refresh_access_token_validates_expires() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "new",
                "expires_in": 0,
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let tokens = OAuthTokens {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: Utc::now(),
        };

        let err = refresh_access_token_with_endpoint(
            &client,
            "client",
            "secret",
            &tokens,
            &format!("{}/token", server.uri()),
        )
        .await
        .expect_err("zero expires should fail");

        assert!(matches!(err, OAuthError::InvalidExpires(_)));
    }

    #[tokio::test]
    async fn refresh_access_token_requires_refresh_token() {
        let client = reqwest::Client::new();
        let tokens = OAuthTokens {
            access_token: "a".into(),
            refresh_token: String::new(),
            expires_at: Utc::now(),
        };

        let err = refresh_access_token_with_endpoint(
            &client,
            "client",
            "secret",
            &tokens,
            "http://localhost/token",
        )
        .await
        .expect_err("missing refresh token");

        assert!(matches!(err, OAuthError::MissingRefreshToken));
    }

    #[tokio::test]
    async fn refresh_access_token_surfaces_decode_errors() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
            .expect(1)
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let tokens = OAuthTokens {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: Utc::now(),
        };

        let err = refresh_access_token_with_endpoint(
            &client,
            "client",
            "secret",
            &tokens,
            &format!("{}/token", server.uri()),
        )
        .await
        .expect_err("should surface decode errors");

        assert!(matches!(err, OAuthError::Decode(_)));
    }

    #[tokio::test]
    async fn refresh_access_token_retains_existing_refresh_token() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "new_access",
                "expires_in": 1200
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let tokens = OAuthTokens {
            access_token: "old".into(),
            refresh_token: "keep_me".into(),
            expires_at: Utc::now(),
        };

        let refreshed = refresh_access_token_with_endpoint(
            &client,
            "client",
            "secret",
            &tokens,
            &format!("{}/token", server.uri()),
        )
        .await
        .expect("refresh succeeds");

        assert_eq!(refreshed.refresh_token, "keep_me");
    }
}
