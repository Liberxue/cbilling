// Copyright 2025 OpenObserve Inc.

//! Tencent Cloud Billing Provider
//!
//! This module implements billing integration with Tencent Cloud using their REST API

use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::super::{BillingError, Result};

type HmacSha256 = Hmac<Sha256>;

const TENCENT_BILLING_HOST: &str = "billing.tencentcloudapi.com";
const TENCENT_BILLING_ENDPOINT: &str = "https://billing.tencentcloudapi.com";

/// Tencent Cloud Billing Client
pub struct TencentCloudBillingClient {
    secret_id: String,
    secret_key: String,
    region: String,
    http_client: Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TencentBillSummary {
    #[serde(rename = "Response")]
    pub response: TencentBillSummaryResponse,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TencentBillSummaryResponse {
    #[serde(rename = "Ready")]
    pub ready: Option<u8>,
    #[serde(rename = "SummaryTotal")]
    pub summary_total: Option<TencentSummaryTotal>,
    #[serde(rename = "SummaryOverview")]
    pub summary_overview: Option<Vec<TencentBillItem>>,
    #[serde(rename = "RequestId")]
    pub request_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TencentSummaryTotal {
    #[serde(rename = "RealTotalCost")]
    pub real_total_cost: String,
    #[serde(rename = "TotalCost")]
    pub total_cost: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TencentBillItem {
    #[serde(rename = "BusinessCode")]
    pub business_code: Option<String>,
    #[serde(rename = "BusinessCodeName")]
    pub business_code_name: Option<String>,
    #[serde(rename = "RealTotalCost")]
    pub real_total_cost: String,
    #[serde(rename = "TotalCost")]
    pub total_cost: String,
    #[serde(rename = "CashPayAmount")]
    pub cash_pay_amount: Option<String>,
    #[serde(rename = "VoucherPayAmount")]
    pub voucher_pay_amount: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TencentBillDetail {
    #[serde(rename = "Response")]
    pub response: TencentBillDetailResponse,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TencentBillDetailResponse {
    #[serde(rename = "DetailSet")]
    pub detail_set: Option<Vec<TencentBillDetailItem>>,
    #[serde(rename = "Total")]
    pub total: Option<u64>,
    #[serde(rename = "RequestId")]
    pub request_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TencentBillDetailItem {
    #[serde(rename = "BusinessCode")]
    pub business_code: Option<String>,
    #[serde(rename = "ProductCode")]
    pub product_code: Option<String>,
    #[serde(rename = "PayMode")]
    pub pay_mode: Option<String>,
    #[serde(rename = "ProjectName")]
    pub project_name: Option<String>,
    #[serde(rename = "RegionName")]
    pub region_name: Option<String>,
    #[serde(rename = "RealTotalCost")]
    pub real_total_cost: Option<String>,
    #[serde(rename = "CashPayAmount")]
    pub cash_pay_amount: Option<String>,
    #[serde(rename = "PayTime")]
    pub pay_time: Option<String>,
}

impl TencentCloudBillingClient {
    pub fn new(secret_id: String, secret_key: String, region: String) -> Self {
        Self {
            secret_id,
            secret_key,
            region,
            http_client: Client::new(),
        }
    }

    /// Generate Tencent Cloud API v3 signature
    fn generate_signature(
        &self,
        _action: &str,
        params: &Value,
        timestamp: i64,
        _nonce: u32,
    ) -> Result<String> {
        // Build canonical request
        let canonical_headers = format!(
            "content-type:application/json\nhost:{}\n",
            TENCENT_BILLING_HOST
        );
        let signed_headers = "content-type;host";
        let payload = serde_json::to_string(params).map_err(|e| {
            BillingError::ServiceError(format!("Failed to serialize params: {}", e))
        })?;

        let hashed_payload = hex::encode(Sha256::digest(payload.as_bytes()));

        let canonical_request = format!(
            "POST\n/\n\n{}\n{}\n{}",
            canonical_headers, signed_headers, hashed_payload
        );

        let hashed_canonical_request = hex::encode(Sha256::digest(canonical_request.as_bytes()));

        // Build string to sign
        let date = DateTime::from_timestamp(timestamp, 0)
            .ok_or_else(|| BillingError::ServiceError("Invalid timestamp".to_string()))?
            .format("%Y-%m-%d")
            .to_string();
        let credential_scope = format!("{}/billing/tc3_request", date);
        let string_to_sign = format!(
            "TC3-HMAC-SHA256\n{}\n{}\n{}",
            timestamp, credential_scope, hashed_canonical_request
        );

        // Calculate signature
        let secret_date = self.hmac_sha256(
            format!("TC3{}", self.secret_key).as_bytes(),
            date.as_bytes(),
        );
        let secret_service = self.hmac_sha256(&secret_date, b"billing");
        let secret_signing = self.hmac_sha256(&secret_service, b"tc3_request");
        let signature = hex::encode(self.hmac_sha256(&secret_signing, string_to_sign.as_bytes()));

        Ok(signature)
    }

    fn hmac_sha256(&self, key: &[u8], data: &[u8]) -> Vec<u8> {
        let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
        mac.update(data);
        mac.finalize().into_bytes().to_vec()
    }

    /// Call Tencent Cloud API
    async fn call_api(&self, action: &str, params: Value) -> Result<Value> {
        let timestamp = Utc::now().timestamp();
        // Generate nonce using timestamp to avoid rand dependency
        let nonce = (Utc::now().timestamp_nanos_opt().unwrap_or(0) % u32::MAX as i64) as u32;

        let signature = self.generate_signature(action, &params, timestamp, nonce)?;

        let date = DateTime::from_timestamp(timestamp, 0)
            .ok_or_else(|| BillingError::ServiceError("Invalid timestamp".to_string()))?
            .format("%Y-%m-%d")
            .to_string();
        let credential_scope = format!("{}/billing/tc3_request", date);
        let authorization = format!(
            "TC3-HMAC-SHA256 Credential={}/{}, SignedHeaders=content-type;host, Signature={}",
            self.secret_id, credential_scope, signature
        );

        let response = self
            .http_client
            .post(TENCENT_BILLING_ENDPOINT)
            .header("Content-Type", "application/json")
            .header("Host", TENCENT_BILLING_HOST)
            .header("X-TC-Action", action)
            .header("X-TC-Timestamp", timestamp.to_string())
            .header("X-TC-Version", "2018-07-09")
            .header("X-TC-Region", &self.region)
            .header("Authorization", authorization)
            .json(&params)
            .send()
            .await
            .map_err(|e| BillingError::ServiceError(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BillingError::ServiceError(format!(
                "Tencent Cloud API error: {} - {}",
                status, body
            )));
        }

        let json_response = response
            .json::<Value>()
            .await
            .map_err(|e| BillingError::ServiceError(format!("Failed to parse response: {}", e)))?;

        Ok(json_response)
    }

    /// Get bill summary by product
    pub async fn get_bill_summary(&self, month: &str) -> Result<TencentBillSummary> {
        tracing::info!("Querying Tencent Cloud billing for month: {}", month);

        // 根据官方文档，DescribeBillSummaryByProduct 需要 BeginTime 和 EndTime
        // 格式为 YYYY-MM，必须相同月份
        let params = json!({
            "BeginTime": month,
            "EndTime": month,
        });

        tracing::debug!(
            "Tencent Cloud API params: {}",
            serde_json::to_string(&params).unwrap_or_default()
        );

        let response = self
            .call_api("DescribeBillSummaryByProduct", params)
            .await?;

        tracing::debug!(
            "Tencent Cloud API response: {}",
            serde_json::to_string_pretty(&response).unwrap_or_default()
        );

        let result: TencentBillSummary = serde_json::from_value(response.clone()).map_err(|e| {
            BillingError::ServiceError(format!("Failed to parse bill summary: {}", e))
        })?;

        tracing::info!(
            "Tencent Cloud response - Ready: {:?}, Has SummaryTotal: {}, Has SummaryOverview: {}, Overview items: {}",
            result.response.ready,
            result.response.summary_total.is_some(),
            result.response.summary_overview.is_some(),
            result
                .response
                .summary_overview
                .as_ref()
                .map(|v| v.len())
                .unwrap_or(0)
        );

        if let Some(ref total) = result.response.summary_total {
            tracing::info!("Tencent Cloud total cost: {}", total.real_total_cost);
        }

        Ok(result)
    }

    /// Get detailed bill list
    pub async fn get_bill_detail(
        &self,
        month: &str,
        offset: u64,
        limit: u64,
    ) -> Result<TencentBillDetail> {
        let params = json!({
            "Month": month,
            "Offset": offset,
            "Limit": limit,
        });

        let response = self.call_api("DescribeBillDetail", params).await?;

        serde_json::from_value(response)
            .map_err(|e| BillingError::ServiceError(format!("Failed to parse bill detail: {}", e)))
    }

    /// Get account balance
    pub async fn get_account_balance(&self) -> Result<Value> {
        self.call_api("DescribeAccountBalance", json!({})).await
    }

    /// Test credentials
    pub async fn test_credentials(&self) -> Result<bool> {
        match self.get_account_balance().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = TencentCloudBillingClient::new(
            "test_id".to_string(),
            "test_key".to_string(),
            "ap-guangzhou".to_string(),
        );
        assert_eq!(client.secret_id, "test_id");
        assert_eq!(client.region, "ap-guangzhou");
    }
}
