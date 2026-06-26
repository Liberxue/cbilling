// Copyright 2025 OpenObserve Inc.
// SPDX-License-Identifier: AGPL-3.0

//! Vast.ai Billing Provider
//!
//! Implements billing integration with Vast.ai (<https://cloud.vast.ai>) using the
//! Charges API (`GET /api/v0/charges/`). Authentication uses a Bearer API key,
//! obtained from <https://cloud.vast.ai/manage-keys/>.

use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::super::{BillingError, Result};

const VASTAI_API_ENDPOINT: &str = "https://console.vast.ai/api/v0";

/// Vast.ai Billing Client
pub struct VastaiBillingClient {
    api_key: String,
    http_client: Client,
}

// ── Response types ──────────────────────────────────────────────────────

/// Top-level response of the Vast.ai charges endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct VastaiChargesResponse {
    #[serde(default)]
    pub success: bool,
    #[serde(default)]
    pub count: i64,
    #[serde(default)]
    pub total: i64,
    #[serde(default)]
    pub next_token: Option<String>,
    #[serde(default)]
    pub results: Vec<VastaiCharge>,
}

/// A single charge entry. Top-level entries (one per contract, e.g. an instance)
/// contain nested `items` broken down by resource kind (gpu, disk, bandwidth).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VastaiCharge {
    #[serde(default)]
    pub start: Option<f64>,
    #[serde(default)]
    pub end: Option<f64>,
    /// Charge category: top-level is `instance`/`volume`/`serverless`; nested
    /// items are `gpu`/`disk`/`bwd`/`bwu`.
    #[serde(rename = "type", default)]
    pub charge_type: Option<String>,
    /// Resource source identifier, e.g. `instance-12345678`.
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub amount: Option<f64>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub items: Vec<VastaiCharge>,
}

// ── Client implementation ───────────────────────────────────────────────

impl VastaiBillingClient {
    /// Create a new Vast.ai Billing Client using a Bearer API key.
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            http_client: Client::new(),
        }
    }

    async fn call_api(&self, url: &str) -> Result<VastaiChargesResponse> {
        let response = self
            .http_client
            .get(url)
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| BillingError::HttpError(format!("Vast.ai API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BillingError::ApiError(format!(
                "Vast.ai API error {}: {}",
                status, body
            )));
        }

        response.json::<VastaiChargesResponse>().await.map_err(|e| {
            BillingError::SerializationError(format!("Failed to parse Vast.ai response: {}", e))
        })
    }

    /// Fetch a single page of charges for a unix-second date range.
    pub async fn get_charges(
        &self,
        start_ts: i64,
        end_ts: i64,
        after_token: Option<&str>,
    ) -> Result<VastaiChargesResponse> {
        let filters =
            serde_json::json!({ "day": { "gte": start_ts, "lte": end_ts } }).to_string();
        let mut url = format!(
            "{}/charges/?select_filters={}&limit=500",
            VASTAI_API_ENDPOINT,
            urlencoding::encode(&filters)
        );
        if let Some(token) = after_token {
            url.push_str("&after_token=");
            url.push_str(&urlencoding::encode(token));
        }
        self.call_api(&url).await
    }

    /// Fetch all charges for a date range, following pagination.
    pub async fn get_all_charges(
        &self,
        start_ts: i64,
        end_ts: i64,
    ) -> Result<Vec<VastaiCharge>> {
        let mut all = Vec::new();
        let mut after_token: Option<String> = None;

        loop {
            let resp = self
                .get_charges(start_ts, end_ts, after_token.as_deref())
                .await?;

            if resp.results.is_empty() {
                break;
            }
            all.extend(resp.results);

            match resp.next_token {
                Some(token) if !token.is_empty() => after_token = Some(token),
                _ => break,
            }
        }

        Ok(all)
    }

    /// Test credentials by requesting a tiny window of charges.
    pub async fn test_credentials(&self) -> Result<bool> {
        // 0..1 is a valid (empty) window; a 2xx response means the key works.
        match self.get_charges(0, 1, None).await {
            Ok(resp) => Ok(resp.success),
            Err(_) => Ok(false),
        }
    }
}

// ── BillingProvider adapter ──────────────────────────────────────────────

use super::traits::{BillingProvider, RawBillItem};
use crate::service::CloudAccountConfig;

/// Map a Vast.ai charge `type` to a human-readable product name.
fn product_name_for(charge_type: &str) -> String {
    match charge_type {
        "gpu" => "GPU Compute".to_string(),
        "disk" => "Disk Storage".to_string(),
        "bwd" => "Bandwidth (Download)".to_string(),
        "bwu" => "Bandwidth (Upload)".to_string(),
        "instance" => "Instance".to_string(),
        "volume" => "Volume".to_string(),
        "serverless" => "Serverless".to_string(),
        other => other.to_string(),
    }
}

/// Convert a unix-seconds month boundary helper for a `YYYY-MM` billing cycle.
fn month_range(billing_cycle: &str) -> Result<(i64, i64)> {
    let mut parts = billing_cycle.split('-');
    let year: i32 = parts
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| BillingError::ServiceError(format!("Invalid month: {}", billing_cycle)))?;
    let month: u32 = parts
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| BillingError::ServiceError(format!("Invalid month: {}", billing_cycle)))?;

    let start = chrono::NaiveDate::from_ymd_opt(year, month, 1)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .ok_or_else(|| BillingError::ServiceError(format!("Invalid month: {}", billing_cycle)))?;
    let (ny, nm) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let next = chrono::NaiveDate::from_ymd_opt(ny, nm, 1)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .ok_or_else(|| BillingError::ServiceError(format!("Invalid month: {}", billing_cycle)))?;

    let start_ts = start.and_utc().timestamp();
    let end_ts = next.and_utc().timestamp() - 1;
    Ok((start_ts, end_ts))
}

/// Adapter that implements [`BillingProvider`] for Vast.ai.
pub struct VastaiBillingAdapter {
    client: VastaiBillingClient,
}

impl VastaiBillingAdapter {
    /// Create an adapter from a [`CloudAccountConfig`].
    pub fn from_config(config: &CloudAccountConfig) -> Result<Self> {
        // The API key may live in `secret_key` (token-style field) or, for
        // convenience, in `access_key_id`.
        let api_key = config
            .secret_key
            .clone()
            .or_else(|| config.access_key_id.clone())
            .ok_or_else(|| {
                BillingError::InvalidCredentials("Missing Vast.ai API key".to_string())
            })?;

        Ok(Self {
            client: VastaiBillingClient::new(api_key),
        })
    }
}

impl BillingProvider for VastaiBillingAdapter {
    fn provider_name(&self) -> &'static str {
        "vastai"
    }

    fn currency(&self) -> &'static str {
        "USD"
    }

    async fn query_bill_items(&self, billing_cycle: &str) -> Result<Vec<RawBillItem>> {
        let (start_ts, end_ts) = month_range(billing_cycle)?;
        let charges = self.client.get_all_charges(start_ts, end_ts).await?;

        let mut items = Vec::new();
        for charge in &charges {
            let instance_id = charge.source.clone().unwrap_or_default();

            if charge.items.is_empty() {
                // No breakdown -- record the contract-level charge directly.
                let ctype = charge.charge_type.clone().unwrap_or_else(|| "usage".into());
                items.push(RawBillItem {
                    product_code: ctype.clone(),
                    product_name: product_name_for(&ctype),
                    cost: charge.amount.unwrap_or(0.0),
                    region: String::new(),
                    instance_id,
                    usage: None,
                    unit: None,
                });
            } else {
                // Break the charge down by resource kind (gpu/disk/bandwidth).
                for item in &charge.items {
                    let ctype = item.charge_type.clone().unwrap_or_else(|| "usage".into());
                    items.push(RawBillItem {
                        product_code: ctype.clone(),
                        product_name: product_name_for(&ctype),
                        cost: item.amount.unwrap_or(0.0),
                        region: String::new(),
                        instance_id: instance_id.clone(),
                        usage: None,
                        unit: None,
                    });
                }
            }
        }

        Ok(items)
    }

    async fn test_credentials(&self) -> Result<bool> {
        self.client.test_credentials().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = VastaiBillingClient::new("test_key".to_string());
        assert_eq!(client.api_key, "test_key");
    }

    #[test]
    fn test_month_range() {
        // 2026 is not a leap year, so February spans 28 days.
        let (start, end) = month_range("2026-02").expect("range");
        assert!(end > start);
        assert_eq!(end - start, 28 * 24 * 3600 - 1);
    }

    #[test]
    fn test_month_range_december() {
        let (start, end) = month_range("2026-12").expect("range");
        assert!(end > start);
    }

    #[test]
    fn test_month_range_invalid() {
        assert!(month_range("not-a-month").is_err());
        assert!(month_range("2026-13").is_err());
    }

    #[test]
    fn test_product_name_mapping() {
        assert_eq!(product_name_for("gpu"), "GPU Compute");
        assert_eq!(product_name_for("disk"), "Disk Storage");
        assert_eq!(product_name_for("unknown"), "unknown");
    }

    #[test]
    fn test_adapter_from_config_missing_key() {
        let config = CloudAccountConfig {
            id: "x".into(),
            name: "x".into(),
            access_key_id: None,
            access_key_secret: None,
            secret_access_key: None,
            secret_id: None,
            secret_key: None,
            public_key: None,
            private_key: None,
            project_id: None,
            region: None,
            enabled: true,
        };
        assert!(VastaiBillingAdapter::from_config(&config).is_err());
    }
}
