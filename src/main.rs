use std::time::Duration;

use anyhow::Context;
use chrono::{Local, TimeDelta};
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, fmt};

use aioworldline::{ReportOptions, Settings, WorldlineSession};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialise structured logging; level is controlled by RUST_LOG.
    // e.g.  RUST_LOG=aioworldline=debug  or  RUST_LOG=info
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let settings = Settings::from_env().context("failed to load configuration")?;

    let today = Local::now().date_naive();
    let date_from = today - TimeDelta::days(5);
    let date_till = date_from; // same as Python: current_date_till = date_from

    info!("Logging in to Worldline portal");
    let session = WorldlineSession::login(
        &settings.login,
        &settings.password,
        Some(Duration::from_secs(15 * 60)),
    )
        .await
        .context("login failed")?;

    let opts = ReportOptions {
        account_id: &settings.account_id,
        ..Default::default()
    };

    info!("Fetching transaction report for {date_from} \u{2013} {date_till}");
    let csv_bytes = session
        .get_transaction_report(
            date_from,
            date_till,
            opts.account_id,
            opts.date_type,
            opts.use_date,
            opts.merchant,
            opts.term_id,
            opts.export_type,
        )
        .await
        .context("failed to fetch transaction report")?;

    // The portal returns UTF-8 with a BOM (Python's "utf-8-sig"); strip it.
    let csv_data = csv_bytes
        .strip_prefix(b"\xEF\xBB\xBF")
        .unwrap_or(&csv_bytes);

    let csv_str = std::str::from_utf8(csv_data).context("report is not valid UTF-8")?;

    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b';')
        .has_headers(true)
        .from_reader(csv_str.as_bytes());

    for result in reader.records() {
        match result {
            Ok(record) => info!("{record:?}"),
            Err(err) => error!("CSV parse error: {err}"),
        }
    }

    Ok(())
}
