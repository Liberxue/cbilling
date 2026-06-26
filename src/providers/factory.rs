// Copyright 2025 OpenObserve Inc.
// SPDX-License-Identifier: AGPL-3.0

//! Provider Factory
//!
//! Creates a provider adapter and queries bill items in one step.
//! This avoids needing `dyn` trait objects or enum dispatch by using
//! a static match on the provider name.

use crate::error::{BillingError, Result};
use crate::service::CloudAccountConfig;

use super::traits::RawBillItem;

/// Create a provider adapter and query bill items in one step.
///
/// Returns `(items, currency)` where `currency` is the provider's default currency string.
pub async fn query_provider_items(
    provider_name: &str,
    config: &CloudAccountConfig,
    billing_cycle: &str,
) -> Result<(Vec<RawBillItem>, &'static str)> {
    match provider_name {
        #[cfg(feature = "aliyun")]
        "aliyun" => {
            use super::aliyun::AliyunBillingAdapter;
            use super::traits::BillingProvider;
            let adapter = AliyunBillingAdapter::from_config(config)?;
            let items = adapter.query_bill_items(billing_cycle).await?;
            Ok((items, adapter.currency()))
        }
        #[cfg(feature = "aws")]
        "aws" => {
            use super::aws::AwsBillingAdapter;
            use super::traits::BillingProvider;
            let adapter = AwsBillingAdapter::from_config(config).await?;
            let items = adapter.query_bill_items(billing_cycle).await?;
            Ok((items, adapter.currency()))
        }
        #[cfg(feature = "tencentcloud")]
        "tencentcloud" => {
            use super::tencentcloud::TencentCloudBillingAdapter;
            use super::traits::BillingProvider;
            let adapter = TencentCloudBillingAdapter::from_config(config)?;
            let items = adapter.query_bill_items(billing_cycle).await?;
            Ok((items, adapter.currency()))
        }
        #[cfg(feature = "volcengine")]
        "volcengine" => {
            use super::traits::BillingProvider;
            use super::volcengine::VolcengineBillingAdapter;
            let adapter = VolcengineBillingAdapter::from_config(config)?;
            let items = adapter.query_bill_items(billing_cycle).await?;
            Ok((items, adapter.currency()))
        }
        #[cfg(feature = "ucloud")]
        "ucloud" => {
            use super::traits::BillingProvider;
            use super::ucloud::UCloudBillingAdapter;
            let adapter = UCloudBillingAdapter::from_config(config)?;
            let items = adapter.query_bill_items(billing_cycle).await?;
            Ok((items, adapter.currency()))
        }
        #[cfg(feature = "gcp")]
        "gcp" => {
            use super::gcp::GcpBillingAdapter;
            use super::traits::BillingProvider;
            let adapter = GcpBillingAdapter::from_config(config).await?;
            let items = adapter.query_bill_items(billing_cycle).await?;
            Ok((items, adapter.currency()))
        }
        #[cfg(feature = "cloudflare")]
        "cloudflare" => {
            use super::cloudflare::CloudflareBillingAdapter;
            use super::traits::BillingProvider;
            let adapter = CloudflareBillingAdapter::from_config(config)?;
            let items = adapter.query_bill_items(billing_cycle).await?;
            Ok((items, adapter.currency()))
        }
        #[cfg(feature = "vastai")]
        "vastai" => {
            use super::traits::BillingProvider;
            use super::vastai::VastaiBillingAdapter;
            let adapter = VastaiBillingAdapter::from_config(config)?;
            let items = adapter.query_bill_items(billing_cycle).await?;
            Ok((items, adapter.currency()))
        }
        _ => Err(BillingError::ServiceError(format!(
            "Unknown or disabled provider: {}",
            provider_name
        ))),
    }
}
