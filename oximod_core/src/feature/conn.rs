//! MongoDB client initialization and access.
//!
//! This module contains [`OxiClient`](client::OxiClient), OxiMod’s wrapper
//! around the MongoDB Rust driver client. It supports both globally initialized
//! and explicitly supplied client workflows.

/// MongoDB client initialization, global access, and explicit-client helpers.
pub mod client;
