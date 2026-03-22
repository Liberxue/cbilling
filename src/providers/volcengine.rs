// Copyright 2025 OpenObserve Inc.

//! Volcengine (火山云) Billing Provider
//!
//! This module implements billing integration with Volcengine using their REST API

use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::super::{BillingError, Result};

type HmacSha256 = Hmac<Sha256>;

const VOLCENGINE_BILLING_HOST: &str = "billing.volcengineapi.com";
const VOLCENGINE_BILLING_ENDPOINT: &str = "https://billing.volcengineapi.com";
const VOLCENGINE_SERVICE: &str = "billing";

/// Volcengine Billing Client
pub struct VolcengineBillingClient {
    access_key_id: String,
    secret_access_key: String,
    region: String,
    http_client: Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VolcengineBillDetailResponse {
    #[serde(rename = "Result")]
    pub result: Option<VolcengineBillDetailResult>,
    #[serde(rename = "ResponseMetadata")]
    pub response_metadata: VolcengineResponseMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VolcengineResponseMetadata {
    #[serde(rename = "RequestId")]
    pub request_id: String,
    #[serde(rename = "Action")]
    pub action: String,
    #[serde(rename = "Version")]
    pub version: String,
    #[serde(rename = "Service")]
    pub service: String,
    #[serde(rename = "Region")]
    pub region: String,
    #[serde(rename = "Error")]
    pub error: Option<VolcengineError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VolcengineError {
    #[serde(rename = "Code")]
    pub code: String,
    #[serde(rename = "Message")]
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VolcengineBillDetailResult {
    #[serde(rename = "List")]
    pub list: Option<Vec<VolcengineBillItem>>,
    #[serde(rename = "Total")]
    pub total: Option<i64>,
    #[serde(rename = "Limit")]
    pub limit: Option<i32>,
    #[serde(rename = "Offset")]
    pub offset: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VolcengineBillItem {
    #[serde(rename = "BillID")]
    pub bill_id: Option<String>,
    #[serde(rename = "OwnerID")]
    pub owner_id: Option<String>,
    #[serde(rename = "OwnerUserName")]
    pub owner_user_name: Option<String>,
    #[serde(rename = "BillPeriod")]
    pub bill_period: Option<String>,
    #[serde(rename = "Product")]
    pub product: Option<String>,
    #[serde(rename = "ProductZh")]
    pub product_zh: Option<String>,
    #[serde(rename = "BillingMode")]
    pub billing_mode: Option<String>,
    #[serde(rename = "ExpenseTime")]
    pub expense_time: Option<String>,
    #[serde(rename = "Region")]
    pub region: Option<String>,
    #[serde(rename = "Zone")]
    pub zone: Option<String>,
    #[serde(rename = "InstanceID")]
    pub instance_id: Option<String>,
    #[serde(rename = "InstanceName")]
    pub instance_name: Option<String>,
    #[serde(rename = "ConfigName")]
    pub config_name: Option<String>,
    #[serde(rename = "BillCategory")]
    pub bill_category: Option<String>,
    #[serde(rename = "BillCategoryParent")]
    pub bill_category_parent: Option<String>,
    #[serde(rename = "OriginalBillAmount")]
    pub original_bill_amount: Option<String>,
    #[serde(rename = "DiscountBillAmount")]
    pub discount_bill_amount: Option<String>,
    #[serde(rename = "CouponAmount")]
    pub coupon_amount: Option<String>,
    #[serde(rename = "PayableAmount")]
    pub payable_amount: Option<String>,
    #[serde(rename = "PaidAmount")]
    pub paid_amount: Option<String>,
    #[serde(rename = "UnpaidAmount")]
    pub unpaid_amount: Option<String>,
    #[serde(rename = "Currency")]
    pub currency: Option<String>,
}

impl VolcengineBillingClient {
    pub fn new(access_key_id: String, secret_access_key: String, region: String) -> Self {
        Self {
            access_key_id,
            secret_access_key,
            region,
            http_client: Client::new(),
        }
    }

    /// Generate Volcengine API signature (HMAC-SHA256)
    fn generate_signature(
        &self,
        canonical_request: &str,
        date_short: &str,
        date_time: &str,
        region: &str,
    ) -> Result<String> {
        // Build credential scope
        let credential_scope = format!("{}/{}/{}/request", date_short, region, VOLCENGINE_SERVICE);

        // Build string to sign
        let hashed_canonical_request = hex::encode(Sha256::digest(canonical_request.as_bytes()));
        let string_to_sign = format!(
            "HMAC-SHA256\n{}\n{}\n{}",
            date_time, credential_scope, hashed_canonical_request
        );

        // Calculate signing key - following AWS Signature Version 4 algorithm
        let k_date = self.hmac_sha256(self.secret_access_key.as_bytes(), date_short.as_bytes());
        let k_region = self.hmac_sha256(&k_date, region.as_bytes());
        let k_service = self.hmac_sha256(&k_region, VOLCENGINE_SERVICE.as_bytes());
        let k_signing = self.hmac_sha256(&k_service, b"request");

        // Calculate signature
        let signature = hex::encode(self.hmac_sha256(&k_signing, string_to_sign.as_bytes()));

        Ok(signature)
    }

    fn hmac_sha256(&self, key: &[u8], data: &[u8]) -> Vec<u8> {
        let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
        mac.update(data);
        mac.finalize().into_bytes().to_vec()
    }

    /// Build canonical request for signature
    fn build_canonical_request(
        &self,
        http_method: &str,
        canonical_uri: &str,
        canonical_query_string: &str,
        canonical_headers: &str,
        signed_headers: &str,
        hashed_payload: &str,
    ) -> String {
        format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            http_method,
            canonical_uri,
            canonical_query_string,
            canonical_headers,
            signed_headers,
            hashed_payload
        )
    }

    /// Call Volcengine API
    async fn call_api(&self, action: &str, params: Value) -> Result<Value> {
        let now = Utc::now();
        let x_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date_short = now.format("%Y%m%d").to_string();

        // Prepare request body
        let body = serde_json::to_string(&params).map_err(|e| {
            BillingError::ServiceError(format!("Failed to serialize params: {}", e))
        })?;

        // Calculate content hash
        let content_hash = hex::encode(Sha256::digest(body.as_bytes()));

        // Build canonical headers
        let canonical_headers = format!(
            "host:{}\nx-content-sha256:{}\nx-date:{}\n",
            VOLCENGINE_BILLING_HOST, content_hash, x_date
        );
        let signed_headers = "host;x-content-sha256;x-date";

        // Build canonical query string (for POST with Action and Version)
        let canonical_query_string = format!("Action={}&Version=2022-01-01", action);

        // Build canonical request
        let canonical_request = self.build_canonical_request(
            "POST",
            "/",
            &canonical_query_string,
            &canonical_headers,
            signed_headers,
            &content_hash,
        );

        // Generate signature
        let signature =
            self.generate_signature(&canonical_request, &date_short, &x_date, &self.region)?;

        // Build authorization header
        let credential_scope = format!(
            "{}/{}/{}/request",
            date_short, self.region, VOLCENGINE_SERVICE
        );
        let authorization = format!(
            "HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            self.access_key_id, credential_scope, signed_headers, signature
        );

        // Build full URL with query parameters
        let url = format!(
            "{}/?Action={}&Version=2022-01-01",
            VOLCENGINE_BILLING_ENDPOINT, action
        );

        tracing::debug!("Volcengine API Request URL: {}", url);
        tracing::debug!("Volcengine API Request Body: {}", body);
        tracing::debug!("Volcengine Canonical Request:\n{}", canonical_request);
        tracing::debug!("Volcengine Authorization: {}", authorization);

        // Make request
        let response = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Host", VOLCENGINE_BILLING_HOST)
            .header("X-Date", &x_date)
            .header("X-Content-Sha256", &content_hash)
            .header("Authorization", authorization)
            .body(body)
            .send()
            .await
            .map_err(|e| BillingError::ServiceError(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BillingError::ServiceError(format!(
                "Volcengine API error: {} - {}",
                status, body
            )));
        }

        let json_response = response
            .json::<Value>()
            .await
            .map_err(|e| BillingError::ServiceError(format!("Failed to parse response: {}", e)))?;

        tracing::debug!(
            "Volcengine API Response: {}",
            serde_json::to_string_pretty(&json_response).unwrap_or_default()
        );

        Ok(json_response)
    }

    /// List bill details
    pub async fn list_bill_detail(
        &self,
        bill_period: &str,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<VolcengineBillDetailResponse> {
        tracing::info!("Querying Volcengine billing for period: {}", bill_period);

        let mut params = json!({
            "BillPeriod": bill_period,
        });

        if let Some(l) = limit {
            params["Limit"] = json!(l);
        }

        if let Some(o) = offset {
            params["Offset"] = json!(o);
        }

        let response = self.call_api("ListBillDetail", params).await?;

        let result: VolcengineBillDetailResponse = serde_json::from_value(response.clone())
            .map_err(|e| {
                BillingError::ServiceError(format!("Failed to parse bill detail: {}", e))
            })?;

        // Check for API errors
        if let Some(ref error) = result.response_metadata.error {
            return Err(BillingError::ServiceError(format!(
                "Volcengine API error: {} - {}",
                error.code, error.message
            )));
        }

        tracing::info!(
            "Volcengine response - RequestId: {}, Has result: {}",
            result.response_metadata.request_id,
            result.result.is_some()
        );

        if let Some(ref data) = result.result {
            tracing::info!(
                "Volcengine billing data - Total: {:?}, Items: {}",
                data.total,
                data.list.as_ref().map(|l| l.len()).unwrap_or(0)
            );
        }

        Ok(result)
    }

    /// Test credentials
    pub async fn test_credentials(&self) -> Result<bool> {
        // Use current month for testing
        let bill_period = Utc::now().format("%Y-%m").to_string();
        match self.list_bill_detail(&bill_period, Some(1), Some(0)).await {
            Ok(response) => Ok(response.result.is_some()),
            Err(_) => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = VolcengineBillingClient::new(
            "test_id".to_string(),
            "test_secret".to_string(),
            "cn-beijing".to_string(),
        );
        assert_eq!(client.access_key_id, "test_id");
        assert_eq!(client.region, "cn-beijing");
    }
}
