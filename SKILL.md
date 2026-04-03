---
name: cbilling
description: Query and analyze multi-cloud billing data from AWS, GCP, Alibaba Cloud, Tencent Cloud, Volcengine, UCloud, and Cloudflare. Use this skill when the user asks about cloud costs, billing, spending, or wants to compare costs across cloud providers.
---

# cbilling — Multi-cloud Billing CLI

You have access to the `cbilling` CLI tool that queries real billing data from 7 cloud providers.

## Commands

### List configured providers

```bash
cbilling providers --format json
```

### Query a specific provider

```bash
# Current month
cbilling query <PROVIDER> --format json

# Specific month
cbilling query <PROVIDER> --month 2026-03 --format json
```

Providers: `aliyun`, `aws`, `tencentcloud`, `volcengine`, `ucloud`, `gcp`, `cloudflare`

### Summary across all providers

```bash
cbilling summary --format json
cbilling summary --month 2026-03 --format json
```

### Export to CSV

```bash
cbilling query aliyun --csv /tmp/billing.csv
```

## IMPORTANT: Always use `--format json`

Always add `--format json` to get structured output you can parse. Without it, the output is a human-readable table that is harder to analyze.

## JSON Response Schema

### `cbilling query <provider> --format json`

```json
{
  "billing_cycle": "2026-03",
  "provider": "aws",
  "total_cost": 4332.39,
  "currency": "USD",
  "products": [
    {
      "product_name": "Amazon Elastic Compute Cloud",
      "product_code": "AmazonEC2",
      "cost": 1792.78,
      "count": 12,
      "regions": ["us-east-1", "us-west-2"],
      "region_details": [
        {"region": "us-east-1", "cost": 1200.50, "count": 8},
        {"region": "us-west-2", "cost": 592.28, "count": 4}
      ]
    }
  ]
}
```

### `cbilling summary --format json`

```json
[
  {"provider": "aliyun", "billing_cycle": "2026-03", "total_cost": 98765.43, "currency": "CNY", "product_count": 65},
  {"provider": "aws", "billing_cycle": "2026-03", "total_cost": 4332.39, "currency": "USD", "product_count": 25}
]
```

## Example Workflows

### "What's my total cloud spend this month?"

```bash
cbilling summary --format json
```

Then sum costs by currency and present a breakdown.

### "Which AWS services cost the most?"

```bash
cbilling query aws --format json
```

Sort products by cost descending, show top 5.

### "Compare last month vs this month for Aliyun"

```bash
cbilling query aliyun --month 2026-03 --format json
cbilling query aliyun --month 2026-02 --format json
```

Compare `total_cost` and per-product costs between the two months.

### "Show me cost breakdown by region for GCP"

```bash
cbilling query gcp --format json
```

Extract `region_details` from each product and aggregate by region.

## Guidelines

- Always use `--format json` for structured output
- When comparing months, run the query twice with different `--month` values
- Currency may differ between providers (CNY for Chinese clouds, USD for AWS/GCP/Cloudflare)
- If a provider errors, suggest the user check their credentials configuration
- For the TUI dashboard, tell the user to run `cbilling` with no arguments
