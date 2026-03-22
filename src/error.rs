// Copyright 2025 OpenObserve Inc.
// SPDX-License-Identifier: AGPL-3.0

//! Error types for cloud billing operations

use std::fmt;

/// Billing module error types
#[derive(Debug)]
pub enum BillingError {
    /// Configuration error
    ConfigError(String),
    /// Service connection error
    ServiceError(String),
    /// Account not found
    AccountNotFound(String),
    /// Bill not found
    BillNotFound(String),
    /// Invalid credentials
    InvalidCredentials(String),
    /// API error
    ApiError(String),
    /// Database error
    DatabaseError(String),
    /// Internal error
    InternalError(String),
    /// Serialization error
    SerializationError(String),
    /// HTTP request error
    HttpError(String),
}

impl fmt::Display for BillingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BillingError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            BillingError::ServiceError(msg) => write!(f, "Service error: {}", msg),
            BillingError::AccountNotFound(id) => write!(f, "Account not found: {}", id),
            BillingError::BillNotFound(id) => write!(f, "Bill not found: {}", id),
            BillingError::InvalidCredentials(msg) => write!(f, "Invalid credentials: {}", msg),
            BillingError::ApiError(msg) => write!(f, "API error: {}", msg),
            BillingError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            BillingError::InternalError(msg) => write!(f, "Internal error: {}", msg),
            BillingError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            BillingError::HttpError(msg) => write!(f, "HTTP error: {}", msg),
        }
    }
}

impl std::error::Error for BillingError {}

// Common error conversions
impl From<serde_json::Error> for BillingError {
    fn from(err: serde_json::Error) -> Self {
        BillingError::SerializationError(err.to_string())
    }
}

impl From<reqwest::Error> for BillingError {
    fn from(err: reqwest::Error) -> Self {
        BillingError::HttpError(err.to_string())
    }
}


/// Result type for billing operations
pub type Result<T> = std::result::Result<T, BillingError>;
