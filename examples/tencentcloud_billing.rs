// Example: Query Tencent Cloud billing data
//
// Run with: cargo run --example tencentcloud_billing --features tencentcloud
//
// Required environment variables:
// - TENCENTCLOUD_SECRET_ID
// - TENCENTCLOUD_SECRET_KEY
// - TENCENTCLOUD_REGION (optional, defaults to ap-guangzhou)

use cbilling::providers::tencentcloud::TencentCloudBillingClient;

#[tokio::main]
async fn main() -> cbilling::Result<()> {
    tracing_subscriber::fmt::init();

    println!("[TencentCloud] Billing Example\n");

    let secret_id =
        std::env::var("TENCENTCLOUD_SECRET_ID").expect("TENCENTCLOUD_SECRET_ID not set");
    let secret_key =
        std::env::var("TENCENTCLOUD_SECRET_KEY").expect("TENCENTCLOUD_SECRET_KEY not set");
    let region =
        std::env::var("TENCENTCLOUD_REGION").unwrap_or_else(|_| "ap-guangzhou".to_string());

    let client = TencentCloudBillingClient::new(secret_id, secret_key, region);

    let month = chrono::Local::now().format("%Y-%m").to_string();
    println!("Querying bill summary for: {}", month);

    let summary = client.get_bill_summary(&month).await?;

    println!(
        "Query successful! (RequestId: {})\n",
        summary.response.request_id
    );

    if let Some(total) = &summary.response.summary_total {
        println!("Total cost: {} CNY", total.real_total_cost);
        println!("Original cost: {} CNY", total.total_cost);
    }

    if let Some(items) = &summary.response.summary_overview {
        println!("\nServices ({} total):", items.len());
        for (i, item) in items.iter().enumerate().take(10) {
            println!(
                "  {}. {} - {} CNY (original: {} CNY)",
                i + 1,
                item.business_code_name.as_deref().unwrap_or("Unknown"),
                item.real_total_cost,
                item.total_cost,
            );
        }
        if items.len() > 10 {
            println!("  ... and {} more", items.len() - 10);
        }
    }

    println!("\nQuerying bill details...");
    let detail = client.get_bill_detail(&month, 0, 5).await?;

    if let Some(items) = &detail.response.detail_set {
        println!("Detail records (showing up to 5):");
        for (i, item) in items.iter().enumerate() {
            println!(
                "  {}. {} | Region: {} | Cost: {} CNY",
                i + 1,
                item.business_code.as_deref().unwrap_or("Unknown"),
                item.region_name.as_deref().unwrap_or("N/A"),
                item.real_total_cost.as_deref().unwrap_or("0"),
            );
        }
    }

    Ok(())
}
