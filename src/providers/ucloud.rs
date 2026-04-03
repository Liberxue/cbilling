// Copyright 2025 OpenObserve Inc.

//! UCloud Billing Provider
//!
//! This module implements billing integration with UCloud using their REST API

use std::collections::BTreeMap;

use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha1::Digest;

use super::super::{BillingError, Result};

const UCLOUD_API_ENDPOINT: &str = "https://api.ucloud.cn";

/// UCloud Billing Client
pub struct UCloudBillingClient {
    public_key: String,
    private_key: String,
    project_id: String,
    http_client: Client,
}

/// UCloud billing API response.
#[derive(Debug, Serialize, Deserialize)]
pub struct UCloudBillResponse {
    #[serde(rename = "RetCode")]
    pub ret_code: i32,
    #[serde(rename = "Action")]
    pub action: Option<String>,
    #[serde(rename = "Message")]
    pub message: Option<String>,
    #[serde(rename = "TotalCount")]
    pub total_count: Option<i32>,
    #[serde(rename = "Items")]
    pub items: Option<Vec<UCloudBillItem>>,
}

/// A single bill line item from UCloud.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UCloudBillItem {
    #[serde(rename = "ResourceId")]
    pub resource_id: Option<String>,
    #[serde(rename = "ResourceType")]
    pub resource_type: Option<String>,
    #[serde(rename = "OrderNo")]
    pub order_no: Option<String>,
    #[serde(rename = "Amount", deserialize_with = "deserialize_amount", default)]
    pub amount: Option<f64>,
    #[serde(
        rename = "AmountReal",
        deserialize_with = "deserialize_amount",
        default
    )]
    pub amount_real: Option<f64>,
    #[serde(rename = "Region")]
    pub region: Option<String>,
    #[serde(rename = "ProductName")]
    pub product_name: Option<String>,
    #[serde(rename = "ProductCategory")]
    pub product_category: Option<String>,
    #[serde(rename = "ResourceExtId")]
    pub resource_ext_id: Option<String>,
}

/// Deserialize Amount which may be a string or number in UCloud API
fn deserialize_amount<'de, D>(deserializer: D) -> std::result::Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v = Value::deserialize(deserializer)?;
    match v {
        Value::Number(n) => Ok(n.as_f64()),
        Value::String(s) => Ok(s.parse::<f64>().ok()),
        Value::Null => Ok(None),
        _ => Ok(None),
    }
}

impl UCloudBillingClient {
    /// Creates a new UCloud billing client with the given credentials.
    pub fn new(public_key: String, private_key: String, project_id: String) -> Self {
        Self {
            public_key,
            private_key,
            project_id,
            http_client: Client::new(),
        }
    }

    /// Generate UCloud API signature (SHA-1 per UCloud spec)
    fn generate_signature(&self, params: &BTreeMap<String, String>) -> Result<String> {
        // Build canonical string: sorted key-value pairs concatenated, then private key
        let mut param_str = String::new();
        for (key, value) in params.iter() {
            param_str.push_str(key);
            param_str.push_str(value);
        }
        param_str.push_str(&self.private_key);

        let mut hasher = sha1::Sha1::new();
        hasher.update(param_str.as_bytes());
        let result = hasher.finalize();

        Ok(hex::encode(result))
    }

    /// Build common query parameters for UCloud API
    fn build_common_params(&self, action: &str) -> BTreeMap<String, String> {
        let mut params = BTreeMap::new();

        params.insert("Action".to_string(), action.to_string());
        params.insert("PublicKey".to_string(), self.public_key.clone());
        params.insert("ProjectId".to_string(), self.project_id.clone());

        params
    }

    /// Call UCloud API
    async fn call_api(&self, action: &str, mut params: BTreeMap<String, String>) -> Result<Value> {
        // Add common parameters
        let common_params = self.build_common_params(action);
        for (k, v) in common_params {
            params.insert(k, v);
        }

        // Generate signature
        let signature = self.generate_signature(&params)?;
        params.insert("Signature".to_string(), signature);

        // Build URL
        let query_string = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<String>>()
            .join("&");

        let url = format!("{}/?{}", UCLOUD_API_ENDPOINT, query_string);

        tracing::debug!("UCloud API request: {}", url);

        // Make request
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| BillingError::ServiceError(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BillingError::ServiceError(format!(
                "UCloud API error: {} - {}",
                status, body
            )));
        }

        let json_response = response
            .json::<Value>()
            .await
            .map_err(|e| BillingError::ServiceError(format!("Failed to parse response: {}", e)))?;

        Ok(json_response)
    }

    /// Query bill detail list using ListUBillDetail API
    ///
    /// `billing_cycle` should be in "YYYY-MM" format (e.g. "2026-03")
    pub async fn query_bill_list(
        &self,
        billing_cycle: &str,
        offset: Option<i32>,
        limit: Option<i32>,
    ) -> Result<UCloudBillResponse> {
        tracing::info!("Querying UCloud billing for cycle {}", billing_cycle);

        let mut params = BTreeMap::new();
        params.insert("BillingCycle".to_string(), billing_cycle.to_string());

        if let Some(off) = offset {
            params.insert("Offset".to_string(), off.to_string());
        }

        if let Some(lim) = limit {
            params.insert("Limit".to_string(), lim.to_string());
        }

        let response = self.call_api("ListUBillDetail", params).await?;

        tracing::debug!(
            "UCloud API response: {}",
            serde_json::to_string_pretty(&response).unwrap_or_default()
        );

        let result: UCloudBillResponse = serde_json::from_value(response.clone()).map_err(|e| {
            BillingError::ServiceError(format!(
                "Failed to parse UCloud bill response: {} | raw: {}",
                e, response
            ))
        })?;

        if result.ret_code != 0 {
            return Err(BillingError::ServiceError(format!(
                "UCloud API error: code={}, message={:?}",
                result.ret_code, result.message
            )));
        }

        Ok(result)
    }

    /// Test credentials
    pub async fn test_credentials(&self) -> Result<bool> {
        let now = Utc::now();
        let cycle = now.format("%Y-%m").to_string();

        match self.query_bill_list(&cycle, Some(0), Some(1)).await {
            Ok(response) => Ok(response.ret_code == 0),
            Err(_) => Ok(false),
        }
    }
}

// ── BillingProvider adapter ──────────────────────────────────────────────

use super::traits::{BillingProvider, RawBillItem};
use crate::service::CloudAccountConfig;

/// Adapter that implements [`BillingProvider`] for UCloud.
pub struct UCloudBillingAdapter {
    client: UCloudBillingClient,
}

impl UCloudBillingAdapter {
    /// Create an adapter from a [`CloudAccountConfig`].
    pub fn from_config(config: &CloudAccountConfig) -> Result<Self> {
        let public_key = config
            .public_key
            .clone()
            .ok_or_else(|| BillingError::ServiceError("Missing public_key".to_string()))?;
        let private_key = config
            .private_key
            .clone()
            .ok_or_else(|| BillingError::ServiceError("Missing private_key".to_string()))?;
        let project_id = config
            .project_id
            .clone()
            .ok_or_else(|| BillingError::ServiceError("Missing project_id".to_string()))?;
        Ok(Self {
            client: UCloudBillingClient::new(public_key, private_key, project_id),
        })
    }
}

impl BillingProvider for UCloudBillingAdapter {
    fn provider_name(&self) -> &'static str {
        "ucloud"
    }

    fn currency(&self) -> &'static str {
        "CNY"
    }

    async fn query_bill_items(&self, billing_cycle: &str) -> Result<Vec<RawBillItem>> {
        let limit = 100;
        let mut offset = 0;
        let mut items = Vec::new();

        loop {
            let response = self
                .client
                .query_bill_list(billing_cycle, Some(offset), Some(limit))
                .await?;

            let api_total = response.total_count.unwrap_or(0);

            if let Some(bill_items) = response.items {
                if bill_items.is_empty() {
                    break;
                }
                for item in &bill_items {
                    let cost = item.amount_real.or(item.amount).unwrap_or(0.0);
                    let name = item
                        .product_name
                        .clone()
                        .or_else(|| item.resource_type.clone())
                        .unwrap_or_else(|| "Unknown".to_string());
                    items.push(RawBillItem {
                        product_name: name.clone(),
                        product_code: name,
                        cost,
                        region: String::new(),
                        instance_id: String::new(),
                        usage: None,
                        unit: None,
                    });
                }
            } else {
                break;
            }

            offset += limit;
            if offset >= api_total {
                break;
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
        let client = UCloudBillingClient::new(
            "test_public_key".to_string(),
            "test_private_key".to_string(),
            "test_project_id".to_string(),
        );
        assert_eq!(client.public_key, "test_public_key");
        assert_eq!(client.project_id, "test_project_id");
    }
}
