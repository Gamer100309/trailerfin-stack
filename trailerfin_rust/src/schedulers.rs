use std::sync::Arc;
use once_cell::sync::OnceCell;
use tracing::{debug};
use crate::schedulers::scraping_scheduler::ScrapingScheduler;

pub mod scraping_scheduler;
pub mod traits;
pub mod types;

pub static SCRAPING_SCHEDULER: OnceCell<Arc<ScrapingScheduler>> = OnceCell::new();

pub fn initialize_schedulers() {
    SCRAPING_SCHEDULER.get_or_init(|| Arc::new(ScrapingScheduler {}));
    debug!("Initialized schedulers");
}

pub fn get_scraping_scheduler() -> Arc<ScrapingScheduler> {
    SCRAPING_SCHEDULER
        .get()
        .expect("Scraping scheduler not initialized")
        .clone()
}