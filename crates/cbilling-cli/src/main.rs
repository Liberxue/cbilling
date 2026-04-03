mod app;
mod event;
mod styles;
mod ui;
mod views;
mod widgets;

use std::time::Duration;

use clap::{Parser, Subcommand};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;

#[derive(Parser)]
#[command(
    name = "cbilling",
    version,
    about = "Multi-cloud billing dashboard & query tool"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Query billing data for a specific provider and month
    Query {
        /// Cloud provider: aliyun, aws, tencentcloud, volcengine, ucloud, gcp, cloudflare
        provider: String,
        /// Billing month (YYYY-MM), defaults to current month
        #[arg(short, long)]
        month: Option<String>,
        /// Export to CSV file
        #[arg(long)]
        csv: Option<String>,
    },
    /// List all configured cloud providers
    Providers,
    /// Query all providers and print summary
    Summary {
        /// Billing month (YYYY-MM), defaults to current month
        #[arg(short, long)]
        month: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Some(cmd) => run_command(cmd).await,
        None => run_tui().await,
    }
}

async fn run_command(cmd: Commands) -> Result<(), Box<dyn std::error::Error>> {
    use cbilling::service::CloudBillingService;

    match cmd {
        Commands::Providers => {
            let providers = CloudBillingService::get_configured_providers();
            if providers.is_empty() {
                println!(
                    "No providers configured. Set environment variables for your cloud accounts."
                );
            } else {
                println!("{:<16} STATUS", "PROVIDER");
                println!("{}", "-".repeat(30));
                for p in &providers {
                    println!("{:<16} ready", p);
                }
                println!("\n{} provider(s) configured", providers.len());
            }
        }
        Commands::Query {
            provider,
            month,
            csv,
        } => {
            let cycle = month.unwrap_or_else(|| chrono::Local::now().format("%Y-%m").to_string());
            println!("Querying {} for {}...\n", provider, cycle);

            let data = CloudBillingService::query_provider(&provider, &cycle).await?;

            // Print table
            println!(
                "{:<4} {:<30} {:<20} {:>12} {:>6} {:<10}",
                "#", "PRODUCT", "CODE", "COST", "QTY", "REGIONS"
            );
            println!("{}", "-".repeat(90));
            for (i, p) in data.products.iter().enumerate() {
                let qty = p.count.map(|c| format!("{}", c)).unwrap_or("-".into());
                let regions = if p.regions.is_empty() {
                    "-".into()
                } else {
                    p.regions.join(",")
                };
                println!(
                    "{:<4} {:<30} {:<20} {:>12.2} {:>6} {}",
                    i + 1,
                    truncate(&p.product_name, 30),
                    truncate(&p.product_code, 20),
                    p.cost,
                    qty,
                    regions,
                );
            }
            println!("{}", "-".repeat(90));
            println!(
                "Total: {:.2} {}  ({} products)",
                data.total_cost,
                data.currency,
                data.products.len()
            );

            // CSV export
            if let Some(path) = csv {
                let mut out = String::new();
                out.push_str("product_name,product_code,cost,qty,regions\n");
                for p in &data.products {
                    let qty = p.count.map(|c| format!("{}", c)).unwrap_or_default();
                    let regions = p.regions.join(";");
                    out.push_str(&format!(
                        "\"{}\",\"{}\",{:.2},{},{}\n",
                        p.product_name, p.product_code, p.cost, qty, regions,
                    ));
                }
                std::fs::write(&path, &out)?;
                println!("\nExported to {}", path);
            }
        }
        Commands::Summary { month } => {
            let cycle = month.unwrap_or_else(|| chrono::Local::now().format("%Y-%m").to_string());
            let providers = CloudBillingService::get_configured_providers();

            if providers.is_empty() {
                println!("No providers configured.");
                return Ok(());
            }

            println!(
                "Querying {} provider(s) for {}...\n",
                providers.len(),
                cycle
            );

            println!(
                "{:<16} {:>14} {:<5} {:>8}",
                "PROVIDER", "COST", "CUR", "PRODUCTS"
            );
            println!("{}", "-".repeat(48));

            let mut grand_total_by_currency: std::collections::BTreeMap<String, f64> =
                std::collections::BTreeMap::new();

            for provider in &providers {
                match CloudBillingService::query_provider(provider, &cycle).await {
                    Ok(data) => {
                        *grand_total_by_currency
                            .entry(data.currency.clone())
                            .or_default() += data.total_cost;
                        println!(
                            "{:<16} {:>14.2} {:<5} {:>8}",
                            provider,
                            data.total_cost,
                            data.currency,
                            data.products.len(),
                        );
                    }
                    Err(e) => {
                        println!("{:<16} ERROR: {}", provider, e);
                    }
                }
            }

            println!("{}", "-".repeat(48));
            for (currency, total) in &grand_total_by_currency {
                println!("{:<16} {:>14.2} {}", "TOTAL", total, currency);
            }
        }
    }

    Ok(())
}

async fn run_tui() -> Result<(), Box<dyn std::error::Error>> {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        let _ = execute!(std::io::stdout(), DisableMouseCapture);
        ratatui::restore();
        original_hook(panic);
    }));

    let mut terminal = ratatui::init();
    execute!(std::io::stdout(), EnableMouseCapture)?;

    let mut events = event::EventHandler::new(Duration::from_millis(200));
    let mut app = app::App::new(events.tx.clone());
    app.start_loading_all();

    loop {
        terminal.draw(|frame| ui::draw(frame, &mut app))?;
        if let Some(event) = events.next().await {
            app.handle_event(event);
        }
        if app.should_quit {
            break;
        }
    }

    execute!(std::io::stdout(), DisableMouseCapture)?;
    ratatui::restore();
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}...", s.chars().take(max - 3).collect::<String>())
    }
}
