use serde::{Deserialize, Serialize};
use crate::request_clients::request_errors::error::Error;
use crate::request_clients::tmdb_client::tmdb_request_client::TmdbRequestClient;

#[derive(Debug, Deserialize)]
pub struct VideoResult {
    pub key: String,
    pub site: String,
    #[serde(rename = "type")]
    pub video_type: String,
    #[serde(default)]
    pub official: bool,
    #[serde(default)]
    pub size: u32,
}

#[derive(Debug, Deserialize)]
pub struct VideosResponse {
    pub id: u64,
    pub results: Vec<VideoResult>,
}

impl VideosResponse {
    /// Picks the best available YouTube trailer: prefers official trailers,
    /// then the highest resolution ("size") available.
    pub fn best_youtube_trailer_key(&self) -> Option<&str> {
        self.results
            .iter()
            .filter(|v| {
                v.site.eq_ignore_ascii_case("YouTube")
                    && v.video_type.eq_ignore_ascii_case("Trailer")
            })
            .max_by_key(|v| (v.official, v.size))
            .map(|v| v.key.as_str())
    }

    /// All YouTube trailer candidates, sorted best-first (official, then
    /// size). Used when channel whitelist/blacklist filtering needs to
    /// consider more than just the single top pick.
    pub fn youtube_trailer_candidates(&self) -> Vec<&VideoResult> {
        let mut candidates: Vec<&VideoResult> = self
            .results
            .iter()
            .filter(|v| {
                v.site.eq_ignore_ascii_case("YouTube")
                    && v.video_type.eq_ignore_ascii_case("Trailer")
            })
            .collect();
        candidates.sort_by_key(|v| std::cmp::Reverse((v.official, v.size)));
        candidates
    }
}

#[derive(Serialize, Default)]
struct VideosQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    language: Option<String>,
}

pub struct VideosService<'a> {
    pub(crate) client: &'a TmdbRequestClient,
}

impl<'a> VideosService<'a> {
    pub async fn get_for_movie(&self, movie_id: &str) -> Result<VideosResponse, Error> {
        self.get_for_movie_with_language(movie_id, None).await
    }

    pub async fn get_for_tv(&self, tv_id: &str) -> Result<VideosResponse, Error> {
        self.get_for_tv_with_language(tv_id, None).await
    }

    pub async fn get_for_movie_with_language(
        &self,
        movie_id: &str,
        language: Option<&str>,
    ) -> Result<VideosResponse, Error> {
        let url = format!("/movie/{movie_id}/videos");
        self.client
            .execute(&url, VideosQuery { language: language.map(str::to_string) })
            .await
    }

    pub async fn get_for_tv_with_language(
        &self,
        tv_id: &str,
        language: Option<&str>,
    ) -> Result<VideosResponse, Error> {
        let url = format!("/tv/{tv_id}/videos");
        self.client
            .execute(&url, VideosQuery { language: language.map(str::to_string) })
            .await
    }
}
