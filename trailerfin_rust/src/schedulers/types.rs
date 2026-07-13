use std::sync::Arc;
use crate::configuration::configuration_provider::AppConfig;

pub type ScanFn = Arc<dyn Fn(Arc<AppConfig>) -> ScanFuture + Send + Sync>;

pub type ScanFuture = std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send>>;