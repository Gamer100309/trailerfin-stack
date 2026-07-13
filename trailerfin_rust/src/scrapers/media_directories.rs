use std::path::PathBuf;
use std::sync::Arc;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use tokio::sync::Semaphore;
use tracing::{error, warn};
use walkdir::WalkDir;
use crate::configuration::configuration_provider::AppConfig;
use crate::scrapers::traits::TrailerScraper;

pub const BACKDROPS_FOLDER: &str = "backdrops";

#[derive(Debug, Clone)]
pub enum FolderType {
    TvShow,
    Movie,
}

#[derive(Debug)]
pub struct TaggedDir {
    pub path: PathBuf,
    pub folder_type: FolderType,
}

pub async fn process_media_folders(
    app_config: &Arc<AppConfig>,
    scraper: Arc<dyn TrailerScraper>,
) -> anyhow::Result<()> {
    let scan_path = PathBuf::from(&app_config.scan_path).canonicalize()?;
    if !scan_path.exists() {
        error!("Provided path does not exist: {:?}", scan_path);
        return Ok(());
    }

    let tv_dirs: Vec<_> = app_config
        .tv_folders
        .iter()
        .flat_map(|f| scan_tagged_subdirs(&scan_path, f, FolderType::TvShow))
        .collect();

    let movie_dirs: Vec<_> = app_config
        .movie_folders
        .iter()
        .flat_map(|f| scan_tagged_subdirs(&scan_path, f, FolderType::Movie))
        .collect();

    let all_dirs = tv_dirs.into_iter().chain(movie_dirs).collect::<Vec<_>>();

    let total = all_dirs.len();
    if total == 0 {
        warn!("No valid media directories found.");
        return Ok(());
    }

    let semaphore = Arc::new(Semaphore::new(app_config.threads));
    let mut tasks = FuturesUnordered::new();

    for tagged_dir in all_dirs {
        let permit = semaphore.clone().acquire_owned().await?;
        let config = Arc::clone(&app_config);
        let service = Arc::clone(&scraper);
        let path = tagged_dir.path;
        let folder_type = tagged_dir.folder_type;

        tasks.push(tokio::spawn(async move {
            let _permit = permit;
            service.process_path(path, config, folder_type).await;
        }));
    }

    while let Some(res) = tasks.next().await {
        if let Err(e) = res {
            error!("A task panicked or failed: {:?}", e);
        }
    }

    Ok(())
}

fn scan_tagged_subdirs(base: &PathBuf, subfolder: &str, folder_type: FolderType) -> Vec<TaggedDir> {
    let path = base.join(subfolder);
    if !path.exists() || !path.is_dir() {
        return vec![];
    }

    WalkDir::new(&path)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
        .map(|e| TaggedDir {
            path: e.into_path(),
            folder_type: folder_type.clone(),
        })
        .collect()
}