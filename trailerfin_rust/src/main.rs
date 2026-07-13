use std::sync::Arc;
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;
use trailerfin_rust::caching::initialize_caching;
use trailerfin_rust::configuration::configuration_provider::{AppConfig, ConfigurationProvider};
use trailerfin_rust::request_clients::initialize_request_clients;
use trailerfin_rust::schedulers::{get_scraping_scheduler, initialize_schedulers};
use trailerfin_rust::scrapers::{get_scraper, initialize_scrapers};

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

#[tokio::main]
async fn main() {
    init_tracing();

    let app_config = ConfigurationProvider::load_config().expect("Failed to load configuration");

    initialize_services(&app_config);

    if app_config.should_schedule {
        info!("Starting in scheduled mode...");
        get_scraping_scheduler()
            .start_scheduler(app_config)
            .await
            .expect("Failed to start scheduler");
    } else {
        info!("Scheduling disabled: Running Once...");
        get_scraper()
            .scan_and_refresh_trailers(&app_config).await
            .expect("Failed to scan and refresh trailers");
    }
}

fn initialize_services(app_config: &Arc<AppConfig>) {
    initialize_caching(app_config.clone());
    initialize_request_clients(app_config.clone());
    initialize_scrapers(app_config.clone());
    initialize_schedulers();
    debug!("Services initialized successfully");
}
