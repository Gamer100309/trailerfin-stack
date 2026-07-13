use std::sync::Arc;
use once_cell::sync::OnceCell;
use tracing::{debug};
use crate::caching::get_tmdb_to_imdb_cache;
use crate::configuration::configuration_provider::{AppConfig, DataSource};
use crate::scrapers::imdb_trailers::ImdbTrailerScraper;
use crate::scrapers::tmdb_trailers::TmdbTrailerScraper;
use crate::scrapers::traits::TrailerScraper;

pub mod imdb_trailers;
pub mod tmdb_trailers;
pub mod traits;
mod media_directories;

pub static TRAILER_SCRAPER: OnceCell<Arc<dyn TrailerScraper>> = OnceCell::new();

pub fn initialize_scrapers(app_config: Arc<AppConfig>) {
    let imdb_scraper = Arc::new(ImdbTrailerScraper {});

    debug!("Using data source: {:?}", app_config.data_source);
    
    match app_config.data_source {
        DataSource::Imdb => {
            let scraper: Arc<dyn TrailerScraper> = imdb_scraper.clone();
            TRAILER_SCRAPER.set(scraper).expect("Scraper already initialized");
        }
        DataSource::Tmdb => {
            let scraper: Arc<dyn TrailerScraper> = Arc::new(
                TmdbTrailerScraper {
                    imdb_trailer_scraper: imdb_scraper.clone(),
                    tmdb_to_imdb_cache: get_tmdb_to_imdb_cache()
                }
            );
            TRAILER_SCRAPER.set(scraper).expect("Scraper already initialized");
        }
    }
    
    debug!("Initialized trailer scraper");
}


pub fn get_scraper() -> Arc<dyn TrailerScraper> {
    TRAILER_SCRAPER
        .get()
        .expect("Trailer scraper not initialized")
        .clone()
}