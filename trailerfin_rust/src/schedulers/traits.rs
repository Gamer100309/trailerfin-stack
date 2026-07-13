use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use async_trait::async_trait;
use tokio_cron_scheduler::JobScheduler;
use crate::configuration::configuration_provider::AppConfig;
use crate::schedulers::types::ScanFn;

#[async_trait]
pub trait Scheduler {
    async fn create_scheduler(
        &self,
        app_config: Arc<AppConfig>,
        scan_fn: Option<ScanFn>,
        is_running: Arc<AtomicBool>,
    ) -> anyhow::Result<JobScheduler>;
}