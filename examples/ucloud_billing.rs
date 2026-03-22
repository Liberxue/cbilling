// Example: Query UCloud billing data
//
// Run with: cargo run --example ucloud_billing --features ucloud
//
// Required environment variables:
// - UCLOUD_PUBLIC_KEY
// - UCLOUD_PRIVATE_KEY
// - UCLOUD_PROJECT_ID

use cbilling::providers::ucloud::UCloudBillingClient;

#[tokio::main]
async fn main() -> cbilling::Result<()> {
    tracing_subscriber::fmt::init();

    println!("[UCloud] Billing Example\n");

    let public_key = std::env::var("UCLOUD_PUBLIC_KEY").expect("UCLOUD_PUBLIC_KEY not set");
    let private_key = std::env::var("UCLOUD_PRIVATE_KEY").expect("UCLOUD_PRIVATE_KEY not set");
    let project_id = std::env::var("UCLOUD_PROJECT_ID").expect("UCLOUD_PROJECT_ID not set");

    let client = UCloudBillingClient::new(public_key, private_key, project_id);

    let now = chrono::Utc::now().timestamp();
    let thirty_days_ago = now - 30 * 86400;

    println!(
        "Querying bill data from {} to {}",
        chrono::DateTime::from_timestamp(thirty_days_ago, 0)
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
        chrono::DateTime::from_timestamp(now, 0)
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
    );

    let response = client
        .query_bill_list(thirty_days_ago, now, Some(0), Some(10))
        .await?;

    println!("Query successful!\n");
    println!("Total records: {:?}", response.total_count.unwrap_or(0));

    if let Some(items) = &response.items {
        println!("\nBill items (showing up to 10):");
        for (i, item) in items.iter().enumerate() {
            println!(
                "  {}. {} | Resource: {} | Amount: {}",
                i + 1,
                item.resource_type.as_deref().unwrap_or("Unknown"),
                item.resource_name.as_deref().unwrap_or("N/A"),
                item.show_amount.as_deref().unwrap_or("0"),
            );
        }
    } else {
        println!("No billing data found for this period.");
    }

    Ok(())
}
