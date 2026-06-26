// Copyright 2025 OpenObserve Inc.
// SPDX-License-Identifier: AGPL-3.0

//! Cloud Provider Integration
//!
//! This module manages integration with different cloud providers.
//! Each provider is behind a feature flag for modular compilation.
//!
//! ## Supported Providers
//!
//! - **Aliyun (Alibaba Cloud)** - feature: `aliyun`
//! - **Tencent Cloud** - feature: `tencentcloud`
//! - **AWS** - feature: `aws`
//! - **Volcengine** - feature: `volcengine`
//! - **UCloud** - feature: `ucloud`
//! - **Azure** - feature: `azure` (planned)

#[cfg(feature = "aliyun")]
pub mod aliyun;

#[cfg(feature = "aws")]
pub mod aws;

#[cfg(feature = "tencentcloud")]
pub mod tencentcloud;

#[cfg(feature = "ucloud")]
pub mod ucloud;

#[cfg(feature = "volcengine")]
pub mod volcengine;

#[cfg(feature = "gcp")]
pub mod gcp;

#[cfg(feature = "cloudflare")]
pub mod cloudflare;

#[cfg(feature = "vastai")]
pub mod vastai;

// Azure support planned for future
//#[cfg(feature = "azure")]
//pub mod azure;

pub mod aggregation;
pub mod factory;
pub mod registry;
pub mod traits;

// Provider trait (can be expanded later with actual implementations)
pub use registry::init;
