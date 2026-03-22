# cbilling

[![CI](https://github.com/Liberxue/cbilling/actions/workflows/ci.yml/badge.svg)](https://github.com/Liberxue/cbilling/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/cbilling.svg)](https://crates.io/crates/cbilling)
[![docs.rs](https://docs.rs/cbilling/badge.svg)](https://docs.rs/cbilling)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)

Multi-cloud billing SDK for Rust. Query billing data from 7 cloud providers through a unified API.

> Looking for the terminal UI / CLI tool? See **[cbilling-cli](crates/cbilling-cli/README.md)**.

## Supported Providers

| Provider | Feature | API | Currency |
|----------|---------|-----|----------|
| Alibaba Cloud | `aliyun` | BSS OpenAPI | CNY |
| AWS | `aws` | Cost Explorer | USD |
| Tencent Cloud | `tencentcloud` | Billing API v3 | CNY |
| Volcengine | `volcengine` | Billing API | CNY |
| UCloud | `ucloud` | UBill API | CNY |
| Google Cloud | `gcp` | Cloud Billing + BigQuery | USD |
| Cloudflare | `cloudflare` | Billing API v4 | USD |

## Install

```toml
[dependencies]
# All providers (default)
cbilling = "0.1"

# Or pick only what you need
cbilling = { version = "0.1", default-features = false, features = ["aws", "aliyun"] }
```

## Quick Start

### Direct Provider Client

```rust
use cbilling::providers::aliyun::AliyunBillingClient;

#[tokio::main]
async fn main() -> cbilling::Result<()> {
    let client = AliyunBillingClient::new(
        "your_access_key_id".into(),
        "your_access_key_secret".into(),
    );

    let response = client
        .query_instance_bill("2026-03", Some(1), Some(100), None)
        .await?;

    if let Some(data) = response.data {
        println!("Total: {} records", data.total_count);
    }
    Ok(())
}
```

### Unified Service (recommended)

Normalizes all providers into a single `BillingData` struct with automatic pagination and credential loading from environment variables:

```rust
use cbilling::service::CloudBillingService;

#[tokio::main]
async fn main() -> cbilling::Result<()> {
    // List available providers (auto-detected from env vars)
    let providers = CloudBillingService::get_configured_providers();
    println!("Configured: {:?}", providers);

    // Query any provider by name
    let data = CloudBillingService::query_provider("aws", "2026-03").await?;

    println!("{}: {:.2} {} ({} products)",
        data.provider, data.total_cost, data.currency, data.products.len());

    for p in &data.products {
        println!("  {:<30} {:>10.2}  qty={:?}  regions={:?}",
            p.product_name, p.cost, p.count, p.regions);
    }
    Ok(())
}
```

## API

### Provider Clients

Each provider exposes a typed client:

```rust
// Aliyun
AliyunBillingClient::new(key, secret)
    .query_instance_bill(cycle, page, size, product)
    .query_account_bill(cycle, page, size)

// AWS
AwsBillingClient::new_with_default_credentials(region)
    .get_cost_and_usage(start, end, granularity, metrics, group_by)

// Tencent Cloud
TencentCloudBillingClient::new(id, key, region)
    .get_bill_summary(month)
    .get_bill_detail(month, offset, limit)

// Volcengine
VolcengineBillingClient::new(key, secret, region)
    .list_bill_detail(period, limit, offset)

// UCloud
UCloudBillingClient::new(pub_key, priv_key, project)
    .query_bill_list(start_ts, end_ts, offset, limit)

// GCP
GcpBillingClient::new(project, sa_json)
    .list_billing_accounts()
    .query_billing_costs(cycle, dataset, table)

// Cloudflare
CloudflareBillingClient::new_with_token(account_id, token)
    .get_subscriptions()
    .get_all_billing_history()
```

### Unified Service

```rust
CloudBillingService::get_configured_providers() -> Vec<String>
CloudBillingService::query_provider(provider, cycle) -> Result<BillingData>
CloudBillingService::load_accounts_for_provider(provider) -> Result<Vec<CloudAccountConfig>>
```

### Data Model

```rust
pub struct BillingData {
    pub billing_cycle: String,       // "2026-03"
    pub provider: String,            // "aliyun", "aws", etc.
    pub total_cost: f64,
    pub currency: String,            // "CNY", "USD"
    pub products: Vec<ProductCost>,
}

pub struct ProductCost {
    pub product_name: String,        // "Elastic Compute Service"
    pub product_code: String,        // "ecs"
    pub cost: f64,
    pub count: Option<u32>,          // resource instance count
    pub regions: Vec<String>,        // ["cn-beijing", "cn-shanghai"]
    pub region_details: Vec<RegionDetail>,
}

pub struct RegionDetail {
    pub region: String,
    pub cost: f64,
    pub count: u32,
}
```

## Configuration

Each provider reads credentials from environment variables:

| Provider | Environment Variables |
|----------|----------------------|
| Aliyun | `ALIBABA_CLOUD_ACCESS_KEY_ID`, `ALIBABA_CLOUD_ACCESS_KEY_SECRET` |
| AWS | `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY` (or `~/.aws/credentials`) |
| Tencent Cloud | `TENCENTCLOUD_SECRET_ID`, `TENCENTCLOUD_SECRET_KEY` |
| Volcengine | `VOLCENGINE_ACCESS_KEY_ID`, `VOLCENGINE_SECRET_ACCESS_KEY` |
| UCloud | `UCLOUD_PUBLIC_KEY`, `UCLOUD_PRIVATE_KEY`, `UCLOUD_PROJECT_ID` |
| GCP | `GCP_PROJECT_ID` + `GCP_SERVICE_ACCOUNT_JSON` or `GOOGLE_APPLICATION_CREDENTIALS` |
| Cloudflare | `CLOUDFLARE_ACCOUNT_ID`, `CLOUDFLARE_API_TOKEN` |

Multi-account JSON config is also supported via `<PROVIDER>_ACCOUNTS` env vars. See [examples/](examples/).

## Feature Flags

| Feature | Description |
|---------|-------------|
| `aliyun` | Alibaba Cloud (default) |
| `tencentcloud` | Tencent Cloud (default) |
| `aws` | AWS Cost Explorer (default, adds `aws-sdk-costexplorer`) |
| `volcengine` | Volcengine (default) |
| `ucloud` | UCloud (default) |
| `gcp` | Google Cloud (default, adds `rsa` + `pkcs8`) |
| `cloudflare` | Cloudflare (default) |
| `all-providers` | All of the above |

## Project Structure

```
cbilling/
  src/                        # Library crate (cbilling)
    providers/                # One module per cloud provider
    service.rs                # Unified query API
    models.rs                 # Shared data types
    error.rs                  # Error types
  crates/
    cbilling-cli/        # CLI + TUI crate (cbilling-cli)
  examples/                   # Per-provider usage examples
```

## Examples

```bash
cargo run --example aliyun_billing --features aliyun
cargo run --example aws_billing --features aws
cargo run --example tencentcloud_billing --features tencentcloud
cargo run --example volcengine_billing --features volcengine
cargo run --example ucloud_billing --features ucloud
cargo run --example gcp_billing --features gcp
cargo run --example cloudflare_billing --features cloudflare
```

## License

Apache-2.0
