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

    let billing_cycle = chrono::Utc::now().format("%Y-%m").to_string();

    println!("Querying bill data for cycle {}", billing_cycle);

    let response = client
        .query_bill_list(&billing_cycle, Some(0), Some(10))
        .await?;

    println!("Query successful!\n");
    println!("Total records: {:?}", response.total_count.unwrap_or(0));

    if let Some(items) = &response.items {
        println!("\nBill items (showing up to 10):");
        for (i, item) in items.iter().enumerate() {
            println!(
                "  {}. {} | Resource: {} | Amount: {:.2}",
                i + 1,
                item.product_name.as_deref().unwrap_or("Unknown"),
                item.resource_id.as_deref().unwrap_or("N/A"),
                item.amount_real.or(item.amount).unwrap_or(0.0),
            );
        }
    } else {
        println!("No billing data found for this period.");
    }

    Ok(())
}
