// Example: Query Aliyun billing data
//
// Run with: cargo run --example aliyun_billing --features aliyun
//
// Required environment variables:
// - ALIBABA_CLOUD_ACCESS_KEY_ID
// - ALIBABA_CLOUD_ACCESS_KEY_SECRET

use cbilling::providers::aliyun::AliyunBillingClient;

#[tokio::main]
async fn main() -> cbilling::Result<()> {
    tracing_subscriber::fmt::init();

    println!("[Aliyun] Billing Example\n");

    let access_key_id =
        std::env::var("ALIBABA_CLOUD_ACCESS_KEY_ID").expect("ALIBABA_CLOUD_ACCESS_KEY_ID not set");
    let access_key_secret = std::env::var("ALIBABA_CLOUD_ACCESS_KEY_SECRET")
        .expect("ALIBABA_CLOUD_ACCESS_KEY_SECRET not set");

    let client = AliyunBillingClient::new(access_key_id, access_key_secret);

    let billing_cycle = chrono::Local::now().format("%Y-%m").to_string();
    println!("Querying billing data for cycle: {}", billing_cycle);

    let response = client
        .query_instance_bill(&billing_cycle, Some(1), Some(10), None)
        .await?;

    if response.success {
        println!("Query successful!\n");

        if let Some(data) = response.data {
            println!("Total records: {}", data.total_count);
            println!("Current page: {}", data.page_num);

            if let Some(items) = data.items {
                println!("\nSample bills (first 10):");
                for item in items.item.iter().take(10) {
                    println!(
                        "  - Product: {}",
                        item.product_name.as_deref().unwrap_or("Unknown")
                    );
                    println!(
                        "    Cost: {} {}",
                        item.pretax_amount.unwrap_or(0.0),
                        item.currency.as_deref().unwrap_or("CNY")
                    );
                    println!();
                }
            }
        }
    } else {
        eprintln!("Query failed: {} - {}", response.code, response.message);
    }

    Ok(())
}
