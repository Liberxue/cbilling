// Copyright 2025 OpenObserve Inc.

//! Cloudflare Billing Provider
//!
//! Implements billing integration with Cloudflare using the Billing API v4.
//! Authentication uses API Token (recommended) or API Key + Email.

use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::super::{BillingError, Result};

const CF_API_ENDPOINT: &str = "https://api.cloudflare.com/client/v4";

/// Cloudflare Billing Client
pub struct CloudflareBillingClient {
    account_id: String,
    api_token: Option<String>,
    api_key: Option<String>,
    api_email: Option<String>,
    http_client: Client,
}

// ── Response types ──────────────────────────────────────────────────────

/// Generic Cloudflare API v4 response wrapper.
#[derive(Debug, Serialize, Deserialize)]
pub struct CfApiResponse<T> {
    pub success: bool,
    pub errors: Vec<CfApiError>,
    pub messages: Vec<serde_json::Value>,
    pub result: Option<T>,
    pub result_info: Option<CfResultInfo>,
}

/// Cloudflare API error detail.
#[derive(Debug, Serialize, Deserialize)]
pub struct CfApiError {
    pub code: i32,
    pub message: String,
}

/// Pagination metadata from the Cloudflare API.
#[derive(Debug, Serialize, Deserialize)]
pub struct CfResultInfo {
    pub page: Option<i32>,
    pub per_page: Option<i32>,
    pub total_pages: Option<i32>,
    pub count: Option<i32>,
    pub total_count: Option<i32>,
}

/// Cloudflare account billing profile.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CfBillingProfile {
    pub id: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub currency: Option<String>,
    pub payment_email: Option<String>,
}

/// A single billing history entry from Cloudflare.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CfBillingHistory {
    pub id: String,
    #[serde(rename = "type")]
    pub bill_type: Option<String>,
    pub action: Option<String>,
    pub description: Option<String>,
    pub occurred_at: Option<String>,
    pub amount: Option<f64>,
    pub currency: Option<String>,
    pub zone: Option<CfBillingZone>,
}

/// Zone reference within a Cloudflare billing entry.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CfBillingZone {
    pub name: Option<String>,
}

/// Cloudflare subscription details.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CfSubscription {
    pub id: Option<String>,
    pub state: Option<String>,
    pub price: Option<f64>,
    pub currency: Option<String>,
    pub component_values: Option<Vec<CfComponentValue>>,
    pub zone: Option<CfSubscriptionZone>,
    pub rate_plan: Option<CfRatePlan>,
    pub frequency: Option<String>,
}

/// A price component within a Cloudflare subscription.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CfComponentValue {
    pub name: Option<String>,
    pub value: Option<f64>,
    pub price: Option<f64>,
    pub default: Option<f64>,
}

/// Zone associated with a Cloudflare subscription.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CfSubscriptionZone {
    pub id: Option<String>,
    pub name: Option<String>,
}

/// Rate plan metadata for a Cloudflare subscription.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CfRatePlan {
    pub id: Option<String>,
    pub public_name: Option<String>,
    pub currency: Option<String>,
    pub scope: Option<String>,
    pub externally_managed: Option<bool>,
}

// ── Client implementation ───────────────────────────────────────────────

impl CloudflareBillingClient {
    /// Create a new Cloudflare Billing Client using API Token (recommended)
    pub fn new_with_token(account_id: String, api_token: String) -> Self {
        Self {
            account_id,
            api_token: Some(api_token),
            api_key: None,
            api_email: None,
            http_client: Client::new(),
        }
    }

    /// Create a new Cloudflare Billing Client using API Key + Email
    pub fn new_with_key(account_id: String, api_key: String, api_email: String) -> Self {
        Self {
            account_id,
            api_token: None,
            api_key: Some(api_key),
            api_email: Some(api_email),
            http_client: Client::new(),
        }
    }

    fn auth_headers(&self) -> Vec<(&str, String)> {
        if let Some(ref token) = self.api_token {
            vec![("Authorization", format!("Bearer {}", token))]
        } else if let (Some(ref key), Some(ref email)) = (&self.api_key, &self.api_email) {
            vec![("X-Auth-Key", key.clone()), ("X-Auth-Email", email.clone())]
        } else {
            vec![]
        }
    }

    async fn call_api<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
    ) -> Result<CfApiResponse<T>> {
        let mut req = self
            .http_client
            .get(url)
            .header("Content-Type", "application/json");

        for (key, value) in self.auth_headers() {
            req = req.header(key, value);
        }

        let response = req.send().await.map_err(|e| {
            BillingError::HttpError(format!("Cloudflare API request failed: {}", e))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BillingError::ApiError(format!(
                "Cloudflare API error {}: {}",
                status, body
            )));
        }

        let api_response: CfApiResponse<T> = response.json().await.map_err(|e| {
            BillingError::SerializationError(format!("Failed to parse response: {}", e))
        })?;

        if !api_response.success {
            let err_msg = api_response
                .errors
                .iter()
                .map(|e| format!("{}: {}", e.code, e.message))
                .collect::<Vec<_>>()
                .join("; ");
            return Err(BillingError::ApiError(format!(
                "Cloudflare API error: {}",
                err_msg
            )));
        }

        Ok(api_response)
    }

    /// Get billing profile
    pub async fn get_billing_profile(&self) -> Result<CfBillingProfile> {
        let url = format!(
            "{}/accounts/{}/billing/profile",
            CF_API_ENDPOINT, self.account_id
        );
        let resp: CfApiResponse<CfBillingProfile> = self.call_api(&url).await?;
        resp.result
            .ok_or_else(|| BillingError::ApiError("No billing profile returned".to_string()))
    }

    /// Get billing history with pagination
    pub async fn get_billing_history(
        &self,
        page: i32,
        per_page: i32,
    ) -> Result<CfApiResponse<Vec<CfBillingHistory>>> {
        let url = format!(
            "{}/accounts/{}/billing/history?page={}&per_page={}",
            CF_API_ENDPOINT, self.account_id, page, per_page
        );
        self.call_api(&url).await
    }

    /// Get all billing history records with full pagination
    pub async fn get_all_billing_history(&self) -> Result<Vec<CfBillingHistory>> {
        let mut all_records = Vec::new();
        let per_page = 50;
        let mut page = 1;

        loop {
            let resp = self.get_billing_history(page, per_page).await?;
            let records = resp.result.unwrap_or_default();

            if records.is_empty() {
                break;
            }
            all_records.extend(records);

            let total_pages = resp
                .result_info
                .as_ref()
                .and_then(|ri| ri.total_pages)
                .unwrap_or(1);

            if page >= total_pages {
                break;
            }
            page += 1;
        }

        Ok(all_records)
    }

    /// Get account subscriptions
    pub async fn get_subscriptions(&self) -> Result<Vec<CfSubscription>> {
        let url = format!(
            "{}/accounts/{}/subscriptions",
            CF_API_ENDPOINT, self.account_id
        );
        let resp: CfApiResponse<Vec<CfSubscription>> = self.call_api(&url).await?;
        Ok(resp.result.unwrap_or_default())
    }

    /// Test credentials
    pub async fn test_credentials(&self) -> Result<bool> {
        match self.get_billing_profile().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation_token() {
        let client = CloudflareBillingClient::new_with_token(
            "test_account".to_string(),
            "test_token".to_string(),
        );
        assert_eq!(client.account_id, "test_account");
        assert!(client.api_token.is_some());
    }

    #[test]
    fn test_client_creation_key() {
        let client = CloudflareBillingClient::new_with_key(
            "test_account".to_string(),
            "test_key".to_string(),
            "test@example.com".to_string(),
        );
        assert_eq!(client.account_id, "test_account");
        assert!(client.api_key.is_some());
        assert!(client.api_email.is_some());
    }
}
