//! HTTP API handlers for the Ashford web UI.
//!
//! This module provides REST API endpoints for:
//! - Accounts listing
//! - Actions history and management
//! - Rules configuration (future)
//! - Settings (future)

pub mod accounts;
pub mod actions;

use axum::Router;

use crate::AppState;

/// Create the main API router with all endpoints mounted.
pub fn router(_state: AppState) -> Router<AppState> {
    Router::new()
        .nest("/accounts", accounts::router())
        .nest("/actions", actions::router())
}
