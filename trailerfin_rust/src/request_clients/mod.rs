use std::sync::Arc;
use once_cell::sync::OnceCell;
use crate::configuration::configuration_provider::{AppConfig, DataSource};
use crate::request_clients::imdb_client::imdb_request_client;
use crate::request_clients::imdb_client::imdb_request_client::ImdbRequestClient;
use crate::request_clients::rate_limited_client::RateLimitedClient;
use crate::request_clients::tmdb_client::tmdb_request_client;
use crate::request_clients::tmdb_client::tmdb_request_client::TmdbRequestClient;

pub mod tmdb_client;
pub mod imdb_client;
pub mod rate_limited_client;
pub mod request_errors;

static IMDB_REQUEST_CLIENT: OnceCell<Arc<ImdbRequestClient>> = OnceCell::new();

static TMDB_REQUEST_CLIENT: OnceCell<Arc<TmdbRequestClient>> = OnceCell::new();

pub fn initialize_request_clients(app_config: Arc<AppConfig>) {
    initialize_imdb_request_client(app_config.clone());

    if app_config.data_source == DataSource::Tmdb {
        initialize_tmdb_request_client(app_config.clone());
    }
}

pub fn initialize_imdb_request_client(app_config: Arc<AppConfig>) {
    let executor = RateLimitedClient::from_config(
        &app_config.user_agent,
        &app_config.imdb_rate_limit,
    ).expect("Failed to create IMDB executor");

    let inner_client = imdb_request_client::ClientBuilder::default()
        .with_executor(executor)
        .build()
        .expect("Failed to build IMDB request client");

    let client = ImdbRequestClient(inner_client);
    
    IMDB_REQUEST_CLIENT
        .set(Arc::new(client))
        .expect("IMDB request client already initialized");
}

pub fn initialize_tmdb_request_client(app_config: Arc<AppConfig>) {
    let executor = RateLimitedClient::from_config(
        &app_config.user_agent,
        &app_config.tmdb_rate_limit,
    ).expect("Failed to create TMDB executor");

    let tmdb_api_key = app_config.tmdb_api_key.clone().unwrap_or_else(|| {
        panic!("TMDB API key is required but not provided in the configuration");
    });

    let inner_client = tmdb_request_client::ClientBuilder::default()
        .with_api_key(tmdb_api_key)
        .with_executor(executor)
        .build()
        .expect("Failed to build TMDB request client");

    let client = TmdbRequestClient(inner_client);

    TMDB_REQUEST_CLIENT
        .set(Arc::new(client))
        .expect("TMDB request client already initialized");
}

pub fn get_imdb_client() -> Arc<ImdbRequestClient> {
    IMDB_REQUEST_CLIENT.get().expect("IMDB Client not initialized").clone()
}

pub fn get_tmdb_client() -> Arc<TmdbRequestClient> {
    TMDB_REQUEST_CLIENT.get().expect("TMDB Client not initialized").clone()
}