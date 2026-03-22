// Example: Query AWS billing data via Cost Explorer
//
// Run with: cargo run --example aws_billing --features aws
//
// AWS credentials will be loaded from:
// 1. Environment variables (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY)
// 2. ~/.aws/credentials file
// 3. IAM role (if running on EC2/ECS)

use cbilling::providers::aws::AwsBillingClient;

#[tokio::main]
async fn main() -> cbilling::Result<()> {
    tracing_subscriber::fmt::init();

    println!("[AWS] Billing Example\n");

    let client = AwsBillingClient::new_with_default_credentials("us-east-1".to_string()).await?;

    let now = chrono::Local::now();
    let last_month = now - chrono::Duration::days(30);
    let start_date = last_month.format("%Y-%m-01").to_string();
    let end_date = now.format("%Y-%m-01").to_string();

    println!("Querying AWS costs from {} to {}", start_date, end_date);

    let result = client
        .get_cost_and_usage(
            &start_date,
            &end_date,
            "MONTHLY",
            vec!["UnblendedCost"],
            Some(vec![("DIMENSION", "SERVICE")]),
        )
        .await?;

    println!("Query successful!\n");
    println!("Results by time:");

    if let Some(results) = result.results_by_time {
        for time_period_result in results {
            let period = &time_period_result.time_period;
            println!("\n  Period: {} to {}", period.start, period.end);

            if let Some(total) = &time_period_result.total {
                if let Some(cost) = total.get("UnblendedCost") {
                    let amount = cost.get("Amount").and_then(|v| v.as_str()).unwrap_or("0");
                    let unit = cost.get("Unit").and_then(|v| v.as_str()).unwrap_or("USD");
                    println!("  Total cost: {} {}", amount, unit);
                }
            }

            if let Some(groups) = &time_period_result.groups {
                println!("  Services ({} total):", groups.len());
                for (i, group) in groups.iter().enumerate().take(10) {
                    let service = group.keys.first().map(|s| s.as_str()).unwrap_or("Unknown");
                    let cost = group
                        .metrics
                        .get("UnblendedCost")
                        .and_then(|v| v.get("Amount"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("0");
                    let unit = group
                        .metrics
                        .get("UnblendedCost")
                        .and_then(|v| v.get("Unit"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("USD");
                    println!("    {}. {} - {} {}", i + 1, service, cost, unit);
                }
                if groups.len() > 10 {
                    println!("    ... and {} more", groups.len() - 10);
                }
            }
        }
    }

    Ok(())
}
