# cbilling

<div align="center">

[![CI](https://github.com/Liberxue/cbilling/actions/workflows/ci.yml/badge.svg)](https://github.com/Liberxue/cbilling/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/cbilling.svg)](https://crates.io/crates/cbilling)
[![docs.rs](https://docs.rs/cbilling/badge.svg)](https://docs.rs/cbilling)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)

**Multi-cloud billing CLI & SDK for Rust** — query, compare, and visualize costs from AWS, GCP, Alibaba Cloud, Tencent Cloud, Volcengine, UCloud, and Cloudflare in one terminal.

[![asciicast](https://asciinema.org/a/Wgsc4BxlnGlc92rl.svg)](https://asciinema.org/a/Wgsc4BxlnGlc92rl)

</div>

Every provider API is covered: cost breakdown by product, region-level detail, month-over-month comparison, multi-account aggregation, and CSV export — all from a single binary or as a Rust library.

## Install

### Shell (macOS / Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/Liberxue/cbilling/main/scripts/install.sh | bash
```

### Cargo

```bash
cargo install cbilling-cli
```

### Manual Download

```bash
VERSION=$(curl -fsSL https://api.github.com/repos/Liberxue/cbilling/releases/latest | grep tag_name | cut -d '"' -f4)

# macOS (Apple Silicon)
curl -fsSL "https://github.com/Liberxue/cbilling/releases/download/${VERSION}/cbilling-${VERSION}-aarch64-apple-darwin.tar.gz" | tar xz -C /usr/local/bin/

# macOS (Intel)
curl -fsSL "https://github.com/Liberxue/cbilling/releases/download/${VERSION}/cbilling-${VERSION}-x86_64-apple-darwin.tar.gz" | tar xz -C /usr/local/bin/

# Linux (x86_64)
curl -fsSL "https://github.com/Liberxue/cbilling/releases/download/${VERSION}/cbilling-${VERSION}-x86_64-unknown-linux-gnu.tar.gz" | tar xz -C /usr/local/bin/

# Linux (aarch64)
curl -fsSL "https://github.com/Liberxue/cbilling/releases/download/${VERSION}/cbilling-${VERSION}-aarch64-unknown-linux-gnu.tar.gz" | tar xz -C /usr/local/bin/
```

### Rust Library

```toml
[dependencies]
cbilling = "0.1"
# Or pick only what you need
cbilling = { version = "0.1", default-features = false, features = ["aws", "gcp"] }
```

## Supported Providers

| Provider | Feature Flag | API | Currency |
|:---------|:------------|:----|:---------|
| Alibaba Cloud (Aliyun) | `aliyun` | BSS OpenAPI | CNY |
| AWS | `aws` | Cost Explorer | USD |
| Tencent Cloud | `tencentcloud` | Billing API v3 | CNY |
| Volcengine | `volcengine` | Billing API | CNY |
| UCloud | `ucloud` | UBill API | CNY |
| Google Cloud (GCP) | `gcp` | Cloud Billing + BigQuery | USD |
| Cloudflare | `cloudflare` | Billing API v4 | USD |

## Usage

### TUI Dashboard (default)

```bash
cbilling
```

Launches a full-screen terminal UI with:

- Provider tabs with real-time cost totals
- Horizontal bar chart showing cost distribution
- Sortable product table with MoM (month-over-month) comparison
- Region detail expansion, search/filter, mouse scroll
- Month navigation with `←`/`→`

### CLI Commands

```bash
# List configured providers
cbilling providers

# Query a specific provider
cbilling query aliyun
cbilling query aws --month 2026-03

# JSON output (for scripting & AI agents)
cbilling query gcp --format json
cbilling summary --format json

# Export to CSV
cbilling query tencentcloud --csv billing.csv

# Summary across all providers
cbilling summary
cbilling summary --month 2026-01
```

All commands support `--format json` for machine-readable output.

### Output Examples

```
$ cbilling providers
PROVIDER         STATUS
------------------------------
aliyun           ready
tencentcloud     ready
aws              ready
gcp              ready

4 provider(s) configured
```

```
$ cbilling summary --month 2026-03
Querying 4 provider(s) for 2026-03...

PROVIDER                   COST CUR   PRODUCTS
------------------------------------------------
aliyun                 98765.43 CNY         65
tencentcloud           33618.72 CNY         12
aws                     4332.39 USD         25
gcp                     1287.64 USD         14
------------------------------------------------
TOTAL                 132384.15 CNY
TOTAL                   5620.03 USD
```

### TUI Keyboard Shortcuts

| Key | Action |
|:----|:-------|
| `j` / `k` / `↑` / `↓` | Navigate up/down |
| `Ctrl+f` / `Ctrl+b` | Page down/up |
| `g` / `G` | Jump to top/bottom |
| `Tab` / `h` / `l` | Switch provider tab |
| `←` / `→` | Previous/next month |
| `s` / `S` | Cycle sort column / toggle direction |
| `1`–`5` | Sort by column # |
| `Enter` | Expand/collapse region details |
| `/` | Search/filter |
| `Esc` | Clear filter |
| `r` | Refresh |
| `?` | Help |
| `q` | Quit |

## Configuration

### Environment Variables (single account)

| Provider | Variables |
|:---------|:---------|
| Aliyun | `ALIBABA_CLOUD_ACCESS_KEY_ID` `ALIBABA_CLOUD_ACCESS_KEY_SECRET` |
| AWS | `AWS_ACCESS_KEY_ID` `AWS_SECRET_ACCESS_KEY` or `~/.aws/credentials` |
| Tencent Cloud | `TENCENTCLOUD_SECRET_ID` `TENCENTCLOUD_SECRET_KEY` |
| Volcengine | `VOLCENGINE_ACCESS_KEY_ID` `VOLCENGINE_SECRET_ACCESS_KEY` |
| UCloud | `UCLOUD_PUBLIC_KEY` `UCLOUD_PRIVATE_KEY` `UCLOUD_PROJECT_ID` |
| GCP | `GCP_PROJECT_ID` + `GCP_SERVICE_ACCOUNT_JSON` |
| Cloudflare | `CLOUDFLARE_ACCOUNT_ID` + `CLOUDFLARE_API_TOKEN` |

### Multi-Account JSON (recommended)

Use `<PROVIDER>_ACCOUNTS` env vars for multi-account setups:

```bash
export ALIYUN_ACCOUNTS='[
  { "id": "prod", "name": "Production", "access_key_id": "LTAI...", "access_key_secret": "HkbP..." },
  { "id": "dev",  "name": "Development", "access_key_id": "LTAI...", "access_key_secret": "Jx9a..." }
]'

export AWS_ACCOUNTS='[
  { "id": "main", "name": "Main Account", "access_key_id": "AKIA...", "secret_access_key": "wJal...", "region": "us-east-1" }
]'

export GCP_ACCOUNTS='[
  { "id": "proj-1", "name": "My GCP Project", "project_id": "my-project-123", "private_key": "{...service account JSON...}" }
]'
```

<details>
<summary>All supported account fields</summary>

| Field | Type | Description |
|:------|:-----|:-----------|
| `id` | string | Unique account identifier |
| `name` | string | Display name |
| `access_key_id` | string? | Aliyun / AWS / Volcengine access key |
| `access_key_secret` | string? | Aliyun access key secret |
| `secret_access_key` | string? | AWS / Volcengine secret access key |
| `secret_id` / `secret_key` | string? | Tencent Cloud credentials |
| `public_key` / `private_key` | string? | UCloud credentials |
| `project_id` | string? | UCloud / GCP project ID |
| `region` | string? | Default region |
| `enabled` | bool | Include this account (default: `true`) |

</details>

## SDK Usage

### Unified Service (recommended)

```rust
use cbilling::service::CloudBillingService;

#[tokio::main]
async fn main() -> cbilling::Result<()> {
    let providers = CloudBillingService::get_configured_providers();
    let data = CloudBillingService::query_provider("aws", "2026-03").await?;

    println!("{}: {:.2} {} ({} products)",
        data.provider, data.total_cost, data.currency, data.products.len());
    Ok(())
}
```

### Direct Provider Client

```rust
use cbilling::providers::aliyun::AliyunBillingClient;

let client = AliyunBillingClient::new("key".into(), "secret".into());
let resp = client.query_instance_bill("2026-03", Some(1), Some(100), None).await?;
```

<details>
<summary>All provider client APIs</summary>

```rust
// Aliyun
AliyunBillingClient::new(key, secret)
    .query_instance_bill(cycle, page, size, product)
    .query_account_bill(cycle, page, size)

// AWS (Cost Explorer — forced us-east-1)
AwsBillingClient::new(key, secret, region)
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

</details>

### Data Model

```rust
BillingData { billing_cycle, provider, total_cost, currency, products: Vec<ProductCost> }
ProductCost { product_name, product_code, cost, count, regions, region_details }
RegionDetail { region, cost, count }
```

## Feature Flags

| Feature | Description |
|:--------|:-----------|
| `aliyun` | Alibaba Cloud (default) |
| `tencentcloud` | Tencent Cloud (default) |
| `aws` | AWS Cost Explorer (default) |
| `volcengine` | Volcengine (default) |
| `ucloud` | UCloud (default) |
| `gcp` | Google Cloud (default) |
| `cloudflare` | Cloudflare (default) |
| `all-providers` | All of the above |

## Skill (AI Integration)

Install the skill to give Claude full knowledge of all `cbilling` commands:

```bash
npx skills add Liberxue/cbilling
```

Once installed, Claude can query your cloud billing data directly:

```
claude> What's my total cloud spend this month?
claude> Which Aliyun services cost the most in March?
claude> Compare AWS costs between February and March
```

All commands output structured JSON (`--format json`) for AI-agent tool-calling.

## Project Structure

```
cbilling/
  src/                          # Library crate
    providers/                  # One module per cloud provider
    service.rs                  # Unified query API
    models.rs                   # Shared data types
    error.rs                    # Error types
  crates/cbilling-cli/          # CLI + TUI binary
    src/
      views/                    # TUI view components
      widgets/                  # Reusable TUI widgets
      styles.rs                 # Semantic style system
  scripts/                      # install.sh, record-demo.sh
  examples/                     # Per-provider usage examples
```

## License

Apache-2.0
