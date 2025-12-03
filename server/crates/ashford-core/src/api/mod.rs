//! API types for web UI communication.
//!
//! This module contains types designed for API responses, including:
//! - Summary types that expose only the necessary fields (excluding sensitive data)
//! - Pagination wrapper types

pub mod types;

pub use types::{AccountSummary, LabelColors, LabelSummary, MessageSummary, PaginatedResponse};
