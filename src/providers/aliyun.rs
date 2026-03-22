// Copyright 2025 OpenObserve Inc.

//! Aliyun (Alibaba Cloud) Billing Provider
//!
//! This module implements billing integration with Aliyun using their REST API

use std::collections::BTreeMap;

use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::{BillingError, Result};

type HmacSha1 = Hmac<sha1::Sha1>;

const ALIYUN_BILLING_ENDPOINT: &str = "https://business.aliyuncs.com";

/// Aliyun Billing Client
pub struct AliyunBillingClient {
    access_key_id: String,
    access_key_secret: String,
    http_client: Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AliyunAccountBillResponse {
    #[serde(rename = "Success")]
    pub success: bool,
    #[serde(rename = "Code")]
    pub code: String,
    #[serde(rename = "Message")]
    pub message: String,
    #[serde(rename = "Data")]
    pub data: Option<AliyunAccountBillData>,
    #[serde(rename = "RequestId")]
    pub request_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AliyunAccountBillData {
    #[serde(rename = "BillingCycle")]
    pub billing_cycle: String,
    #[serde(rename = "AccountID")]
    pub account_id: String,
    #[serde(rename = "AccountName")]
    pub account_name: String,
    #[serde(rename = "TotalCount")]
    pub total_count: i32,
    #[serde(rename = "PageNum")]
    pub page_num: i32,
    #[serde(rename = "PageSize")]
    pub page_size: i32,
    #[serde(rename = "Items")]
    pub items: Option<AliyunBillItems>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AliyunBillItems {
    #[serde(rename = "Item")]
    pub item: Vec<AliyunBillItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AliyunBillItem {
    #[serde(rename = "BillingDate")]
    pub billing_date: Option<String>,
    #[serde(rename = "ProductCode")]
    pub product_code: Option<String>,
    #[serde(rename = "ProductName")]
    pub product_name: Option<String>,
    #[serde(rename = "SubscriptionType")]
    pub subscription_type: Option<String>,
    #[serde(rename = "PretaxAmount")]
    pub pretax_amount: Option<f64>,
    #[serde(rename = "PretaxGrossAmount")]
    pub pretax_gross_amount: Option<f64>,
    #[serde(rename = "Currency")]
    pub currency: Option<String>,
    #[serde(rename = "AdjustAmount")]
    pub adjust_amount: Option<f64>,
    #[serde(rename = "InvoiceDiscount")]
    pub invoice_discount: Option<f64>,
    #[serde(rename = "DeductedByCashCoupons")]
    pub deducted_by_cash_coupons: Option<f64>,
    #[serde(rename = "DeductedByCoupons")]
    pub deducted_by_coupons: Option<f64>,
    #[serde(rename = "DeductedByPrepaidCard")]
    pub deducted_by_prepaid_card: Option<f64>,
    #[serde(rename = "PaymentAmount")]
    pub payment_amount: Option<f64>,
    #[serde(rename = "OutstandingAmount")]
    pub outstanding_amount: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AliyunInstanceBillResponse {
    #[serde(rename = "Success")]
    pub success: bool,
    #[serde(rename = "Code")]
    pub code: String,
    #[serde(rename = "Message")]
    pub message: String,
    #[serde(rename = "Data")]
    pub data: Option<AliyunInstanceBillData>,
    #[serde(rename = "RequestId")]
    pub request_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AliyunInstanceBillData {
    #[serde(rename = "BillingCycle")]
    pub billing_cycle: String,
    #[serde(rename = "AccountID")]
    pub account_id: String,
    #[serde(rename = "TotalCount")]
    pub total_count: i32,
    #[serde(rename = "PageNum")]
    pub page_num: i32,
    #[serde(rename = "PageSize")]
    pub page_size: i32,
    #[serde(rename = "Items")]
    pub items: Option<AliyunInstanceBillItems>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AliyunInstanceBillItems {
    #[serde(rename = "Item")]
    pub item: Vec<AliyunInstanceBillItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AliyunInstanceBillItem {
    #[serde(rename = "InstanceID")]
    pub instance_id: Option<String>,
    #[serde(rename = "ProductCode")]
    pub product_code: Option<String>,
    #[serde(rename = "ProductName")]
    pub product_name: Option<String>,
    #[serde(rename = "Region")]
    pub region: Option<String>,
    #[serde(rename = "SubscriptionType")]
    pub subscription_type: Option<String>,
    #[serde(rename = "PretaxAmount")]
    pub pretax_amount: Option<f64>,
    #[serde(rename = "PretaxGrossAmount")]
    pub pretax_gross_amount: Option<f64>,
    #[serde(rename = "Currency")]
    pub currency: Option<String>,
    #[serde(rename = "PaymentAmount")]
    pub payment_amount: Option<f64>,
}

impl AliyunBillingClient {
    pub fn new(access_key_id: String, access_key_secret: String) -> Self {
        Self {
            access_key_id,
            access_key_secret,
            http_client: Client::new(),
        }
    }

    /// Generate Aliyun API signature (Signature v1)
    fn generate_signature(&self, string_to_sign: &str) -> Result<String> {
        use base64::Engine;
        let mut mac =
            HmacSha1::new_from_slice(format!("{}&", self.access_key_secret).as_bytes())
                .map_err(|e| BillingError::ServiceError(format!("Failed to create HMAC: {}", e)))?;

        mac.update(string_to_sign.as_bytes());
        let result = mac.finalize();
        Ok(base64::engine::general_purpose::STANDARD.encode(result.into_bytes()))
    }

    /// Build common query parameters for Aliyun API
    fn build_common_params(&self, action: &str) -> BTreeMap<String, String> {
        let mut params = BTreeMap::new();

        params.insert("AccessKeyId".to_string(), self.access_key_id.clone());
        params.insert("Action".to_string(), action.to_string());
        params.insert("Format".to_string(), "JSON".to_string());
        params.insert("SignatureMethod".to_string(), "HMAC-SHA1".to_string());
        // Generate nonce using timestamp to avoid uuid dependency
        params.insert(
            "SignatureNonce".to_string(),
            format!(
                "{}-{}",
                Utc::now().timestamp_nanos_opt().unwrap_or(0),
                std::process::id()
            ),
        );
        params.insert("SignatureVersion".to_string(), "1.0".to_string());
        params.insert(
            "Timestamp".to_string(),
            Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        );
        params.insert("Version".to_string(), "2017-12-14".to_string());

        params
    }

    /// Canonicalize query string for signature
    fn canonicalize_query_string(&self, params: &BTreeMap<String, String>) -> String {
        params
            .iter()
            .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
            .collect::<Vec<String>>()
            .join("&")
    }

    /// Call Aliyun API
    async fn call_api(&self, action: &str, mut params: BTreeMap<String, String>) -> Result<Value> {
        // Add common parameters
        let common_params = self.build_common_params(action);
        for (k, v) in common_params {
            params.insert(k, v);
        }

        // Build canonicalized query string
        let canonicalized_query_string = self.canonicalize_query_string(&params);

        // Build string to sign
        let string_to_sign = format!(
            "GET&{}&{}",
            percent_encode("/"),
            percent_encode(&canonicalized_query_string)
        );

        // Generate signature
        let signature = self.generate_signature(&string_to_sign)?;
        params.insert("Signature".to_string(), signature);

        // Build final URL
        let query_string = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<String>>()
            .join("&");

        let url = format!("{}/?{}", ALIYUN_BILLING_ENDPOINT, query_string);

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
                "Aliyun API error: {} - {}",
                status, body
            )));
        }

        let json_response = response
            .json::<Value>()
            .await
            .map_err(|e| BillingError::ServiceError(format!("Failed to parse response: {}", e)))?;

        Ok(json_response)
    }

    /// Query account bill
    pub async fn query_account_bill(
        &self,
        billing_cycle: &str,
        page_num: Option<i32>,
        page_size: Option<i32>,
    ) -> Result<AliyunAccountBillResponse> {
        tracing::info!("Querying Aliyun billing for cycle: {}", billing_cycle);

        let mut params = BTreeMap::new();
        params.insert("BillingCycle".to_string(), billing_cycle.to_string());

        if let Some(num) = page_num {
            params.insert("PageNum".to_string(), num.to_string());
        }

        if let Some(size) = page_size {
            params.insert("PageSize".to_string(), size.to_string());
        }

        let response = self.call_api("QueryAccountBill", params).await?;

        tracing::debug!(
            "Aliyun API response: {}",
            serde_json::to_string_pretty(&response).unwrap_or_default()
        );

        let result: AliyunAccountBillResponse =
            serde_json::from_value(response.clone()).map_err(|e| {
                BillingError::ServiceError(format!("Failed to parse account bill: {}", e))
            })?;

        tracing::info!(
            "Aliyun response - Success: {}, Code: {}, Has data: {}",
            result.success,
            result.code,
            result.data.is_some()
        );

        if let Some(ref data) = result.data {
            tracing::info!(
                "Aliyun billing data - Total count: {}, Items: {}",
                data.total_count,
                data.items.as_ref().map(|i| i.item.len()).unwrap_or(0)
            );

            if let Some(ref items) = data.items {
                if !items.item.is_empty() {
                    let first_item = &items.item[0];
                    tracing::debug!(
                        "Sample Aliyun item - ProductName: {:?}, ProductCode: {:?}, Amount: {:?}",
                        first_item.product_name,
                        first_item.product_code,
                        first_item.pretax_amount
                    );
                }
            }
        }

        Ok(result)
    }

    /// Query instance bill (实例账单明细)
    pub async fn query_instance_bill(
        &self,
        billing_cycle: &str,
        page_num: Option<i32>,
        page_size: Option<i32>,
        product_code: Option<&str>,
    ) -> Result<AliyunInstanceBillResponse> {
        let mut params = BTreeMap::new();
        params.insert("BillingCycle".to_string(), billing_cycle.to_string());

        if let Some(num) = page_num {
            params.insert("PageNum".to_string(), num.to_string());
        }

        if let Some(size) = page_size {
            params.insert("PageSize".to_string(), size.to_string());
        }

        if let Some(code) = product_code {
            params.insert("ProductCode".to_string(), code.to_string());
        }

        let response = self.call_api("QueryInstanceBill", params).await?;

        serde_json::from_value(response).map_err(|e| {
            BillingError::ServiceError(format!("Failed to parse instance bill: {}", e))
        })
    }

    /// Test credentials
    pub async fn test_credentials(&self) -> Result<bool> {
        // Use current month for testing
        let billing_cycle = Utc::now().format("%Y-%m").to_string();
        match self
            .query_account_bill(&billing_cycle, Some(1), Some(1))
            .await
        {
            Ok(response) => Ok(response.success),
            Err(_) => Ok(false),
        }
    }
}

/// Percent-encode string for URL (RFC 3986)
fn percent_encode(input: &str) -> String {
    urlencoding::encode(input)
        .replace("+", "%20")
        .replace("*", "%2A")
        .replace("%7E", "~")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = AliyunBillingClient::new(
            "test_id".to_string(),
            "test_secret".to_string(),
        );
        assert_eq!(client.access_key_id, "test_id");
    }

    #[test]
    fn test_percent_encode() {
        assert_eq!(percent_encode("hello world"), "hello%20world");
        assert_eq!(percent_encode("test&value=1"), "test%26value%3D1");
    }
}
