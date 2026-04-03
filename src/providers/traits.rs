// Copyright 2025 OpenObserve Inc.
// SPDX-License-Identifier: AGPL-3.0

//! Billing Provider Trait
//!
//! Defines the core abstraction every cloud billing provider implements.

use crate::error::Result;

/// A single normalized bill line item. Every provider maps its raw API items into this.
#[derive(Debug, Clone)]
pub struct RawBillItem {
    /// Product code / identifier
    pub product_code: String,
    /// Human-readable product name
    pub product_name: String,
    /// Cost amount
    pub cost: f64,
    /// Region where the resource is deployed (empty if unknown)
    pub region: String,
    /// Instance/resource identifier (empty if unknown)
    pub instance_id: String,
    /// Usage quantity
    pub usage: Option<f64>,
    /// Usage unit
    pub unit: Option<String>,
}

/// Core abstraction every cloud billing provider implements.
pub trait BillingProvider: Send + Sync {
    /// Provider identifier (e.g. "aliyun")
    fn provider_name(&self) -> &'static str;
    /// Default currency for this provider
    fn currency(&self) -> &'static str;
    /// Fetch all bill items for a billing cycle. Handles pagination internally.
    fn query_bill_items(
        &self,
        billing_cycle: &str,
    ) -> impl std::future::Future<Output = Result<Vec<RawBillItem>>> + Send;
    /// Test credentials.
    fn test_credentials(&self) -> impl std::future::Future<Output = Result<bool>> + Send;
}
