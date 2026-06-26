// Example: Query Vast.ai billing data
//
// Run with: cargo run --example vastai_billing --features vastai
//
// Required environment variables:
// - VASTAI_API_KEY (Bearer API key from https://cloud.vast.ai/manage-keys/)
//   (VAST_API_KEY is also accepted)

use chrono::Datelike;

use cbilling::providers::vastai::VastaiBillingClient;

#[tokio::main]
async fn main() -> cbilling::Result<()> {
    tracing_subscriber::fmt::init();

    println!("[Vast.ai] Billing Example\n");

    let api_key = std::env::var("VASTAI_API_KEY")
        .or_else(|_| std::env::var("VAST_API_KEY"))
        .expect("VASTAI_API_KEY not set");

    let client = VastaiBillingClient::new(api_key);

    // Current month range (UTC) in unix seconds.
    let now = chrono::Utc::now();
    let start = now
        .date_naive()
        .with_day(1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp();
    let end = now.timestamp();

    println!("Querying charges for the current month...");
    let charges = client.get_all_charges(start, end).await?;
    println!("Charge entries ({}):", charges.len());

    let mut total = 0.0;
    for charge in charges.iter().take(10) {
        let amount = charge.amount.unwrap_or(0.0);
        total += amount;
        println!(
            "  - {} | {:.4} USD | {}",
            charge.source.as_deref().unwrap_or("N/A"),
            amount,
            charge.description.as_deref().unwrap_or("N/A"),
        );
    }

    println!("\nApprox. total (first 10 entries shown): {:.2} USD", total);

    Ok(())
}
