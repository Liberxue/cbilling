// Copyright 2025 OpenObserve Inc.

//! UCloud Billing Provider
//!
//! This module implements billing integration with UCloud using their REST API

use std::collections::BTreeMap;

use chrono::Utc;
use hmac::Hmac;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Sha256;

use super::super::{BillingError, Result};

type _HmacSha256 = Hmac<Sha256>;

const UCLOUD_API_ENDPOINT: &str = "https://api.ucloud.cn";

/// UCloud Billing Client
pub struct UCloudBillingClient {
    public_key: String,
    private_key: String,
    project_id: String,
    http_client: Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UCloudBillResponse {
    #[serde(rename = "RetCode")]
    pub ret_code: i32,
    #[serde(rename = "Action")]
    pub action: String,
    #[serde(rename = "Message")]
    pub message: Option<String>,
    #[serde(rename = "TotalCount")]
    pub total_count: Option<i32>,
    #[serde(rename = "Items")]
    pub items: Option<Vec<UCloudBillItem>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UCloudBillItem {
    #[serde(rename = "StartTime")]
    pub start_time: Option<i64>,
    #[serde(rename = "EndTime")]
    pub end_time: Option<i64>,
    #[serde(rename = "OrderNo")]
    pub order_no: Option<String>,
    #[serde(rename = "ResourceId")]
    pub resource_id: Option<String>,
    #[serde(rename = "ResourceType")]
    pub resource_type: Option<String>,
    #[serde(rename = "ChargeType")]
    pub charge_type: Option<String>,
    #[serde(rename = "Amount")]
    pub amount: Option<f64>,
    #[serde(rename = "ShowAmount")]
    pub show_amount: Option<String>,
    #[serde(rename = "Region")]
    pub region: Option<String>,
    #[serde(rename = "Zone")]
    pub zone: Option<String>,
    #[serde(rename = "ProductType")]
    pub product_type: Option<String>,
    #[serde(rename = "ResourceName")]
    pub resource_name: Option<String>,
}

impl UCloudBillingClient {
    pub fn new(public_key: String, private_key: String, project_id: String) -> Self {
        Self {
            public_key,
            private_key,
            project_id,
            http_client: Client::new(),
        }
    }

    /// Generate UCloud API signature
    fn generate_signature(&self, params: &BTreeMap<String, String>) -> Result<String> {
        // Build canonical query string
        let mut param_str = String::new();
        for (key, value) in params.iter() {
            param_str.push_str(key);
            param_str.push_str(value);
        }
        param_str.push_str(&self.private_key);

        // Calculate SHA256 hash
        use sha2::Digest;
        let mut hasher = sha2::Sha256::new();
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
        params.insert("Region".to_string(), "cn-bj2".to_string()); // Default region

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

    /// Query bill list
    pub async fn query_bill_list(
        &self,
        begin_time: i64,
        end_time: i64,
        offset: Option<i32>,
        limit: Option<i32>,
    ) -> Result<UCloudBillResponse> {
        tracing::info!(
            "Querying UCloud billing from {} to {}",
            begin_time,
            end_time
        );

        let mut params = BTreeMap::new();
        params.insert("BeginTime".to_string(), begin_time.to_string());
        params.insert("EndTime".to_string(), end_time.to_string());

        if let Some(off) = offset {
            params.insert("Offset".to_string(), off.to_string());
        }

        if let Some(lim) = limit {
            params.insert("Limit".to_string(), lim.to_string());
        }

        let response = self.call_api("GetBillDataFileUrl", params).await?;

        tracing::debug!(
            "UCloud API response: {}",
            serde_json::to_string_pretty(&response).unwrap_or_default()
        );

        let result: UCloudBillResponse = serde_json::from_value(response.clone()).map_err(|e| {
            BillingError::ServiceError(format!("Failed to parse UCloud bill response: {}", e))
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
        let now = Utc::now().timestamp();
        let yesterday = now - 86400;

        match self.query_bill_list(yesterday, now, Some(0), Some(1)).await {
            Ok(response) => Ok(response.ret_code == 0),
            Err(_) => Ok(false),
        }
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
