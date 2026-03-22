// Example: Query Volcengine billing data
//
// Run with: cargo run --example volcengine_billing --features volcengine
//
// Required environment variables:
// - VOLCENGINE_ACCESS_KEY_ID
// - VOLCENGINE_SECRET_ACCESS_KEY

use cbilling::providers::volcengine::VolcengineBillingClient;

#[tokio::main]
async fn main() -> cbilling::Result<()> {
    tracing_subscriber::fmt::init();

    println!("[Volcengine] Billing Example\n");

    let access_key_id =
        std::env::var("VOLCENGINE_ACCESS_KEY_ID").expect("VOLCENGINE_ACCESS_KEY_ID not set");
    let secret_access_key = std::env::var("VOLCENGINE_SECRET_ACCESS_KEY")
        .expect("VOLCENGINE_SECRET_ACCESS_KEY not set");

    let client =
        VolcengineBillingClient::new(access_key_id, secret_access_key, "cn-beijing".to_string());

    let bill_period = chrono::Local::now().format("%Y-%m").to_string();
    println!("Querying bill details for: {}", bill_period);

    let response = client
        .list_bill_detail(&bill_period, Some(10), Some(0))
        .await?;

    println!(
        "Query successful! (RequestId: {})\n",
        response.response_metadata.request_id
    );

    if let Some(result) = &response.result {
        println!("Total records: {:?}", result.total.unwrap_or(0));

        if let Some(items) = &result.list {
            println!("\nBill items (showing up to 10):");
            for (i, item) in items.iter().enumerate() {
                println!(
                    "  {}. {} ({}) | Instance: {} | Payable: {} {}",
                    i + 1,
                    item.product_zh.as_deref().unwrap_or("Unknown"),
                    item.product.as_deref().unwrap_or(""),
                    item.instance_name.as_deref().unwrap_or("N/A"),
                    item.payable_amount.as_deref().unwrap_or("0"),
                    item.currency.as_deref().unwrap_or("CNY"),
                );
            }
        }
    } else {
        println!("No billing data found for this period.");
    }

    Ok(())
}
