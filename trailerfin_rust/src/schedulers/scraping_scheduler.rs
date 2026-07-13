use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use async_trait::async_trait;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info};

use crate::configuration::configuration_provider::AppConfig;
use crate::schedulers::traits::Scheduler;
use crate::schedulers::types::ScanFn;
use crate::scrapers::{get_scraper};
use crate::scrapers::traits::TrailerScraper;

#[derive(Debug)]
pub struct ScrapingScheduler;



#[async_trait]
impl Scheduler for ScrapingScheduler {
    async fn create_scheduler(
        &self,
        app_config: Arc<AppConfig>,
        scan_fn: Option<ScanFn>,
        is_running: Arc<AtomicBool>,
    ) -> Result<JobScheduler> {
        self.setup_scheduler_with_lock(app_config, scan_fn, is_running).await
    }
}

impl ScrapingScheduler {
    pub async fn start_scheduler(&self, app_config: Arc<AppConfig>) -> Result<()> {
        let is_running = Arc::new(AtomicBool::new(false)); // shared lock
        let sched = self.setup_scheduler_with_lock(app_config.clone(), None, Arc::clone(&is_running)).await?;

        info!(
        "Scheduler started with schedule: {}",
        app_config.schedule.as_deref().unwrap_or("No schedule")
    );

        let scraper = get_scraper();
        let config = Arc::clone(&app_config);
        let is_running_clone = Arc::clone(&is_running);

        tokio::spawn(async move {
            if is_running_clone.swap(true, Ordering::SeqCst) {
                tracing::warn!("Initial one-shot scan skipped: job already running");
                return;
            }

            if let Err(err) = scraper.scan_and_refresh_trailers(&config).await {
                error!("Initial trailer scan failed: {err}");
            }

            is_running_clone.store(false, Ordering::SeqCst);
        });

        sched.start().await?;
        tokio::signal::ctrl_c().await?;
        Ok(())
    }

    pub async fn setup_scheduler_with_lock(
        &self,
        app_config: Arc<AppConfig>,
        scan_fn: Option<ScanFn>,
        is_running: Arc<AtomicBool>,
    ) -> Result<JobScheduler> {
        let sched = JobScheduler::new().await?;
        let schedule_expr = app_config.schedule.clone().unwrap_or_else(|| "0 0 * * *".to_string());

        let scan_handler: ScanFn = scan_fn.unwrap_or_else(|| {
            Arc::new(|config: Arc<AppConfig>| {
                Box::pin(async move {
                    info!("Starting trailer scan and refresh...");
                    let scraper: Arc<dyn TrailerScraper> = get_scraper();
                    scraper.scan_and_refresh_trailers(&config)
                        .await
                        .map_err(|e| anyhow::anyhow!(e))
                })
            })
        });

        let config = Arc::clone(&app_config);
        let scan_handler_clone = Arc::clone(&scan_handler);
        let is_running_clone = Arc::clone(&is_running);

        let job = Job::new_async(&schedule_expr, move |_uuid, _l| {
            let config = Arc::clone(&config);
            let scan_handler = Arc::clone(&scan_handler_clone);
            let is_running = Arc::clone(&is_running_clone);

            Box::pin(async move {
                if is_running.swap(true, Ordering::SeqCst) {
                    tracing::warn!("Scheduled scan skipped: previous job still running");
                    return;
                }

                tracing::info!("Running scheduled trailer scan...");
                if let Err(err) = scan_handler(config).await {
                    error!("Scheduled scan failed: {err}");
                }

                is_running.store(false, Ordering::SeqCst);
            })
        })?;

        sched.add(job).await?;
        Ok(sched)
    }
}