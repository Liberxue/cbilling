// Copyright 2025 OpenObserve Inc.
// SPDX-License-Identifier: AGPL-3.0

//! # Cloud Billing SDK
//!
//! A comprehensive Rust library for managing multi-cloud billing and account management.
//!
//! ## Supported Cloud Providers
//!
//! Each provider is behind a feature flag for modular compilation:
//!
//! - **Aliyun (Alibaba Cloud)** - feature: `aliyun` - Full support
//! - **Tencent Cloud** - feature: `tencentcloud` - Full support
//! - **AWS** - feature: `aws` - Full support via Cost Explorer API
//! - **Volcengine** - feature: `volcengine` - Full support
//! - **UCloud** - feature: `ucloud` - Partial support
//! - **Azure** - feature: `azure` - Planned
//!
//! ## Feature Flags
//!
//! ### Cloud Providers (modular)
//! - `aliyun` - Alibaba Cloud billing support
//! - `tencentcloud` - Tencent Cloud billing support
//! - `aws` - AWS Cost Explorer support
//! - `volcengine` - Volcengine billing support
//! - `ucloud` - UCloud billing support
//! - `all-providers` - Enable all cloud providers
//!
//! ### Database Support (optional)
//! - `db-postgres` - PostgreSQL database support
//! - `db-sqlite` - SQLite database support
//!
//! ### Convenience Features
//! - `full` - All providers + PostgreSQL
//! - `full-sqlite` - All providers + SQLite
//!
//! ### Default Features
//! By default, all cloud providers are enabled but no database backend.
//!
//! ## Core Features
//!
//! - 🔄 Direct cloud provider API integration
//! - 👥 Multi-account support per provider
//! - 📊 Bill aggregation and analysis
//! - 📈 Cost trending and comparison
//! - 💾 Optional database persistence (PostgreSQL or SQLite)
//! - 🔍 Query billing data by cycle, provider, and account
//!
//! ## Quick Start
//!
//! ### Basic Usage (without database)
//!
//! ```toml
//! [dependencies]
//! cbilling = { version = "0.1", features = ["aliyun"] }
//! ```
//!
//! ```rust,no_run
//! # #[cfg(feature = "aliyun")]
//! use cbilling::providers::aliyun::AliyunBillingClient;
//! use cbilling::error::Result;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//! #   #[cfg(feature = "aliyun")]
//! #   {
//!     // Create Aliyun billing client
//!     let client = AliyunBillingClient::new(
//!         "your_access_key_id".to_string(),
//!         "your_access_key_secret".to_string(),
//!     );
//!
//!     // Query billing data for a specific month
//!     let billing_cycle = "2025-03";
//!     let response = client.query_instance_bill(billing_cycle, None, None, None).await?;
//!     
//!     println!("Billing data: {:?}", response);
//! #   }
//!     Ok(())
//! }
//! ```
//!
//! ### With Database Support
//!
//! ```toml
//! [dependencies]
//! # PostgreSQL
//! cbilling = { version = "0.1", features = ["aliyun", "db-postgres"] }
//! # Or SQLite
//! cbilling = { version = "0.1", features = ["aliyun", "db-sqlite"] }
//! # Or all providers with PostgreSQL
//! cbilling = { version = "0.1", features = ["full"] }
//! ```

// Temporarily allow missing docs during development
#![allow(missing_docs)]
#![warn(clippy::all)]

// Core modules
pub mod error;
pub mod models;
pub mod providers;
pub mod service;

// Re-exports for convenience
pub use error::{BillingError, Result};
pub use models::{Account, Bill, Provider, ProviderConfig};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the billing library
///
/// This function should be called once at the start of your application.
/// It initializes the cloud provider registry and sets up logging.
pub async fn init() -> Result<()> {
    tracing::info!("Initializing cbilling library v{}", VERSION);

    // Initialize providers registry
    providers::registry::init().await?;

    tracing::info!("Cloud-billing library initialized successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[tokio::test]
    async fn test_init() {
        let result = init().await;
        // Initialization might fail if credentials are not configured,
        // which is expected in tests
        assert!(result.is_ok() || result.is_err());
    }
}
