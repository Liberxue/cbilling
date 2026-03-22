// Copyright 2025 OpenObserve Inc.
// SPDX-License-Identifier: AGPL-3.0

//! Provider Registry
//!
//! Manages registration and initialization of cloud providers

use crate::error::Result;

/// Initialize provider registry
#[allow(clippy::vec_init_then_push)]
pub async fn init() -> Result<()> {
    tracing::info!("Initializing cloud billing provider registry");

    let mut enabled_providers = Vec::new();

    #[cfg(feature = "aliyun")]
    enabled_providers.push("aliyun");

    #[cfg(feature = "tencentcloud")]
    enabled_providers.push("tencentcloud");

    #[cfg(feature = "aws")]
    enabled_providers.push("aws");

    #[cfg(feature = "volcengine")]
    enabled_providers.push("volcengine");

    #[cfg(feature = "ucloud")]
    enabled_providers.push("ucloud");

    #[cfg(feature = "gcp")]
    enabled_providers.push("gcp");

    #[cfg(feature = "cloudflare")]
    enabled_providers.push("cloudflare");

    #[cfg(feature = "azure")]
    enabled_providers.push("azure");

    if enabled_providers.is_empty() {
        tracing::warn!("No cloud providers enabled! Enable at least one provider feature.");
    } else {
        tracing::info!("Enabled providers: {}", enabled_providers.join(", "));
    }

    Ok(())
}
