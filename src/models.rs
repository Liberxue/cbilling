// Copyright 2025 OpenObserve Inc.
// SPDX-License-Identifier: AGPL-3.0

//! Data models for multi-cloud billing management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Cloud Account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: i64,
    pub org_id: String,
    pub name: String,
    pub provider: String, // aliyun, tencent, aws, azure, volcengine
    pub account_id: String,
    pub access_key: String,
    pub secret_key: String,
    pub region: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Bill Record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bill {
    pub id: i64,
    pub account_id: i64,
    pub org_id: String,
    pub billing_cycle: String,
    pub amount: f64,
    pub currency: String,
    pub product_code: String,
    pub product_name: Option<String>,
    pub provider: Option<String>,
    pub region: Option<String>,
    pub instance_id: Option<String>,
    pub project_id: Option<String>,
    pub usage_amount: f64,
    pub usage_unit: String,
    pub deduction_amount: Option<f64>,
    pub created_at: DateTime<Utc>,
}

/// Cost Overview for a billing cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostOverview {
    pub billing_cycle: String,
    pub total_cost: f64,
    pub account_count: i32,
    pub accounts: Vec<AccountCost>,
}

/// Cost per account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountCost {
    pub account_id: i64,
    pub account_name: String,
    pub provider: String,
    pub cost: f64,
    pub percentage: f64,
}

/// Billing Statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingStats {
    pub month: String,
    pub total_cost: f64,
    pub service_cost: f64,
    pub discount: f64,
    pub monthly_recurring: f64,
}

/// Provider Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub display_name: String,
    pub enabled: bool,
    pub fields: Vec<ConfigField>,
}

/// Configuration field for a provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigField {
    pub name: String,
    pub label: String,
    pub field_type: String, // text, password, select
    pub required: bool,
    pub options: Option<Vec<String>>,
}

/// Cloud Provider type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Aliyun,
    TencentCloud,
    AWS,
    Azure,
    Volcengine,
    UCloud,
    Gcp,
    Cloudflare,
}

impl Provider {
    /// Returns the string identifier for this provider.
    pub fn as_str(&self) -> &'static str {
        match self {
            Provider::Aliyun => "aliyun",
            Provider::TencentCloud => "tencentcloud",
            Provider::AWS => "aws",
            Provider::Azure => "azure",
            Provider::Volcengine => "volcengine",
            Provider::UCloud => "ucloud",
            Provider::Gcp => "gcp",
            Provider::Cloudflare => "cloudflare",
        }
    }
}

impl std::str::FromStr for Provider {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "aliyun" => Ok(Provider::Aliyun),
            "tencentcloud" | "tencent" => Ok(Provider::TencentCloud),
            "aws" => Ok(Provider::AWS),
            "azure" => Ok(Provider::Azure),
            "volcengine" => Ok(Provider::Volcengine),
            "ucloud" => Ok(Provider::UCloud),
            "gcp" | "google" | "googlecloud" => Ok(Provider::Gcp),
            "cloudflare" | "cf" => Ok(Provider::Cloudflare),
            _ => Err(format!("unknown provider: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_creation() {
        let account = Account {
            id: 1,
            org_id: "org_123".to_string(),
            name: "Production Account".to_string(),
            provider: "aliyun".to_string(),
            account_id: "1234567890".to_string(),
            access_key: "test_key".to_string(),
            secret_key: "test_secret".to_string(),
            region: Some("cn-beijing".to_string()),
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(account.id, 1);
        assert_eq!(account.provider, "aliyun");
        assert!(account.enabled);
    }

    #[test]
    fn test_provider_enum() {
        assert_eq!(Provider::Aliyun.as_str(), "aliyun");
        assert_eq!("aws".parse::<Provider>().unwrap(), Provider::AWS);
        assert_eq!(
            "tencent".parse::<Provider>().unwrap(),
            Provider::TencentCloud
        );
        assert!("unknown".parse::<Provider>().is_err());
    }
}
