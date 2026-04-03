// Copyright 2025 OpenObserve Inc.
// SPDX-License-Identifier: AGPL-3.0

//! Cloud Billing Service
//!
//! Provides core billing query functionality for multiple cloud providers.
//! For database operations, use the `db` module directly.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Aggregator: product_key -> (total_cost, unique_instance_ids, region -> (cost, count))
type ProductAggregator = HashMap<
    String,
    (
        f64,
        std::collections::HashSet<String>,
        HashMap<String, (f64, u32)>,
    ),
>;

use crate::error::{BillingError, Result};

#[cfg(feature = "aliyun")]
use crate::providers::aliyun;
#[cfg(feature = "aws")]
use crate::providers::aws;
#[cfg(feature = "cloudflare")]
use crate::providers::cloudflare;
#[cfg(feature = "gcp")]
use crate::providers::gcp;
#[cfg(feature = "tencentcloud")]
use crate::providers::tencentcloud;
#[cfg(feature = "ucloud")]
use crate::providers::ucloud;
#[cfg(feature = "volcengine")]
use crate::providers::volcengine;

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

    /// Query billing data from Aliyun for a specific month
    #[cfg(feature = "aliyun")]
    pub async fn query_aliyun(billing_cycle: &str) -> Result<BillingData> {
        let accounts = Self::load_accounts_for_provider("aliyun")?;

        if accounts.is_empty() {
            return Err(BillingError::ServiceError(
                "No enabled Aliyun accounts".to_string(),
            ));
        }

        let mut all_products = Vec::new();
        let mut total_cost = 0.0;
        let mut account_summaries = Vec::new();

        for account in accounts {
            let access_key_id = account
                .access_key_id
                .ok_or_else(|| BillingError::ServiceError("Missing access_key_id".to_string()))?;
            let access_key_secret = account.access_key_secret.ok_or_else(|| {
                BillingError::ServiceError("Missing access_key_secret".to_string())
            })?;
            let client = aliyun::AliyunBillingClient::new(access_key_id, access_key_secret);

            // Paginate through all pages, collecting per-region detail
            let page_size = 300;
            let mut page_num = 1;
            let mut products_map: ProductAggregator = HashMap::new();

            loop {
                let response = client
                    .query_instance_bill(billing_cycle, Some(page_num), Some(page_size), None)
                    .await?;

                if !response.success {
                    tracing::error!(
                        "Aliyun API error for account {}: {} - {}",
                        account.name,
                        response.code,
                        response.message
                    );
                    break;
                }

                let data = match response.data {
                    Some(d) => d,
                    None => break,
                };
                let api_total = data.total_count;

                if let Some(items) = data.items {
                    for item in items.item {
                        let cost = item.pretax_amount.unwrap_or(0.0);
                        total_cost += cost;

                        let product_name = item
                            .product_name
                            .unwrap_or_else(|| "Unknown Product".to_string());
                        let product_code =
                            item.product_code.unwrap_or_else(|| "unknown".to_string());
                        let region = item.region.unwrap_or_default();
                        let instance_id = item.instance_id.unwrap_or_default();

                        let key = format!("{}:{}", product_code, product_name);
                        let entry = products_map.entry(key).or_insert_with(|| {
                            (0.0, std::collections::HashSet::new(), HashMap::new())
                        });
                        entry.0 += cost;
                        if !instance_id.is_empty() {
                            entry.1.insert(instance_id);
                        }
                        if !region.is_empty() {
                            let r = entry.2.entry(region).or_insert((0.0, 0));
                            r.0 += cost;
                            r.1 += 1;
                        }
                    }
                }

                if page_num * page_size >= api_total {
                    break;
                }
                page_num += 1;
            }

            let products: Vec<ProductCost> = products_map
                .into_iter()
                .map(|(key, (cost, instances, regions_map))| {
                    let parts: Vec<&str> = key.splitn(2, ':').collect();
                    let count = if instances.is_empty() {
                        None
                    } else {
                        Some(instances.len() as u32)
                    };
                    let mut regions: Vec<String> = regions_map.keys().cloned().collect();
                    regions.sort();
                    let mut region_details: Vec<RegionDetail> = regions_map
                        .into_iter()
                        .map(|(r, (c, n))| RegionDetail {
                            region: r,
                            cost: c,
                            count: n,
                        })
                        .collect();
                    region_details.sort_by(|a, b| {
                        b.cost
                            .partial_cmp(&a.cost)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                    ProductCost {
                        product_code: parts.first().unwrap_or(&"unknown").to_string(),
                        product_name: parts.get(1).unwrap_or(&"Unknown").to_string(),
                        cost,
                        usage: None,
                        unit: None,
                        account_id: Some(account.id.clone()),
                        account_name: Some(account.name.clone()),
                        count,
                        regions,
                        region_details,
                    }
                })
                .collect();

            let account_cost: f64 = products.iter().map(|p| p.cost).sum();
            account_summaries.push(AccountBillingSummary {
                account_id: account.id.clone(),
                account_name: account.name.clone(),
                total_cost: account_cost,
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
            provider: "aliyun".to_string(),
            total_cost,
            currency: "CNY".to_string(),
            products: all_products,
            cost_change: None,
            previous_cost: None,
            accounts: account_summaries,
        })
    }

    /// Query billing data from Tencent Cloud for a specific month
    #[cfg(feature = "tencentcloud")]
    pub async fn query_tencentcloud(billing_cycle: &str) -> Result<BillingData> {
        let accounts = Self::load_accounts_for_provider("tencentcloud")
            .or_else(|_| Self::load_legacy_tencentcloud())?;

        if accounts.is_empty() {
            return Err(BillingError::ServiceError(
                "No enabled TencentCloud accounts".to_string(),
            ));
        }

        let mut all_products = Vec::new();
        let mut total_cost = 0.0;
        let mut account_summaries = Vec::new();

        for account in accounts {
            let secret_id = account
                .secret_id
                .ok_or_else(|| BillingError::ServiceError("Missing secret_id".to_string()))?;
            let secret_key = account
                .secret_key
                .ok_or_else(|| BillingError::ServiceError("Missing secret_key".to_string()))?;
            let region = account.region.unwrap_or_else(|| "ap-guangzhou".to_string());

            let client =
                tencentcloud::TencentCloudBillingClient::new(secret_id, secret_key, region);
            let summary = client.get_bill_summary(billing_cycle).await?;

            if let Some(items) = summary.response.summary_overview {
                let mut products_map: HashMap<String, f64> = HashMap::new();

                for item in &items {
                    let cost: f64 = item.real_total_cost.parse().unwrap_or(0.0);
                    total_cost += cost;
                    let name = item
                        .business_code_name
                        .clone()
                        .unwrap_or_else(|| "Unknown".to_string());
                    let code = item
                        .business_code
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string());
                    let key = format!("{}:{}", code, name);
                    *products_map.entry(key).or_insert(0.0) += cost;
                }

                let products: Vec<ProductCost> = products_map
                    .into_iter()
                    .map(|(key, cost)| {
                        let parts: Vec<&str> = key.splitn(2, ':').collect();
                        ProductCost {
                            product_code: parts.first().unwrap_or(&"unknown").to_string(),
                            product_name: parts.get(1).unwrap_or(&"Unknown").to_string(),
                            cost,
                            usage: None,
                            unit: None,
                            account_id: Some(account.id.clone()),
                            account_name: Some(account.name.clone()),
                            count: None,
                            regions: vec![],
                            region_details: vec![],
                        }
                    })
                    .collect();

                let account_cost: f64 = products.iter().map(|p| p.cost).sum();
                account_summaries.push(AccountBillingSummary {
                    account_id: account.id.clone(),
                    account_name: account.name.clone(),
                    total_cost: account_cost,
                    product_count: products.len(),
                });
                all_products.extend(products);
            }
        }

        all_products.sort_by(|a, b| {
            b.cost
                .partial_cmp(&a.cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(BillingData {
            billing_cycle: billing_cycle.to_string(),
            provider: "tencentcloud".to_string(),
            total_cost,
            currency: "CNY".to_string(),
            products: all_products,
            cost_change: None,
            previous_cost: None,
            accounts: account_summaries,
        })
    }

    /// Query billing data from AWS for a specific month
    #[cfg(feature = "aws")]
    pub async fn query_aws(billing_cycle: &str) -> Result<BillingData> {
        // Convert "2026-03" to date range
        let start_date = format!("{}-01", billing_cycle);
        let end_date = {
            let parts: Vec<&str> = billing_cycle.split('-').collect();
            let year: i32 = parts[0].parse().unwrap_or(2026);
            let month: u32 = parts[1].parse().unwrap_or(1);
            if month == 12 {
                format!("{}-01-01", year + 1)
            } else {
                format!("{}-{:02}-01", year, month + 1)
            }
        };

        let client =
            aws::AwsBillingClient::new_with_default_credentials("us-east-1".to_string()).await?;

        let result = client
            .get_cost_and_usage(
                &start_date,
                &end_date,
                "MONTHLY",
                vec!["UnblendedCost"],
                Some(vec![("DIMENSION", "SERVICE")]),
            )
            .await?;

        let mut all_products = Vec::new();
        let mut total_cost = 0.0;

        if let Some(results) = result.results_by_time {
            for period in results {
                if let Some(groups) = period.groups {
                    for group in groups {
                        let service = group.keys.first().cloned().unwrap_or_default();
                        let cost = group
                            .metrics
                            .get("UnblendedCost")
                            .and_then(|v| v.get("Amount"))
                            .and_then(|v| v.as_str())
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(0.0);
                        total_cost += cost;
                        all_products.push(ProductCost {
                            product_name: service.clone(),
                            product_code: service,
                            cost,
                            usage: None,
                            unit: None,
                            account_id: None,
                            account_name: None,
                            count: None,
                            regions: vec![],
                            region_details: vec![],
                        });
                    }
                }
            }
        }

        all_products.sort_by(|a, b| {
            b.cost
                .partial_cmp(&a.cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(BillingData {
            billing_cycle: billing_cycle.to_string(),
            provider: "aws".to_string(),
            total_cost,
            currency: "USD".to_string(),
            products: all_products,
            cost_change: None,
            previous_cost: None,
            accounts: vec![],
        })
    }

    /// Query billing data from Volcengine for a specific month
    #[cfg(feature = "volcengine")]
    pub async fn query_volcengine(billing_cycle: &str) -> Result<BillingData> {
        let accounts = Self::load_accounts_for_provider("volcengine")
            .or_else(|_| Self::load_legacy_volcengine())?;

        if accounts.is_empty() {
            return Err(BillingError::ServiceError(
                "No enabled Volcengine accounts".to_string(),
            ));
        }

        let mut all_products = Vec::new();
        let mut total_cost = 0.0;

        for account in accounts {
            let access_key_id = account
                .access_key_id
                .ok_or_else(|| BillingError::ServiceError("Missing access_key_id".to_string()))?;
            let secret_access_key = account
                .secret_access_key
                .or(account.access_key_secret)
                .ok_or_else(|| {
                    BillingError::ServiceError("Missing secret_access_key".to_string())
                })?;
            let region = account.region.unwrap_or_else(|| "cn-beijing".to_string());

            let client =
                volcengine::VolcengineBillingClient::new(access_key_id, secret_access_key, region);

            let limit = 100;
            let mut offset = 0;
            let mut products_map: ProductAggregator = HashMap::new();

            loop {
                let response = client
                    .list_bill_detail(billing_cycle, Some(limit), Some(offset))
                    .await?;

                let result = match response.result {
                    Some(r) => r,
                    None => break,
                };
                let api_total = result.total.unwrap_or(0);

                if let Some(items) = result.list {
                    if items.is_empty() {
                        break;
                    }
                    for item in &items {
                        let cost: f64 = item
                            .payable_amount
                            .as_deref()
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0.0);
                        total_cost += cost;
                        let name = item
                            .product_zh
                            .clone()
                            .unwrap_or_else(|| "Unknown".to_string());
                        let code = item
                            .product
                            .clone()
                            .unwrap_or_else(|| "unknown".to_string());
                        let region = item.region.clone().unwrap_or_default();
                        let instance_id = item.instance_id.clone().unwrap_or_default();

                        let key = format!("{}:{}", code, name);
                        let entry = products_map.entry(key).or_insert_with(|| {
                            (0.0, std::collections::HashSet::new(), HashMap::new())
                        });
                        entry.0 += cost;
                        if !instance_id.is_empty() {
                            entry.1.insert(instance_id);
                        }
                        if !region.is_empty() {
                            let r = entry.2.entry(region).or_insert((0.0, 0));
                            r.0 += cost;
                            r.1 += 1;
                        }
                    }
                } else {
                    break;
                }

                offset += limit;
                if (offset as i64) >= api_total {
                    break;
                }
            }

            let products: Vec<ProductCost> = products_map
                .into_iter()
                .map(|(key, (cost, instances, regions_map))| {
                    let parts: Vec<&str> = key.splitn(2, ':').collect();
                    let count = if instances.is_empty() {
                        None
                    } else {
                        Some(instances.len() as u32)
                    };
                    let mut regions: Vec<String> = regions_map.keys().cloned().collect();
                    regions.sort();
                    let mut region_details: Vec<RegionDetail> = regions_map
                        .into_iter()
                        .map(|(r, (c, n))| RegionDetail {
                            region: r,
                            cost: c,
                            count: n,
                        })
                        .collect();
                    region_details.sort_by(|a, b| {
                        b.cost
                            .partial_cmp(&a.cost)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                    ProductCost {
                        product_code: parts.first().unwrap_or(&"unknown").to_string(),
                        product_name: parts.get(1).unwrap_or(&"Unknown").to_string(),
                        cost,
                        usage: None,
                        unit: None,
                        account_id: Some(account.id.clone()),
                        account_name: Some(account.name.clone()),
                        count,
                        regions,
                        region_details,
                    }
                })
                .collect();

            all_products.extend(products);
        }

        all_products.sort_by(|a, b| {
            b.cost
                .partial_cmp(&a.cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(BillingData {
            billing_cycle: billing_cycle.to_string(),
            provider: "volcengine".to_string(),
            total_cost,
            currency: "CNY".to_string(),
            products: all_products,
            cost_change: None,
            previous_cost: None,
            accounts: vec![],
        })
    }

    /// Query billing data from UCloud for a specific month
    #[cfg(feature = "ucloud")]
    pub async fn query_ucloud(billing_cycle: &str) -> Result<BillingData> {
        let accounts =
            Self::load_accounts_for_provider("ucloud").or_else(|_| Self::load_legacy_ucloud())?;

        if accounts.is_empty() {
            return Err(BillingError::ServiceError(
                "No enabled UCloud accounts".to_string(),
            ));
        }

        let mut all_products = Vec::new();
        let mut total_cost = 0.0;

        for account in accounts {
            let public_key = account
                .public_key
                .ok_or_else(|| BillingError::ServiceError("Missing public_key".to_string()))?;
            let private_key = account
                .private_key
                .ok_or_else(|| BillingError::ServiceError("Missing private_key".to_string()))?;
            let project_id = account
                .project_id
                .ok_or_else(|| BillingError::ServiceError("Missing project_id".to_string()))?;

            let client = ucloud::UCloudBillingClient::new(public_key, private_key, project_id);

            // Paginate through all pages
            let limit = 100;
            let mut offset = 0;
            let mut products_map: HashMap<String, f64> = HashMap::new();

            loop {
                let response = client
                    .query_bill_list(billing_cycle, Some(offset), Some(limit))
                    .await?;

                let api_total = response.total_count.unwrap_or(0);

                if let Some(items) = response.items {
                    if items.is_empty() {
                        break;
                    }
                    for item in &items {
                        let cost = item.amount_real.or(item.amount).unwrap_or(0.0);
                        total_cost += cost;
                        let name = item
                            .product_name
                            .clone()
                            .or_else(|| item.resource_type.clone())
                            .unwrap_or_else(|| "Unknown".to_string());
                        *products_map.entry(name).or_insert(0.0) += cost;
                    }
                } else {
                    break;
                }

                offset += limit;
                if offset >= api_total {
                    break;
                }
            }

            let products: Vec<ProductCost> = products_map
                .into_iter()
                .map(|(name, cost)| ProductCost {
                    product_name: name.clone(),
                    product_code: name,
                    cost,
                    usage: None,
                    unit: None,
                    account_id: Some(account.id.clone()),
                    account_name: Some(account.name.clone()),
                    count: None,
                    regions: vec![],
                    region_details: vec![],
                })
                .collect();
            all_products.extend(products);
        }

        all_products.sort_by(|a, b| {
            b.cost
                .partial_cmp(&a.cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(BillingData {
            billing_cycle: billing_cycle.to_string(),
            provider: "ucloud".to_string(),
            total_cost,
            currency: "CNY".to_string(),
            products: all_products,
            cost_change: None,
            previous_cost: None,
            accounts: vec![],
        })
    }

    /// Query billing data from GCP for a specific month
    ///
    /// Uses BigQuery billing export when `GCP_BILLING_DATASET` is set.
    /// Falls back to listing services (without cost data) otherwise.
    #[cfg(feature = "gcp")]
    pub async fn query_gcp(billing_cycle: &str) -> Result<BillingData> {
        let accounts =
            Self::load_accounts_for_provider("gcp").or_else(|_| Self::load_legacy_gcp())?;

        if accounts.is_empty() {
            return Err(BillingError::ServiceError(
                "No enabled GCP accounts".to_string(),
            ));
        }

        let dataset = std::env::var("GCP_BILLING_DATASET").ok();
        let billing_table = std::env::var("GCP_BILLING_TABLE").ok();

        let mut all_products = Vec::new();
        let mut total_cost = 0.0;

        for account in accounts {
            let project_id = account
                .project_id
                .ok_or_else(|| BillingError::ServiceError("Missing project_id".to_string()))?;

            let client = if let Some(sa_json) = account.private_key {
                gcp::GcpBillingClient::new(project_id.clone(), sa_json).await?
            } else {
                gcp::GcpBillingClient::new_from_credentials_file(project_id.clone()).await?
            };

            if let Some(ref ds) = dataset {
                // BigQuery billing export — returns actual costs
                let items = client
                    .query_billing_costs(billing_cycle, ds, billing_table.as_deref())
                    .await?;

                let mut products_map: HashMap<String, (f64, Option<f64>, Option<String>)> =
                    HashMap::new();
                for item in &items {
                    let entry = products_map
                        .entry(item.service_id.clone())
                        .or_insert((0.0, None, None));
                    entry.0 += item.cost;
                    entry.1 = item.usage_amount.or(entry.1);
                    entry.2 = item.usage_unit.clone().or_else(|| entry.2.clone());
                }

                for (service_id, (cost, usage, unit)) in products_map {
                    total_cost += cost;
                    // Find display name from items
                    let name = items
                        .iter()
                        .find(|i| i.service_id == service_id)
                        .map(|i| i.service_name.clone())
                        .unwrap_or_else(|| service_id.clone());
                    all_products.push(ProductCost {
                        product_name: name,
                        product_code: service_id,
                        cost,
                        usage,
                        unit,
                        account_id: Some(account.id.clone()),
                        account_name: Some(account.name.clone()),
                        count: None,
                        regions: vec![],
                        region_details: vec![],
                    });
                }
            } else {
                // Fallback: list services (no cost data)
                let services = client.list_services().await.unwrap_or_default();
                for svc in services {
                    let name = svc.display_name.unwrap_or_else(|| svc.name.clone());
                    let code = svc.service_id.unwrap_or_else(|| svc.name.clone());
                    all_products.push(ProductCost {
                        product_name: name,
                        product_code: code,
                        cost: 0.0,
                        usage: None,
                        unit: None,
                        account_id: Some(account.id.clone()),
                        account_name: Some(account.name.clone()),
                        count: None,
                        regions: vec![],
                        region_details: vec![],
                    });
                }
            }
        }

        all_products.sort_by(|a, b| {
            b.cost
                .partial_cmp(&a.cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(BillingData {
            billing_cycle: billing_cycle.to_string(),
            provider: "gcp".to_string(),
            total_cost,
            currency: "USD".to_string(),
            products: all_products,
            cost_change: None,
            previous_cost: None,
            accounts: vec![],
        })
    }

    /// Query billing data from Cloudflare for a specific month
    #[cfg(feature = "cloudflare")]
    pub async fn query_cloudflare(billing_cycle: &str) -> Result<BillingData> {
        let accounts = Self::load_accounts_for_provider("cloudflare")
            .or_else(|_| Self::load_legacy_cloudflare())?;

        if accounts.is_empty() {
            return Err(BillingError::ServiceError(
                "No enabled Cloudflare accounts".to_string(),
            ));
        }

        let mut all_products = Vec::new();
        let mut total_cost = 0.0;

        for account in accounts {
            let account_id = account
                .access_key_id
                .ok_or_else(|| BillingError::ServiceError("Missing account_id".to_string()))?;

            let client = if let Some(token) = account.secret_key {
                cloudflare::CloudflareBillingClient::new_with_token(account_id, token)
            } else if let (Some(key), Some(email)) = (account.access_key_secret, account.secret_id)
            {
                cloudflare::CloudflareBillingClient::new_with_key(account_id, key, email)
            } else {
                return Err(BillingError::InvalidCredentials(
                    "Missing Cloudflare API token or API key+email".to_string(),
                ));
            };

            // Get subscriptions — these have actual pricing
            let subs = client.get_subscriptions().await.unwrap_or_default();
            let mut products_map: HashMap<String, f64> = HashMap::new();

            for sub in &subs {
                let price = sub.price.unwrap_or(0.0);
                total_cost += price;
                let name = sub
                    .rate_plan
                    .as_ref()
                    .and_then(|rp| rp.public_name.clone())
                    .unwrap_or_else(|| "Unknown Plan".to_string());
                *products_map.entry(name).or_insert(0.0) += price;
            }

            // Also fetch billing history for the specified month
            let history = client.get_all_billing_history().await.unwrap_or_default();
            for record in &history {
                // Filter by billing_cycle month
                if let Some(ref occurred) = record.occurred_at {
                    if occurred.starts_with(billing_cycle) {
                        if let Some(amount) = record.amount {
                            let desc = record
                                .description
                                .clone()
                                .unwrap_or_else(|| "Billing Charge".to_string());
                            *products_map.entry(desc).or_insert(0.0) += amount;
                            // Don't add to total_cost again — subscriptions already counted
                        }
                    }
                }
            }

            let products: Vec<ProductCost> = products_map
                .into_iter()
                .map(|(name, cost)| ProductCost {
                    product_name: name.clone(),
                    product_code: name,
                    cost,
                    usage: None,
                    unit: None,
                    account_id: Some(account.id.clone()),
                    account_name: Some(account.name.clone()),
                    count: None,
                    regions: vec![],
                    region_details: vec![],
                })
                .collect();
            all_products.extend(products);
        }

        all_products.sort_by(|a, b| {
            b.cost
                .partial_cmp(&a.cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(BillingData {
            billing_cycle: billing_cycle.to_string(),
            provider: "cloudflare".to_string(),
            total_cost,
            currency: "USD".to_string(),
            products: all_products,
            cost_change: None,
            previous_cost: None,
            accounts: vec![],
        })
    }

    /// Query billing data from any configured provider
    pub async fn query_provider(provider: &str, billing_cycle: &str) -> Result<BillingData> {
        match provider {
            #[cfg(feature = "aliyun")]
            "aliyun" => Self::query_aliyun(billing_cycle).await,
            #[cfg(feature = "tencentcloud")]
            "tencentcloud" => Self::query_tencentcloud(billing_cycle).await,
            #[cfg(feature = "aws")]
            "aws" => Self::query_aws(billing_cycle).await,
            #[cfg(feature = "volcengine")]
            "volcengine" => Self::query_volcengine(billing_cycle).await,
            #[cfg(feature = "ucloud")]
            "ucloud" => Self::query_ucloud(billing_cycle).await,
            #[cfg(feature = "gcp")]
            "gcp" => Self::query_gcp(billing_cycle).await,
            #[cfg(feature = "cloudflare")]
            "cloudflare" => Self::query_cloudflare(billing_cycle).await,
            _ => Err(BillingError::ServiceError(format!(
                "Unknown or disabled provider: {}",
                provider
            ))),
        }
    }

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
        // May be empty in tests without env vars — just verify it doesn't panic
        let _ = providers;
    }
}
