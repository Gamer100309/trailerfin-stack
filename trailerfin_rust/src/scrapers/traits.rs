use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::Arc;
use async_trait::async_trait;
use crate::configuration::configuration_provider::AppConfig;
use crate::scrapers::media_directories::FolderType;

#[async_trait]
pub trait TrailerScraper: Send + Sync + Debug {
    async fn scan_and_refresh_trailers(self: Arc<Self>, config: &Arc<AppConfig>) -> anyhow::Result<()>;
    async fn process_path(&self, path: PathBuf, config: Arc<AppConfig>, folder_type: FolderType);
}