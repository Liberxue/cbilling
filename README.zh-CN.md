# cbilling

<div align="center">

[![CI](https://github.com/Liberxue/cbilling/actions/workflows/ci.yml/badge.svg)](https://github.com/Liberxue/cbilling/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/cbilling.svg)](https://crates.io/crates/cbilling)
[![docs.rs](https://docs.rs/cbilling/badge.svg)](https://docs.rs/cbilling)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)

[English](README.md) | **简体中文**

**Rust 多云账单 CLI 与 SDK** —— 在一个终端里查询、对比并可视化来自 AWS、GCP、阿里云、腾讯云、火山引擎、UCloud、Cloudflare 和 Vast.ai 的费用。

[![asciicast](https://asciinema.org/a/Wgsc4BxlnGlc92rl.svg)](https://asciinema.org/a/Wgsc4BxlnGlc92rl)

</div>

覆盖每个云厂商的账单 API：按产品的费用拆分、地域级明细、环比对比、多账号聚合，以及 CSV 导出 —— 全部来自一个二进制文件，也可作为 Rust 库使用。

## 安装

### Shell（macOS / Linux）

```bash
curl -fsSL https://raw.githubusercontent.com/Liberxue/cbilling/main/scripts/install.sh | bash
```

### Cargo

```bash
cargo install cbilling-cli
```

### 手动下载

```bash
VERSION=$(curl -fsSL https://api.github.com/repos/Liberxue/cbilling/releases/latest | grep tag_name | cut -d '"' -f4)

# macOS（Apple Silicon）
curl -fsSL "https://github.com/Liberxue/cbilling/releases/download/${VERSION}/cbilling-${VERSION}-aarch64-apple-darwin.tar.gz" | tar xz -C /usr/local/bin/

# macOS（Intel）
curl -fsSL "https://github.com/Liberxue/cbilling/releases/download/${VERSION}/cbilling-${VERSION}-x86_64-apple-darwin.tar.gz" | tar xz -C /usr/local/bin/

# Linux（x86_64）
curl -fsSL "https://github.com/Liberxue/cbilling/releases/download/${VERSION}/cbilling-${VERSION}-x86_64-unknown-linux-gnu.tar.gz" | tar xz -C /usr/local/bin/

# Linux（aarch64）
curl -fsSL "https://github.com/Liberxue/cbilling/releases/download/${VERSION}/cbilling-${VERSION}-aarch64-unknown-linux-gnu.tar.gz" | tar xz -C /usr/local/bin/
```

### 作为 Rust 库

```toml
[dependencies]
cbilling = "0.1"
# 或只启用你需要的部分
cbilling = { version = "0.1", default-features = false, features = ["aws", "gcp"] }
```

## 支持的厂商

| 厂商 | Feature 标志 | API | 币种 |
|:-----|:------------|:----|:-----|
| 阿里云（Aliyun） | `aliyun` | BSS OpenAPI | CNY |
| AWS | `aws` | Cost Explorer | USD |
| 腾讯云 | `tencentcloud` | Billing API v3 | CNY |
| 火山引擎 | `volcengine` | Billing API | CNY |
| UCloud | `ucloud` | UBill API | CNY |
| 谷歌云（GCP） | `gcp` | Cloud Billing + BigQuery | USD |
| Cloudflare | `cloudflare` | Billing API v4 | USD |
| Vast.ai | `vastai` | Charges API（v0） | USD |

## 使用

### TUI 仪表盘（默认）

```bash
cbilling
```

启动一个全屏终端 UI，包含：

- 带实时费用合计的厂商标签页
- 展示费用分布的水平条形图
- 可排序的产品表格，带环比（MoM）对比
- 地域明细展开、搜索 / 过滤、鼠标滚动
- 用 `←` / `→` 切换月份

### CLI 命令

```bash
# 列出已配置的厂商
cbilling providers

# 查询指定厂商
cbilling query aliyun
cbilling query aws --month 2026-03

# JSON 输出（用于脚本与 AI agent）
cbilling query gcp --format json
cbilling summary --format json

# 导出为 CSV
cbilling query tencentcloud --csv billing.csv

# 跨所有厂商的汇总
cbilling summary
cbilling summary --month 2026-01
```

所有命令都支持 `--format json` 以输出机器可读的结果。

### 输出示例

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

### TUI 快捷键

| 按键 | 操作 |
|:-----|:-----|
| `j` / `k` / `↑` / `↓` | 上下导航 |
| `Ctrl+f` / `Ctrl+b` | 向下 / 向上翻页 |
| `g` / `G` | 跳到顶部 / 底部 |
| `Tab` / `h` / `l` | 切换厂商标签页 |
| `←` / `→` | 上一个 / 下一个月 |
| `s` / `S` | 切换排序列 / 切换方向 |
| `1`–`5` | 按第 # 列排序 |
| `Enter` | 展开 / 收起地域明细 |
| `/` | 搜索 / 过滤 |
| `Esc` | 清除过滤 |
| `r` | 刷新 |
| `?` | 帮助 |
| `q` | 退出 |

## 配置

### 环境变量（单账号）

| 厂商 | 变量 |
|:-----|:-----|
| 阿里云 | `ALIBABA_CLOUD_ACCESS_KEY_ID` `ALIBABA_CLOUD_ACCESS_KEY_SECRET` |
| AWS | `AWS_ACCESS_KEY_ID` `AWS_SECRET_ACCESS_KEY` 或 `~/.aws/credentials` |
| 腾讯云 | `TENCENTCLOUD_SECRET_ID` `TENCENTCLOUD_SECRET_KEY` |
| 火山引擎 | `VOLCENGINE_ACCESS_KEY_ID` `VOLCENGINE_SECRET_ACCESS_KEY` |
| UCloud | `UCLOUD_PUBLIC_KEY` `UCLOUD_PRIVATE_KEY` `UCLOUD_PROJECT_ID` |
| GCP | `GCP_PROJECT_ID` + `GCP_SERVICE_ACCOUNT_JSON` |
| Cloudflare | `CLOUDFLARE_ACCOUNT_ID` + `CLOUDFLARE_API_TOKEN` |
| Vast.ai | `VASTAI_API_KEY`（来自 cloud.vast.ai/manage-keys 的 Bearer key） |

### 多账号 JSON（推荐）

对多账号场景使用 `<PROVIDER>_ACCOUNTS` 环境变量：

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
<summary>全部支持的账号字段</summary>

| 字段 | 类型 | 说明 |
|:-----|:-----|:-----|
| `id` | string | 唯一账号标识 |
| `name` | string | 显示名称 |
| `access_key_id` | string? | 阿里云 / AWS / 火山引擎 access key |
| `access_key_secret` | string? | 阿里云 access key secret |
| `secret_access_key` | string? | AWS / 火山引擎 secret access key |
| `secret_id` / `secret_key` | string? | 腾讯云凭证 |
| `public_key` / `private_key` | string? | UCloud 凭证 |
| `project_id` | string? | UCloud / GCP 项目 ID |
| `region` | string? | 默认地域 |
| `enabled` | bool | 是否启用该账号（默认 `true`） |

</details>

## SDK 用法

### 统一服务（推荐）

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

### 直接使用厂商客户端

```rust
use cbilling::providers::aliyun::AliyunBillingClient;

let client = AliyunBillingClient::new("key".into(), "secret".into());
let resp = client.query_instance_bill("2026-03", Some(1), Some(100), None).await?;
```

<details>
<summary>全部厂商客户端 API</summary>

```rust
// 阿里云
AliyunBillingClient::new(key, secret)
    .query_instance_bill(cycle, page, size, product)
    .query_account_bill(cycle, page, size)

// AWS（Cost Explorer —— 强制使用 us-east-1）
AwsBillingClient::new(key, secret, region)
    .get_cost_and_usage(start, end, granularity, metrics, group_by)

// 腾讯云
TencentCloudBillingClient::new(id, key, region)
    .get_bill_summary(month)
    .get_bill_detail(month, offset, limit)

// 火山引擎
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

// Vast.ai
VastaiBillingClient::new(api_key)
    .get_all_charges(start_ts, end_ts)
```

</details>

### 数据模型

```rust
BillingData { billing_cycle, provider, total_cost, currency, products: Vec<ProductCost> }
ProductCost { product_name, product_code, cost, count, regions, region_details }
RegionDetail { region, cost, count }
```

## Feature 标志

| Feature | 说明 |
|:--------|:-----|
| `aliyun` | 阿里云（默认） |
| `tencentcloud` | 腾讯云（默认） |
| `aws` | AWS Cost Explorer（默认） |
| `volcengine` | 火山引擎（默认） |
| `ucloud` | UCloud（默认） |
| `gcp` | 谷歌云（默认） |
| `cloudflare` | Cloudflare（默认） |
| `vastai` | Vast.ai（默认） |
| `all-providers` | 以上全部 |

## Skill（AI 集成）

安装该 skill，让 Claude 完整掌握所有 `cbilling` 命令：

```bash
npx skills add Liberxue/cbilling
```

安装后，Claude 可直接查询你的云账单数据：

```
claude> 我这个月云上总花费是多少？
claude> 三月份哪些阿里云服务花费最高？
claude> 对比二月和三月的 AWS 费用
```

所有命令都以结构化 JSON（`--format json`）输出，便于 AI agent 进行工具调用。

## 项目结构

```
cbilling/
  src/                          # 库 crate
    providers/                  # 每个云厂商一个模块
    service.rs                  # 统一查询 API
    models.rs                   # 共享数据类型
    error.rs                    # 错误类型
  crates/cbilling-cli/          # CLI + TUI 二进制
    src/
      views/                    # TUI 视图组件
      widgets/                  # 可复用 TUI 组件
      styles.rs                 # 语义化样式系统
  scripts/                      # install.sh、record-demo.sh
  examples/                     # 各厂商用法示例
```

## 许可证

Apache-2.0
