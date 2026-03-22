// Example: Query Cloudflare billing data
//
// Run with: cargo run --example cloudflare_billing --features cloudflare
//
// Required environment variables:
// - CLOUDFLARE_ACCOUNT_ID
// - CLOUDFLARE_API_TOKEN (recommended)
// Or:
// - CLOUDFLARE_API_KEY + CLOUDFLARE_API_EMAIL

use cbilling::providers::cloudflare::CloudflareBillingClient;

#[tokio::main]
async fn main() -> cbilling::Result<()> {
    tracing_subscriber::fmt::init();

    println!("[Cloudflare] Billing Example\n");

    let account_id = std::env::var("CLOUDFLARE_ACCOUNT_ID").expect("CLOUDFLARE_ACCOUNT_ID not set");

    let client = if let Ok(token) = std::env::var("CLOUDFLARE_API_TOKEN") {
        CloudflareBillingClient::new_with_token(account_id, token)
    } else {
        let api_key = std::env::var("CLOUDFLARE_API_KEY")
            .expect("CLOUDFLARE_API_TOKEN or CLOUDFLARE_API_KEY not set");
        let api_email =
            std::env::var("CLOUDFLARE_API_EMAIL").expect("CLOUDFLARE_API_EMAIL not set");
        CloudflareBillingClient::new_with_key(account_id, api_key, api_email)
    };

    // Billing profile
    println!("Querying billing profile...");
    let profile = client.get_billing_profile().await?;
    println!(
        "  Currency: {}",
        profile.currency.as_deref().unwrap_or("N/A")
    );

    // Subscriptions
    println!("\nQuerying subscriptions...");
    let subs = client.get_subscriptions().await?;
    println!("Subscriptions ({}):", subs.len());
    for sub in subs.iter().take(10) {
        let name = sub
            .rate_plan
            .as_ref()
            .and_then(|rp| rp.public_name.as_deref())
            .unwrap_or("N/A");
        let price = sub.price.unwrap_or(0.0);
        let currency = sub.currency.as_deref().unwrap_or("USD");
        println!("  - {} - {:.2} {}", name, price, currency);
    }

    // Billing history
    println!("\nQuerying billing history...");
    let history = client.get_all_billing_history().await?;
    println!("History records ({}, showing first 5):", history.len());
    for record in history.iter().take(5) {
        println!(
            "  - {} | {:.2} {} | {}",
            record.occurred_at.as_deref().unwrap_or("N/A"),
            record.amount.unwrap_or(0.0),
            record.currency.as_deref().unwrap_or("USD"),
            record.description.as_deref().unwrap_or("N/A"),
        );
    }

    Ok(())
}
