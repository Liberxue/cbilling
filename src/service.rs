// Copyright 2025 OpenObserve Inc.
// SPDX-License-Identifier: AGPL-3.0

//! Cloud Billing Service
//!
//! Provides core billing query functionality for multiple cloud providers.
//! Uses the [`BillingProvider`](crate::providers::traits::BillingProvider) trait
//! and shared aggregation logic to avoid code duplication across providers.

use serde::{Deserialize, Serialize};

use crate::error::{BillingError, Result};

/// Cloud account configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CloudAccountConfig {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_key_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_key_secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_access_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

/// Billing data from a cloud provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingData {
    pub billing_cycle: String,
    pub provider: String,
    pub total_cost: f64,
    pub currency: String,
    pub products: Vec<ProductCost>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_change: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_cost: Option<f64>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub accounts: Vec<AccountBillingSummary>,
}

/// Product cost information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProductCost {
    pub product_name: String,
    pub product_code: String,
    pub cost: f64,
    pub usage: Option<f64>,
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_name: Option<String>,
    /// Number of resource instances (available for Aliyun/Volcengine/UCloud/GCP)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    /// Regions where resources are deployed
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub regions: Vec<String>,
    /// Per-region breakdown: (region, cost, count)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub region_details: Vec<RegionDetail>,
}

/// Per-region cost breakdown for a product
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionDetail {
    pub region: String,
    pub cost: f64,
    pub count: u32,
}

/// Account billing summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountBillingSummary {
    pub account_id: String,
    pub account_name: String,
    pub total_cost: f64,
    pub product_count: usize,
}

/// Cloud billing service
pub struct CloudBillingService;

impl CloudBillingService {
    /// Load accounts for a specific provider from environment variables
    pub fn load_accounts_for_provider(provider: &str) -> Result<Vec<CloudAccountConfig>> {
        let env_key = match provider {
            "aliyun" => "ALIYUN_ACCOUNTS",
            "tencentcloud" => "TENCENTCLOUD_ACCOUNTS",
            "aws" => "AWS_ACCOUNTS",
            "ucloud" => "UCLOUD_ACCOUNTS",
            "volcengine" => "VOLCENGINE_ACCOUNTS",
            "gcp" => "GCP_ACCOUNTS",
            "cloudflare" => "CLOUDFLARE_ACCOUNTS",
            _ => {
                return Err(BillingError::ServiceError(format!(
                    "Unknown provider: {}",
                    provider
                )));
            }
        };

        // Try multi-account JSON configuration first
        if let Ok(json_str) = std::env::var(env_key) {
            match serde_json::from_str::<Vec<CloudAccountConfig>>(&json_str) {
                Ok(accounts) => {
                    let enabled_accounts: Vec<_> =
                        accounts.into_iter().filter(|a| a.enabled).collect();
                    tracing::info!(
                        "Loaded {} enabled accounts from {} env var",
                        enabled_accounts.len(),
                        env_key
                    );
                    return Ok(enabled_accounts);
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to parse {}: {}, falling back to legacy env vars",
                        env_key,
                        e
                    );
                }
            }
        }

        // Fallback to legacy single-account environment variables
        Self::load_legacy_account(provider)
    }

    fn load_legacy_account(provider: &str) -> Result<Vec<CloudAccountConfig>> {
        match provider {
            "aliyun" => {
                let access_key_id = std::env::var("ALIBABA_CLOUD_ACCESS_KEY_ID").ok();
                let access_key_secret = std::env::var("ALIBABA_CLOUD_ACCESS_KEY_SECRET").ok();

                if let (Some(ak), Some(sk)) = (access_key_id, access_key_secret) {
                    Ok(vec![CloudAccountConfig {
                        id: "default".to_string(),
                        name: "Default Account".to_string(),
                        access_key_id: Some(ak),
                        access_key_secret: Some(sk),
                        secret_access_key: None,
                        secret_id: None,
                        secret_key: None,
                        public_key: None,
                        private_key: None,
                        project_id: None,
                        region: Some("cn-hangzhou".to_string()),
                        enabled: true,
                    }])
                } else {
                    Err(BillingError::ServiceError(
                        "No Aliyun credentials configured".to_string(),
                    ))
                }
            }
            "aws" => {
                // AWS supports default credential chain (env vars, ~/.aws/credentials, IAM role)
                // so we always return a placeholder config — the SDK resolves credentials itself.
                let ak = std::env::var("AWS_ACCESS_KEY_ID").ok();
                let sk = std::env::var("AWS_SECRET_ACCESS_KEY").ok();
                Ok(vec![CloudAccountConfig {
                    id: "default".to_string(),
                    name: "Default Account".to_string(),
                    access_key_id: ak,
                    access_key_secret: None,
                    secret_access_key: sk,
                    secret_id: None,
                    secret_key: None,
                    public_key: None,
                    private_key: None,
                    project_id: None,
                    region: std::env::var("AWS_DEFAULT_REGION").ok(),
                    enabled: true,
                }])
            }
            "tencentcloud" => Self::load_legacy_tencentcloud(),
            "volcengine" => Self::load_legacy_volcengine(),
            "ucloud" => Self::load_legacy_ucloud(),
            "gcp" => Self::load_legacy_gcp(),
            "cloudflare" => Self::load_legacy_cloudflare(),
            _ => Err(BillingError::ServiceError(format!(
                "Provider {} not implemented for legacy mode",
                provider
            ))),
        }
    }

    // ── Unified provider query ──────────────────────────────────────────

    /// Query billing data from any configured provider.
    ///
    /// This is the primary entry point. It loads accounts, creates adapters via
    /// the provider factory, aggregates results, and returns a [`BillingData`].
    pub async fn query_provider(provider: &str, billing_cycle: &str) -> Result<BillingData> {
        let accounts = Self::load_accounts_for_provider(provider)?;

        if accounts.is_empty() {
            return Err(BillingError::ServiceError(format!(
                "No enabled {} accounts",
                provider
            )));
        }

        let mut all_products = Vec::new();
        let mut total_cost = 0.0;
        let mut account_summaries = Vec::new();
        let mut currency = "USD";

        for account in &accounts {
            let (items, cur) =
                crate::providers::factory::query_provider_items(provider, account, billing_cycle)
                    .await?;
            currency = cur;
            let (products, acct_cost) = crate::providers::aggregation::aggregate_bill_items(
                items,
                Some(&account.id),
                Some(&account.name),
            );
            total_cost += acct_cost;
            account_summaries.push(AccountBillingSummary {
                account_id: account.id.clone(),
                account_name: account.name.clone(),
                total_cost: acct_cost,
                product_count: products.len(),
            });
            all_products.extend(products);
        }

        all_products.sort_by(|a, b| {
            b.cost
                .partial_cmp(&a.cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(BillingData {
            billing_cycle: billing_cycle.to_string(),
            provider: provider.to_string(),
            total_cost,
            currency: currency.to_string(),
            products: all_products,
            cost_change: None,
            previous_cost: None,
            accounts: account_summaries,
        })
    }

    // ── Backward-compatible thin wrappers ────────────────────────────────

    /// Query billing data from Aliyun for a specific month
    #[cfg(feature = "aliyun")]
    pub async fn query_aliyun(billing_cycle: &str) -> Result<BillingData> {
        Self::query_provider("aliyun", billing_cycle).await
    }

    /// Query billing data from Tencent Cloud for a specific month
    #[cfg(feature = "tencentcloud")]
    pub async fn query_tencentcloud(billing_cycle: &str) -> Result<BillingData> {
        Self::query_provider("tencentcloud", billing_cycle).await
    }

    /// Query billing data from AWS for a specific month
    #[cfg(feature = "aws")]
    pub async fn query_aws(billing_cycle: &str) -> Result<BillingData> {
        Self::query_provider("aws", billing_cycle).await
    }

    /// Query billing data from Volcengine for a specific month
    #[cfg(feature = "volcengine")]
    pub async fn query_volcengine(billing_cycle: &str) -> Result<BillingData> {
        Self::query_provider("volcengine", billing_cycle).await
    }

    /// Query billing data from UCloud for a specific month
    #[cfg(feature = "ucloud")]
    pub async fn query_ucloud(billing_cycle: &str) -> Result<BillingData> {
        Self::query_provider("ucloud", billing_cycle).await
    }

    /// Query billing data from GCP for a specific month
    #[cfg(feature = "gcp")]
    pub async fn query_gcp(billing_cycle: &str) -> Result<BillingData> {
        Self::query_provider("gcp", billing_cycle).await
    }

    /// Query billing data from Cloudflare for a specific month
    #[cfg(feature = "cloudflare")]
    pub async fn query_cloudflare(billing_cycle: &str) -> Result<BillingData> {
        Self::query_provider("cloudflare", billing_cycle).await
    }

    // ── Legacy account loaders ──────────────────────────────────────────

    fn load_legacy_tencentcloud() -> Result<Vec<CloudAccountConfig>> {
        let secret_id = std::env::var("TENCENTCLOUD_SECRET_ID").ok();
        let secret_key = std::env::var("TENCENTCLOUD_SECRET_KEY").ok();
        if let (Some(id), Some(key)) = (secret_id, secret_key) {
            Ok(vec![CloudAccountConfig {
                id: "default".to_string(),
                name: "Default Account".to_string(),
                access_key_id: None,
                access_key_secret: None,
                secret_access_key: None,
                secret_id: Some(id),
                secret_key: Some(key),
                public_key: None,
                private_key: None,
                project_id: None,
                region: std::env::var("TENCENTCLOUD_REGION").ok(),
                enabled: true,
            }])
        } else {
            Err(BillingError::ServiceError(
                "No TencentCloud credentials configured".to_string(),
            ))
        }
    }

    fn load_legacy_volcengine() -> Result<Vec<CloudAccountConfig>> {
        let ak = std::env::var("VOLCENGINE_ACCESS_KEY_ID").ok();
        let sk = std::env::var("VOLCENGINE_SECRET_ACCESS_KEY").ok();
        if let (Some(ak), Some(sk)) = (ak, sk) {
            Ok(vec![CloudAccountConfig {
                id: "default".to_string(),
                name: "Default Account".to_string(),
                access_key_id: Some(ak),
                access_key_secret: None,
                secret_access_key: Some(sk),
                secret_id: None,
                secret_key: None,
                public_key: None,
                private_key: None,
                project_id: None,
                region: Some("cn-beijing".to_string()),
                enabled: true,
            }])
        } else {
            Err(BillingError::ServiceError(
                "No Volcengine credentials configured".to_string(),
            ))
        }
    }

    fn load_legacy_ucloud() -> Result<Vec<CloudAccountConfig>> {
        let pub_key = std::env::var("UCLOUD_PUBLIC_KEY").ok();
        let priv_key = std::env::var("UCLOUD_PRIVATE_KEY").ok();
        let project = std::env::var("UCLOUD_PROJECT_ID").ok();
        if let (Some(pk), Some(sk), Some(proj)) = (pub_key, priv_key, project) {
            Ok(vec![CloudAccountConfig {
                id: "default".to_string(),
                name: "Default Account".to_string(),
                access_key_id: None,
                access_key_secret: None,
                secret_access_key: None,
                secret_id: None,
                secret_key: None,
                public_key: Some(pk),
                private_key: Some(sk),
                project_id: Some(proj),
                region: None,
                enabled: true,
            }])
        } else {
            Err(BillingError::ServiceError(
                "No UCloud credentials configured".to_string(),
            ))
        }
    }

    fn load_legacy_gcp() -> Result<Vec<CloudAccountConfig>> {
        let project_id = std::env::var("GCP_PROJECT_ID").ok();
        let sa_json = std::env::var("GCP_SERVICE_ACCOUNT_JSON").ok();
        let has_creds_file = std::env::var("GOOGLE_APPLICATION_CREDENTIALS").is_ok();

        if let Some(proj) = project_id {
            Ok(vec![CloudAccountConfig {
                id: "default".to_string(),
                name: "Default Account".to_string(),
                access_key_id: None,
                access_key_secret: None,
                secret_access_key: None,
                secret_id: None,
                secret_key: None,
                public_key: None,
                private_key: sa_json, // service account JSON content
                project_id: Some(proj),
                region: None,
                enabled: true,
            }])
        } else if has_creds_file {
            // Infer project from credentials file
            Ok(vec![CloudAccountConfig {
                id: "default".to_string(),
                name: "Default Account".to_string(),
                access_key_id: None,
                access_key_secret: None,
                secret_access_key: None,
                secret_id: None,
                secret_key: None,
                public_key: None,
                private_key: None,
                project_id: Some("auto".to_string()),
                region: None,
                enabled: true,
            }])
        } else {
            Err(BillingError::ServiceError(
                "No GCP credentials configured".to_string(),
            ))
        }
    }

    fn load_legacy_cloudflare() -> Result<Vec<CloudAccountConfig>> {
        let account_id = std::env::var("CLOUDFLARE_ACCOUNT_ID").ok();
        let api_token = std::env::var("CLOUDFLARE_API_TOKEN").ok();
        let api_key = std::env::var("CLOUDFLARE_API_KEY").ok();
        let api_email = std::env::var("CLOUDFLARE_API_EMAIL").ok();

        if let Some(acct) = account_id {
            Ok(vec![CloudAccountConfig {
                id: "default".to_string(),
                name: "Default Account".to_string(),
                access_key_id: Some(acct),  // account_id
                access_key_secret: api_key, // api_key (for key+email auth)
                secret_access_key: None,
                secret_id: api_email,  // api_email (for key+email auth)
                secret_key: api_token, // api_token (for token auth)
                public_key: None,
                private_key: None,
                project_id: None,
                region: None,
                enabled: true,
            }])
        } else {
            Err(BillingError::ServiceError(
                "No Cloudflare credentials configured".to_string(),
            ))
        }
    }

    /// Get list of configured cloud providers
    pub fn get_configured_providers() -> Vec<String> {
        let mut providers = Vec::new();

        #[cfg(feature = "aliyun")]
        if Self::load_accounts_for_provider("aliyun").is_ok() {
            providers.push("aliyun".to_string());
        }

        #[cfg(feature = "tencentcloud")]
        if Self::load_accounts_for_provider("tencentcloud").is_ok() {
            providers.push("tencentcloud".to_string());
        }

        #[cfg(feature = "aws")]
        {
            let has_aws = std::env::var("AWS_ACCOUNTS").is_ok()
                || std::env::var("AWS_ACCESS_KEY_ID").is_ok()
                || std::path::Path::new(&format!(
                    "{}/.aws/credentials",
                    std::env::var("HOME").unwrap_or_default()
                ))
                .exists();

            if has_aws {
                providers.push("aws".to_string());
            }
        }

        #[cfg(feature = "ucloud")]
        if Self::load_accounts_for_provider("ucloud").is_ok() {
            providers.push("ucloud".to_string());
        }

        #[cfg(feature = "volcengine")]
        if Self::load_accounts_for_provider("volcengine").is_ok() {
            providers.push("volcengine".to_string());
        }

        #[cfg(feature = "gcp")]
        if Self::load_accounts_for_provider("gcp").is_ok() {
            providers.push("gcp".to_string());
        }

        #[cfg(feature = "cloudflare")]
        if Self::load_accounts_for_provider("cloudflare").is_ok() {
            providers.push("cloudflare".to_string());
        }

        providers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloud_account_config() {
        let config = CloudAccountConfig {
            id: "test".to_string(),
            name: "Test Account".to_string(),
            access_key_id: Some("key".to_string()),
            access_key_secret: Some("secret".to_string()),
            secret_access_key: None,
            secret_id: None,
            secret_key: None,
            public_key: None,
            private_key: None,
            project_id: None,
            region: Some("cn-hangzhou".to_string()),
            enabled: true,
        };

        assert_eq!(config.id, "test");
        assert!(config.enabled);
    }

    #[test]
    fn test_get_configured_providers() {
        let providers = CloudBillingService::get_configured_providers();
        // May be empty in tests without env vars -- just verify it doesn't panic
        let _ = providers;
    }

    #[test]
    fn test_billing_data_serialization() {
        let data = BillingData {
            billing_cycle: "2026-03".to_string(),
            provider: "test".to_string(),
            total_cost: 123.45,
            currency: "USD".to_string(),
            products: vec![ProductCost {
                product_name: "Compute".to_string(),
                product_code: "compute".to_string(),
                cost: 123.45,
                ..Default::default()
            }],
            cost_change: None,
            previous_cost: None,
            accounts: vec![],
        };

        let json = serde_json::to_string(&data).expect("serialize");
        let deserialized: BillingData = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.total_cost, 123.45);
        assert_eq!(deserialized.products.len(), 1);
    }

    #[test]
    fn test_product_cost_default() {
        let pc = ProductCost::default();
        assert_eq!(pc.cost, 0.0);
        assert!(pc.product_name.is_empty());
        assert!(pc.regions.is_empty());
        assert!(pc.region_details.is_empty());
        assert!(pc.count.is_none());
    }
}
