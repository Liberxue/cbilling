// Example: Query GCP billing data
//
// Run with: cargo run --example gcp_billing --features gcp
//
// Required environment variables:
// - GCP_PROJECT_ID
// - GCP_SERVICE_ACCOUNT_JSON (JSON content) or GOOGLE_APPLICATION_CREDENTIALS (file path)

use cbilling::providers::gcp::GcpBillingClient;

#[tokio::main]
async fn main() -> cbilling::Result<()> {
    tracing_subscriber::fmt::init();

    println!("[GCP] Billing Example\n");

    let project_id = std::env::var("GCP_PROJECT_ID").expect("GCP_PROJECT_ID not set");

    let client = if let Ok(sa_json) = std::env::var("GCP_SERVICE_ACCOUNT_JSON") {
        GcpBillingClient::new(project_id, sa_json).await?
    } else {
        GcpBillingClient::new_from_credentials_file(project_id).await?
    };

    // List billing accounts
    println!("Querying billing accounts...");
    let accounts = client.list_billing_accounts().await?;
    println!("Billing accounts ({}):", accounts.billing_accounts.len());
    for acct in &accounts.billing_accounts {
        println!(
            "  - {} ({})",
            acct.display_name.as_deref().unwrap_or("N/A"),
            acct.name
        );
    }

    // Get project billing info
    println!("\nProject billing info:");
    let info = client.get_project_billing_info().await?;
    println!(
        "  Billing account: {}",
        info.billing_account_name.as_deref().unwrap_or("N/A")
    );
    println!(
        "  Billing enabled: {}",
        info.billing_enabled.unwrap_or(false)
    );

    // List services
    println!("\nListing GCP services...");
    let services = client.list_services().await?;
    println!("Services ({} total, showing first 10):", services.len());
    for svc in services.iter().take(10) {
        println!(
            "  - {} ({})",
            svc.display_name.as_deref().unwrap_or("N/A"),
            svc.service_id.as_deref().unwrap_or("N/A"),
        );
    }
    if services.len() > 10 {
        println!("  ... and {} more", services.len() - 10);
    }

    Ok(())
}
