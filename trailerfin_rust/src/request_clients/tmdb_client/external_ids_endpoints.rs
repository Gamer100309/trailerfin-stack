use serde::Deserialize;
use crate::request_clients::request_errors::error::Error;
use crate::request_clients::tmdb_client::tmdb_request_client::{TmdbRequestClient};
use crate::utils::empty_strings;

#[derive(Debug, Deserialize)]
pub struct MovieExternalIds {
    pub id: u64,
    #[serde(deserialize_with = "empty_strings::deserialize")]
    pub imdb_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TvShowExternalIds {
    pub id: u64,
    #[serde(deserialize_with = "empty_strings::deserialize")]
    pub imdb_id: Option<String>,
}

pub(crate) enum ExternalIds {
    Movie(MovieExternalIds),
    Tv(TvShowExternalIds),
}

impl ExternalIds {
    pub fn imdb_id(&self) -> Option<&str> {
        match self {
            ExternalIds::Movie(m) => m.imdb_id.as_deref(),
            ExternalIds::Tv(t) => t.imdb_id.as_deref(),
        }
    }
}


pub struct ExternalIdsService<'a> {
    pub(crate) client: &'a TmdbRequestClient,
}

impl<'a> ExternalIdsService<'a> {
    pub async fn get_for_movie(&self, movie_id: &str) -> Result<MovieExternalIds, Error> {
        let url = format!("/movie/{movie_id}/external_ids");
        self.client.execute(&url, &()).await
    }

    pub async fn get_for_tv(&self, tv_id: &str) -> Result<TvShowExternalIds, Error> {
        let url = format!("/tv/{tv_id}/external_ids");
        let result = self.client.execute(&url, &()).await;

        match &result {
            Ok(data) => {
                tracing::debug!("Successfully fetched external IDs for TV ID {}: {:?}", tv_id, data);
            }
            Err(err) => {
                tracing::warn!("Failed to fetch external IDs for TV ID {}: {}", tv_id, err);
            }
        }

        result
    }
}