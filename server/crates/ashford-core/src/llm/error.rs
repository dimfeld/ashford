use std::fmt;

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimitInfo {
    pub retry_after_ms: Option<u64>,
}

impl RateLimitInfo {
    pub fn new(retry_after_ms: Option<u64>) -> Self {
        Self { retry_after_ms }
    }
}

impl fmt::Display for RateLimitInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ms) = self.retry_after_ms {
            write!(f, " (retry after {}ms)", ms)
        } else {
            Ok(())
        }
    }
}

impl std::error::Error for RateLimitInfo {}

#[derive(Debug, Error)]
pub enum LLMError {
    #[error("rate limited{0}")]
    RateLimited(#[source] RateLimitInfo),
    #[error("authentication failed")]
    AuthenticationFailed,
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("server error: {0}")]
    ServerError(String),
    #[error("timeout")]
    Timeout,
    #[error("parse error: {0}")]
    ParseError(String),
    #[error("provider error: {0}")]
    ProviderError(String),
}

#[cfg(test)]
mod tests {
    use super::{LLMError, RateLimitInfo};

    #[test]
    fn display_messages_match_expected_format() {
        assert_eq!(
            LLMError::RateLimited(RateLimitInfo::new(None)).to_string(),
            "rate limited"
        );
        assert_eq!(
            LLMError::RateLimited(RateLimitInfo::new(Some(1500))).to_string(),
            "rate limited (retry after 1500ms)"
        );
        assert_eq!(
            LLMError::AuthenticationFailed.to_string(),
            "authentication failed"
        );
        assert_eq!(
            LLMError::InvalidRequest("bad payload".into()).to_string(),
            "invalid request: bad payload"
        );
        assert_eq!(
            LLMError::ServerError("500".into()).to_string(),
            "server error: 500"
        );
        assert_eq!(LLMError::Timeout.to_string(), "timeout");
        assert_eq!(
            LLMError::ParseError("json".into()).to_string(),
            "parse error: json"
        );
        assert_eq!(
            LLMError::ProviderError("transient".into()).to_string(),
            "provider error: transient"
        );
    }
}
