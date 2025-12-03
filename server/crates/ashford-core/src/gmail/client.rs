use std::sync::Arc;

use chrono::Utc;
use reqwest::{Client, StatusCode};
use serde::de::DeserializeOwned;
use serde_json;
use thiserror::Error;
use tokio::sync::{Mutex, RwLock};

use crate::gmail::{
    oauth::{
        DEFAULT_REFRESH_BUFFER, OAuthError, OAuthTokens, TOKEN_ENDPOINT, TokenStore,
        refresh_access_token_with_endpoint,
    },
    types::{
        ListHistoryResponse, ListLabelsResponse, ListMessagesResponse, Message, Profile, Thread,
    },
};

const DEFAULT_API_BASE: &str = "https://gmail.googleapis.com/gmail/v1/users";

#[derive(Debug, Error)]
pub enum GmailClientError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("oauth error: {0}")]
    OAuth(#[from] OAuthError),
    #[error("token persistence error: {0}")]
    TokenStore(String),
    #[error("decode error: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("unauthorized after refresh")]
    Unauthorized,
}

pub struct GmailClient<S: TokenStore> {
    http: Client,
    user_id: String,
    client_id: String,
    client_secret: String,
    api_base: String,
    token_endpoint: String,
    tokens: RwLock<OAuthTokens>,
    refresh_lock: Mutex<()>,
    token_store: Arc<S>,
}

impl<S: TokenStore> GmailClient<S> {
    pub fn new(
        http: Client,
        user_id: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        initial_tokens: OAuthTokens,
        token_store: Arc<S>,
    ) -> Self {
        Self {
            http,
            user_id: user_id.into(),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            api_base: DEFAULT_API_BASE.to_string(),
            token_endpoint: TOKEN_ENDPOINT.to_string(),
            tokens: RwLock::new(initial_tokens),
            refresh_lock: Mutex::new(()),
            token_store,
        }
    }

    pub fn with_api_base(mut self, api_base: impl Into<String>) -> Self {
        self.api_base = api_base.into();
        self
    }

    pub fn with_token_endpoint(mut self, token_endpoint: impl Into<String>) -> Self {
        self.token_endpoint = token_endpoint.into();
        self
    }

    pub async fn get_message(&self, message_id: &str) -> Result<Message, GmailClientError> {
        let url = format!("{}/{}/messages/{}", self.api_base, self.user_id, message_id);
        self.send_json(|| self.http.get(&url).query(&[("format", "full")]))
            .await
    }

    pub async fn get_thread(&self, thread_id: &str) -> Result<Thread, GmailClientError> {
        let url = format!("{}/{}/threads/{}", self.api_base, self.user_id, thread_id);
        self.send_json(|| self.http.get(&url).query(&[("format", "full")]))
            .await
    }

    pub async fn list_history(
        &self,
        start_history_id: &str,
        page_token: Option<&str>,
        max_results: Option<u32>,
    ) -> Result<ListHistoryResponse, GmailClientError> {
        let url = format!("{}/{}/history", self.api_base, self.user_id);
        self.send_json(|| {
            let mut builder = self
                .http
                .get(&url)
                .query(&[("startHistoryId", start_history_id)]);
            if let Some(token) = page_token {
                builder = builder.query(&[("pageToken", token)]);
            }
            if let Some(max) = max_results {
                builder = builder.query(&[("maxResults", max)]);
            }
            builder
        })
        .await
    }

    pub async fn list_messages(
        &self,
        query: Option<&str>,
        page_token: Option<&str>,
        include_spam_trash: bool,
        max_results: Option<u32>,
    ) -> Result<ListMessagesResponse, GmailClientError> {
        let url = format!("{}/{}/messages", self.api_base, self.user_id);
        self.send_json(|| {
            let mut builder = self.http.get(&url);
            if let Some(q) = query {
                builder = builder.query(&[("q", q)]);
            }
            if let Some(token) = page_token {
                builder = builder.query(&[("pageToken", token)]);
            }
            if include_spam_trash {
                builder = builder.query(&[("includeSpamTrash", "true")]);
            }
            if let Some(max) = max_results {
                builder = builder.query(&[("maxResults", max)]);
            }
            builder
        })
        .await
    }

    /// Fetches the user's Gmail profile, including the current historyId.
    pub async fn get_profile(&self) -> Result<Profile, GmailClientError> {
        let url = format!("{}/{}/profile", self.api_base, self.user_id);
        self.send_json(|| self.http.get(&url)).await
    }

    /// Fetches all labels for the user's Gmail account.
    pub async fn list_labels(&self) -> Result<ListLabelsResponse, GmailClientError> {
        let url = format!("{}/{}/labels", self.api_base, self.user_id);
        self.send_json(|| self.http.get(&url)).await
    }

    async fn send_json<T, B>(&self, build: B) -> Result<T, GmailClientError>
    where
        T: DeserializeOwned,
        B: Fn() -> reqwest::RequestBuilder + Send + Sync,
    {
        let response = self.perform_authenticated(build).await?;
        let body = response.text().await?;
        serde_json::from_str(&body).map_err(GmailClientError::Decode)
    }

    async fn perform_authenticated<B>(
        &self,
        build: B,
    ) -> Result<reqwest::Response, GmailClientError>
    where
        B: Fn() -> reqwest::RequestBuilder + Send + Sync,
    {
        let tokens = self.ensure_fresh_token(false).await?;
        let mut response = build().bearer_auth(&tokens.access_token).send().await?;

        if response.status() == StatusCode::UNAUTHORIZED {
            let tokens = self.ensure_fresh_token(true).await?;
            response = build().bearer_auth(&tokens.access_token).send().await?;
        }

        if response.status() == StatusCode::UNAUTHORIZED {
            return Err(GmailClientError::Unauthorized);
        }

        Ok(response.error_for_status()?)
    }

    async fn ensure_fresh_token(
        &self,
        force_refresh: bool,
    ) -> Result<OAuthTokens, GmailClientError> {
        {
            let tokens = self.tokens.read().await;
            if !force_refresh && !tokens.needs_refresh(Utc::now(), DEFAULT_REFRESH_BUFFER) {
                return Ok(tokens.clone());
            }
        }

        let _guard = self.refresh_lock.lock().await;

        {
            let tokens = self.tokens.read().await;
            if !force_refresh && !tokens.needs_refresh(Utc::now(), DEFAULT_REFRESH_BUFFER) {
                return Ok(tokens.clone());
            }
        }

        let current = { self.tokens.read().await.clone() };
        let refreshed = refresh_access_token_with_endpoint(
            &self.http,
            &self.client_id,
            &self.client_secret,
            &current,
            &self.token_endpoint,
        )
        .await?;

        {
            let mut tokens = self.tokens.write().await;
            *tokens = refreshed.clone();
        }

        self.token_store
            .save_tokens(&refreshed)
            .await
            .map_err(|err| GmailClientError::TokenStore(err.to_string()))?;

        Ok(refreshed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Duration;
    use serde_json::json;
    use tokio::sync::Mutex as TokioMutex;
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[derive(Default)]
    struct RecordingStore {
        saved: TokioMutex<Vec<OAuthTokens>>,
    }

    #[async_trait]
    impl TokenStore for RecordingStore {
        type Error = std::convert::Infallible;

        async fn save_tokens(&self, tokens: &OAuthTokens) -> Result<(), Self::Error> {
            self.saved.lock().await.push(tokens.clone());
            Ok(())
        }
    }

    #[derive(Debug)]
    struct StoreError;

    impl std::fmt::Display for StoreError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "store failure")
        }
    }

    impl std::error::Error for StoreError {}

    struct FailingStore;

    #[async_trait]
    impl TokenStore for FailingStore {
        type Error = StoreError;

        async fn save_tokens(&self, _tokens: &OAuthTokens) -> Result<(), Self::Error> {
            Err(StoreError)
        }
    }

    fn make_client(
        server: &MockServer,
        tokens: OAuthTokens,
        store: Arc<RecordingStore>,
    ) -> GmailClient<RecordingStore> {
        GmailClient::new(
            reqwest::Client::new(),
            "me",
            "client",
            "secret",
            tokens,
            store,
        )
        .with_api_base(format!("{}/gmail/v1/users", server.uri()))
        .with_token_endpoint(format!("{}/token", server.uri()))
    }

    #[tokio::test]
    async fn refreshes_before_request_when_expiring() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "new_token",
                "refresh_token": "refresh_two",
                "expires_in": 3600,
            })))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/me/messages/abc"))
            .and(header("authorization", "Bearer new_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "abc",
                "labelIds": [],
            })))
            .expect(1)
            .mount(&server)
            .await;

        let tokens = OAuthTokens {
            access_token: "old_token".into(),
            refresh_token: "refresh_one".into(),
            expires_at: Utc::now() + Duration::minutes(1),
        };
        let store = Arc::new(RecordingStore::default());
        let client = make_client(&server, tokens, store.clone());

        let message = client.get_message("abc").await.expect("message loads");

        assert_eq!(message.id, "abc");
        let saved = store.saved.lock().await;
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].access_token, "new_token");
    }

    #[tokio::test]
    async fn retries_after_unauthorized_and_uses_refreshed_token() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "fresh_token",
                "refresh_token": "refresh_new",
                "expires_in": 3600,
            })))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/me/messages/abc"))
            .and(header("authorization", "Bearer old_token"))
            .respond_with(ResponseTemplate::new(401))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/me/messages/abc"))
            .and(header("authorization", "Bearer fresh_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "abc",
                "labelIds": [],
            })))
            .expect(1)
            .mount(&server)
            .await;

        let tokens = OAuthTokens {
            access_token: "old_token".into(),
            refresh_token: "refresh_old".into(),
            expires_at: Utc::now() + Duration::minutes(10),
        };
        let store = Arc::new(RecordingStore::default());
        let client = make_client(&server, tokens, store.clone());

        let message = client.get_message("abc").await.expect("message loads");
        assert_eq!(message.id, "abc");

        let saved = store.saved.lock().await;
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].access_token, "fresh_token");
    }

    #[tokio::test]
    async fn returns_unauthorized_if_retry_still_fails() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "fresh_token",
                "expires_in": 3600,
            })))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/me/messages/abc"))
            .and(header("authorization", "Bearer old_token"))
            .respond_with(ResponseTemplate::new(401))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/me/messages/abc"))
            .and(header("authorization", "Bearer fresh_token"))
            .respond_with(ResponseTemplate::new(401))
            .expect(1)
            .mount(&server)
            .await;

        let tokens = OAuthTokens {
            access_token: "old_token".into(),
            refresh_token: "refresh_old".into(),
            expires_at: Utc::now() + Duration::minutes(10),
        };
        let store = Arc::new(RecordingStore::default());
        let client = make_client(&server, tokens, store);

        let err = client
            .get_message("abc")
            .await
            .expect_err("should surface unauthorized");

        assert!(matches!(err, GmailClientError::Unauthorized));
    }

    #[tokio::test]
    async fn surfaces_not_found_errors() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/me/messages/missing"))
            .respond_with(ResponseTemplate::new(404))
            .expect(1)
            .mount(&server)
            .await;

        let tokens = OAuthTokens {
            access_token: "token".into(),
            refresh_token: "refresh".into(),
            expires_at: Utc::now() + Duration::hours(1),
        };
        let store = Arc::new(RecordingStore::default());
        let client = make_client(&server, tokens, store);

        let err = client
            .get_message("missing")
            .await
            .expect_err("should surface 404");

        match err {
            GmailClientError::Http(e) => {
                assert_eq!(e.status(), Some(StatusCode::NOT_FOUND));
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[tokio::test]
    async fn get_thread_uses_existing_token_and_full_format() {
        let server = MockServer::start().await;

        // Should not refresh when tokens are fresh.
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(500))
            .expect(0)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/me/threads/abc"))
            .and(query_param("format", "full"))
            .and(header("authorization", "Bearer token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "abc",
                "messages": [
                    { "id": "m1", "threadId": "abc", "labelIds": [] }
                ]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let tokens = OAuthTokens {
            access_token: "token".into(),
            refresh_token: "refresh".into(),
            expires_at: Utc::now() + Duration::hours(2),
        };
        let store = Arc::new(RecordingStore::default());
        let client = make_client(&server, tokens, store.clone());

        let thread = client.get_thread("abc").await.expect("thread loads");

        assert_eq!(thread.id, "abc");
        assert_eq!(thread.messages.len(), 1);
        assert_eq!(thread.messages[0].id, "m1");

        let saved = store.saved.lock().await;
        assert!(saved.is_empty(), "tokens should not be refreshed");
    }

    #[tokio::test]
    async fn surfaces_rate_limit_errors() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/me/messages"))
            .respond_with(ResponseTemplate::new(429))
            .expect(1)
            .mount(&server)
            .await;

        let tokens = OAuthTokens {
            access_token: "token".into(),
            refresh_token: "refresh".into(),
            expires_at: Utc::now() + Duration::hours(1),
        };
        let store = Arc::new(RecordingStore::default());
        let client = make_client(&server, tokens, store);

        let err = client
            .list_messages(None, None, false, None)
            .await
            .expect_err("should surface 429");

        match err {
            GmailClientError::Http(e) => {
                assert_eq!(e.status(), Some(StatusCode::TOO_MANY_REQUESTS));
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[tokio::test]
    async fn parses_list_history_response() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/me/history"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "history": [
                    {
                        "id": "10",
                        "messages": [
                            { "id": "m1", "threadId": "t1" }
                        ],
                        "messagesAdded": [
                            { "message": { "id": "m2", "threadId": "t2" } }
                        ],
                        "messagesDeleted": [
                            { "message": { "id": "m3", "threadId": "t3" } }
                        ],
                        "labelsAdded": [
                            { "message": { "id": "m4", "threadId": "t4" }, "labelIds": ["INBOX"] }
                        ],
                        "labelsRemoved": [
                            { "message": { "id": "m5", "threadId": "t5" }, "labelIds": ["TRASH"] }
                        ]
                    }
                ],
                "nextPageToken": "next",
                "historyId": "10"
            })))
            .expect(1)
            .mount(&server)
            .await;

        let tokens = OAuthTokens {
            access_token: "token".into(),
            refresh_token: "refresh".into(),
            expires_at: Utc::now() + Duration::hours(1),
        };
        let store = Arc::new(RecordingStore::default());
        let client = make_client(&server, tokens, store);

        let response = client
            .list_history("5", Some("page"), Some(50))
            .await
            .expect("parses list history");

        assert_eq!(response.history.len(), 1);
        assert_eq!(response.next_page_token.as_deref(), Some("next"));
        assert_eq!(response.history_id.as_deref(), Some("10"));
        let record = &response.history[0];
        assert_eq!(record.messages.as_ref().unwrap()[0].id, "m1");
        assert_eq!(record.messages_added.as_ref().unwrap()[0].message.id, "m2");
        assert_eq!(
            record.messages_deleted.as_ref().unwrap()[0].message.id,
            "m3"
        );
        assert_eq!(
            record.labels_added.as_ref().unwrap()[0].label_ids,
            vec!["INBOX"]
        );
        assert_eq!(
            record.labels_removed.as_ref().unwrap()[0].label_ids,
            vec!["TRASH"]
        );
    }

    #[tokio::test]
    async fn parses_list_messages_response() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/me/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "messages": [
                    { "id": "m1", "threadId": "t1" },
                    { "id": "m2" }
                ],
                "nextPageToken": "p2",
                "resultSizeEstimate": 2
            })))
            .expect(1)
            .mount(&server)
            .await;

        let tokens = OAuthTokens {
            access_token: "token".into(),
            refresh_token: "refresh".into(),
            expires_at: Utc::now() + Duration::hours(1),
        };
        let store = Arc::new(RecordingStore::default());
        let client = make_client(&server, tokens, store);

        let response = client
            .list_messages(Some("from:me"), Some("next"), true, Some(25))
            .await
            .expect("parses list messages");

        assert_eq!(response.messages.len(), 2);
        assert_eq!(response.messages[0].id, "m1");
        assert_eq!(response.messages[0].thread_id.as_deref(), Some("t1"));
        assert_eq!(response.messages[1].thread_id, None);
        assert_eq!(response.next_page_token.as_deref(), Some("p2"));
        assert_eq!(response.result_size_estimate, Some(2));
    }

    #[tokio::test]
    async fn list_messages_builds_expected_query_params() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/me/messages"))
            .and(query_param("q", "from:me"))
            .and(query_param("pageToken", "token2"))
            .and(query_param("includeSpamTrash", "true"))
            .and(query_param("maxResults", "50"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "messages": [],
                "resultSizeEstimate": 0
            })))
            .expect(1)
            .mount(&server)
            .await;

        let tokens = OAuthTokens {
            access_token: "token".into(),
            refresh_token: "refresh".into(),
            expires_at: Utc::now() + Duration::hours(1),
        };
        let store = Arc::new(RecordingStore::default());
        let client = make_client(&server, tokens, store.clone());

        let response = client
            .list_messages(Some("from:me"), Some("token2"), true, Some(50))
            .await
            .expect("list messages succeeds");

        assert!(response.messages.is_empty());
        assert_eq!(response.result_size_estimate, Some(0));
        let saved = store.saved.lock().await;
        assert!(saved.is_empty(), "tokens should not be refreshed");
    }

    #[tokio::test]
    async fn list_history_builds_expected_query_params() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/me/history"))
            .and(query_param("startHistoryId", "123"))
            .and(query_param("pageToken", "p2"))
            .and(query_param("maxResults", "100"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "history": [],
                "historyId": "125"
            })))
            .expect(1)
            .mount(&server)
            .await;

        let tokens = OAuthTokens {
            access_token: "token".into(),
            refresh_token: "refresh".into(),
            expires_at: Utc::now() + Duration::hours(1),
        };
        let store = Arc::new(RecordingStore::default());
        let client = make_client(&server, tokens, store.clone());

        let response = client
            .list_history("123", Some("p2"), Some(100))
            .await
            .expect("list history succeeds");

        assert!(response.history.is_empty());
        assert_eq!(response.history_id.as_deref(), Some("125"));
        let saved = store.saved.lock().await;
        assert!(saved.is_empty(), "tokens should not be refreshed");
    }

    #[tokio::test]
    async fn returns_decode_error_on_invalid_json() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/me/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
            .expect(1)
            .mount(&server)
            .await;

        let tokens = OAuthTokens {
            access_token: "token".into(),
            refresh_token: "refresh".into(),
            expires_at: Utc::now() + Duration::hours(1),
        };
        let store = Arc::new(RecordingStore::default());
        let client = make_client(&server, tokens, store);

        let err = client
            .list_messages(None, None, false, None)
            .await
            .expect_err("should surface decode error");

        assert!(matches!(err, GmailClientError::Decode(_)));
    }

    #[tokio::test]
    async fn surfaces_token_store_errors_on_refresh() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "new_token",
                "refresh_token": "refresh_two",
                "expires_in": 3600,
            })))
            .expect(1)
            .mount(&server)
            .await;

        let tokens = OAuthTokens {
            access_token: "old_token".into(),
            refresh_token: "refresh_one".into(),
            expires_at: Utc::now() - Duration::seconds(1),
        };
        let store = Arc::new(FailingStore);
        let client = GmailClient::new(
            reqwest::Client::new(),
            "me",
            "client",
            "secret",
            tokens,
            store,
        )
        .with_api_base(format!("{}/gmail/v1/users", server.uri()))
        .with_token_endpoint(format!("{}/token", server.uri()));

        let err = client
            .get_message("abc")
            .await
            .expect_err("token store failure surfaces");

        match err {
            GmailClientError::TokenStore(msg) => assert!(msg.contains("store failure")),
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[tokio::test]
    async fn get_profile_returns_history_id() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/me/profile"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "emailAddress": "test@example.com",
                "messagesTotal": 1234,
                "threadsTotal": 567,
                "historyId": "98765"
            })))
            .expect(1)
            .mount(&server)
            .await;

        let tokens = OAuthTokens {
            access_token: "token".into(),
            refresh_token: "refresh".into(),
            expires_at: Utc::now() + Duration::hours(1),
        };
        let store = Arc::new(RecordingStore::default());
        let client = make_client(&server, tokens, store);

        let profile = client.get_profile().await.expect("get_profile succeeds");

        assert_eq!(profile.email_address, "test@example.com");
        assert_eq!(profile.history_id, "98765");
        assert_eq!(profile.messages_total, Some(1234));
        assert_eq!(profile.threads_total, Some(567));
    }

    #[tokio::test]
    async fn list_labels_returns_all_labels() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/me/labels"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "labels": [
                    {
                        "id": "INBOX",
                        "name": "INBOX",
                        "type": "system",
                        "messageListVisibility": "show",
                        "labelListVisibility": "labelShow"
                    },
                    {
                        "id": "Label_123456789",
                        "name": "My Custom Label",
                        "type": "user",
                        "messageListVisibility": "show",
                        "labelListVisibility": "labelShow",
                        "color": {
                            "backgroundColor": "#ffffff",
                            "textColor": "#000000"
                        }
                    },
                    {
                        "id": "STARRED",
                        "name": "STARRED",
                        "type": "system"
                    }
                ]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let tokens = OAuthTokens {
            access_token: "token".into(),
            refresh_token: "refresh".into(),
            expires_at: Utc::now() + Duration::hours(1),
        };
        let store = Arc::new(RecordingStore::default());
        let client = make_client(&server, tokens, store);

        let response = client.list_labels().await.expect("list_labels succeeds");

        assert_eq!(response.labels.len(), 3);

        // Check system label
        let inbox = &response.labels[0];
        assert_eq!(inbox.id, "INBOX");
        assert_eq!(inbox.name, "INBOX");
        assert_eq!(inbox.label_type.as_deref(), Some("system"));
        assert_eq!(inbox.message_list_visibility.as_deref(), Some("show"));
        assert!(inbox.color.is_none());

        // Check user label with color
        let custom = &response.labels[1];
        assert_eq!(custom.id, "Label_123456789");
        assert_eq!(custom.name, "My Custom Label");
        assert_eq!(custom.label_type.as_deref(), Some("user"));
        let color = custom.color.as_ref().expect("should have color");
        assert_eq!(color.background_color.as_deref(), Some("#ffffff"));
        assert_eq!(color.text_color.as_deref(), Some("#000000"));

        // Check label without optional fields
        let starred = &response.labels[2];
        assert_eq!(starred.id, "STARRED");
        assert_eq!(starred.name, "STARRED");
        assert!(starred.message_list_visibility.is_none());
        assert!(starred.label_list_visibility.is_none());
    }

    #[tokio::test]
    async fn list_labels_handles_empty_response() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/me/labels"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "labels": []
            })))
            .expect(1)
            .mount(&server)
            .await;

        let tokens = OAuthTokens {
            access_token: "token".into(),
            refresh_token: "refresh".into(),
            expires_at: Utc::now() + Duration::hours(1),
        };
        let store = Arc::new(RecordingStore::default());
        let client = make_client(&server, tokens, store);

        let response = client.list_labels().await.expect("list_labels succeeds");

        assert!(response.labels.is_empty());
    }

    #[tokio::test]
    async fn list_labels_handles_minimal_label() {
        let server = MockServer::start().await;

        // A label with only required fields (id and name)
        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/me/labels"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "labels": [
                    {
                        "id": "Label_minimal",
                        "name": "Minimal Label"
                    }
                ]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let tokens = OAuthTokens {
            access_token: "token".into(),
            refresh_token: "refresh".into(),
            expires_at: Utc::now() + Duration::hours(1),
        };
        let store = Arc::new(RecordingStore::default());
        let client = make_client(&server, tokens, store);

        let response = client.list_labels().await.expect("list_labels succeeds");

        assert_eq!(response.labels.len(), 1);
        let label = &response.labels[0];
        assert_eq!(label.id, "Label_minimal");
        assert_eq!(label.name, "Minimal Label");
        assert!(label.label_type.is_none());
        assert!(label.message_list_visibility.is_none());
        assert!(label.label_list_visibility.is_none());
        assert!(label.color.is_none());
    }

    #[tokio::test]
    async fn list_labels_handles_label_with_partial_color() {
        let server = MockServer::start().await;

        // Color with only background, missing text color
        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/me/labels"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "labels": [
                    {
                        "id": "Label_partial_color",
                        "name": "Partial Color Label",
                        "color": {
                            "backgroundColor": "#ff0000"
                        }
                    }
                ]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let tokens = OAuthTokens {
            access_token: "token".into(),
            refresh_token: "refresh".into(),
            expires_at: Utc::now() + Duration::hours(1),
        };
        let store = Arc::new(RecordingStore::default());
        let client = make_client(&server, tokens, store);

        let response = client.list_labels().await.expect("list_labels succeeds");

        assert_eq!(response.labels.len(), 1);
        let label = &response.labels[0];
        let color = label.color.as_ref().expect("should have color");
        assert_eq!(color.background_color.as_deref(), Some("#ff0000"));
        assert!(color.text_color.is_none());
    }
}
