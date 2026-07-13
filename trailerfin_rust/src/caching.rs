use std::sync::Arc;
use once_cell::sync::OnceCell;
use tracing::{debug};
use crate::caching::redb_database::RedbDatabase;
use crate::caching::tmdb_to_imdb_cache::TmdbToImdbCache;
use crate::configuration::configuration_provider::AppConfig;

pub mod tmdb_to_imdb_cache;
mod redb_database;

pub static REDB_INSTANCE: OnceCell<Arc<RedbDatabase>> = OnceCell::new();
pub static TMDB_TO_IMDB_CACHE: OnceCell<Arc<TmdbToImdbCache>> = OnceCell::new();

pub fn initialize_caching(app_config: Arc<AppConfig>) {
    let db = init_database(app_config);
    init_tmdb_to_imdb_cache(db);
    debug!("Initialized caching");
}

pub fn get_tmdb_to_imdb_cache() -> Arc<TmdbToImdbCache> {
    TMDB_TO_IMDB_CACHE
        .get()
        .expect("Tmdb to imdb cache not initialized")
        .clone()
}

fn init_database(app_config: Arc<AppConfig>) -> Arc<RedbDatabase> {
    let redb_path = std::path::Path::new(&app_config.cache_path).canonicalize().unwrap().join("caches.redb");
    let db = RedbDatabase::new(&redb_path).expect("Failed to initialize redb database");
    REDB_INSTANCE.get_or_init(|| Arc::new(db)).clone()
}

fn init_tmdb_to_imdb_cache(db: Arc<RedbDatabase>) {
    let cache = TmdbToImdbCache::new(db).expect("Failed to initialize TMDB to IMDB cache");
    TMDB_TO_IMDB_CACHE.get_or_init(|| Arc::new(cache));
}