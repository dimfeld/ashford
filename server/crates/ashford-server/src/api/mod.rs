//! HTTP API handlers for the Ashford web UI.
//!
//! This module provides REST API endpoints for:
//! - Accounts listing
//! - Actions history and management
//! - Rules configuration (deterministic and LLM rules)
//! - Labels listing
//! - Settings (future)

pub mod accounts;
pub mod actions;
pub mod labels;
pub mod rules;

use axum::Router;

use crate::AppState;

/// Create the main API router with all endpoints mounted.
pub fn router(_state: AppState) -> Router<AppState> {
    Router::new()
        .nest("/accounts", accounts::router())
        .nest("/actions", actions::router())
        .nest("/labels", labels::router())
        .nest("/rules", rules::router())
}
