use crate::scrapers::traits::TrailerScraper;
use std::path::{Path, PathBuf};

use anyhow::Result;
use regex::Regex;
use std::sync::Arc;
use async_trait::async_trait;
use once_cell::sync::{Lazy};
use tracing::{info, warn};
use crate::caching::tmdb_to_imdb_cache::TmdbToImdbCache;
use crate::configuration::configuration_provider::{AppConfig, TrailerMode};
use crate::request_clients::get_tmdb_client;
use crate::request_clients::tmdb_client::external_ids_endpoints;
use crate::scrapers::media_directories::{process_media_folders, FolderType, BACKDROPS_FOLDER};
use crate::scrapers::imdb_trailers::ImdbTrailerScraper;

#[derive(Debug)]
pub struct TmdbTrailerScraper {
    pub imdb_trailer_scraper: Arc<ImdbTrailerScraper>,
    pub tmdb_to_imdb_cache: Arc<TmdbToImdbCache>,
}

// Accepts any bracket style ([ ] / { } / ( )) and both "tmdb-" and
// "tmdbid-" as the tag prefix, regardless of where in the folder name
// it appears relative to other tags (e.g. [imdbid-...]).
static TMDB_ID_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[\[\{\(]\s*tmdb(?:id)?-(\d+)\s*[\]\}\)]")
        .expect("Failed to compile TMDB ID Regex")
});

/// Looks up the publishing channel's display name for a YouTube video via
/// YouTube's public oEmbed endpoint (no API key required).
async fn get_youtube_channel_name(video_key: &str) -> Option<String> {
    #[derive(serde::Deserialize)]
    struct OEmbedResponse {
        author_name: Option<String>,
    }

    let url = format!(
        "https://www.youtube.com/oembed?url=https://www.youtube.com/watch?v={}&format=json",
        video_key
    );

    match reqwest::get(&url).await {
        Ok(resp) => match resp.json::<OEmbedResponse>().await {
            Ok(parsed) => parsed.author_name,
            Err(_) => None,
        },
        Err(_) => None,
    }
}

/// Picks the best trailer key from `videos`, honoring an optional channel
/// whitelist (priority order, first match wins) and blacklist (excluded
/// entirely). Falls back to the plain "official + highest size" pick when
/// no whitelist/blacklist is configured, avoiding unnecessary oEmbed calls.
async fn select_trailer_key(
    videos: &crate::request_clients::tmdb_client::videos_endpoints::VideosResponse,
    channel_whitelist: &[String],
    channel_blacklist: &[String],
) -> Option<String> {
    if channel_whitelist.is_empty() && channel_blacklist.is_empty() {
        return videos.best_youtube_trailer_key().map(|s| s.to_string());
    }

    let candidates = videos.youtube_trailer_candidates();
    if candidates.is_empty() {
        return None;
    }

    let mut resolved: Vec<(String, Option<String>)> = Vec::with_capacity(candidates.len());
    for candidate in &candidates {
        let channel = get_youtube_channel_name(&candidate.key).await;
        info!(
            "Trailer candidate {}: channel = {}",
            candidate.key,
            channel.as_deref().unwrap_or("<unknown>")
        );
        resolved.push((candidate.key.clone(), channel));
    }

    let allowed: Vec<&(String, Option<String>)> = resolved
        .iter()
        .filter(|(_, channel)| {
            let Some(channel) = channel else { return true };
            !channel_blacklist
                .iter()
                .any(|blocked| channel.to_lowercase().contains(&blocked.to_lowercase()))
        })
        .collect();

    if allowed.is_empty() {
        warn!("All trailer candidates were excluded by the channel blacklist");
        return None;
    }

    for wanted in channel_whitelist {
        if let Some((key, channel)) = allowed.iter().find(|(_, channel)| {
            channel
                .as_deref()
                .map(|c| c.to_lowercase().contains(&wanted.to_lowercase()))
                .unwrap_or(false)
        }) {
            info!(
                "Selected trailer {} from whitelisted channel \"{}\" (matched \"{}\")",
                key,
                channel.as_deref().unwrap_or("<unknown>"),
                wanted
            );
            return Some(key.clone());
        }
    }

    if let Some((key, channel)) = allowed.first() {
        info!(
            "No whitelist match, falling back to best available trailer {} (channel: {})",
            key,
            channel.as_deref().unwrap_or("<unknown>")
        );
    }

    allowed.first().map(|(key, _)| key.clone())
}

/// Parses a Netscape-format cookies.txt file (the format exported by
/// browser extensions like "Get cookies.txt LOCALLY") and returns the
/// relevant cookies as a single "name=value; name2=value2" header string,
/// filtered to domains containing any of `domain_filters`.
fn parse_netscape_cookies(path: &Path, domain_filters: &[&str]) -> Result<String> {
    let content = std::fs::read_to_string(path)?;
    let mut pairs = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        // Skip blank lines and plain comments, but keep Netscape's special
        // "#HttpOnly_" prefixed lines (still valid cookie entries).
        if line.is_empty() || (line.starts_with('#') && !line.starts_with("#HttpOnly_")) {
            continue;
        }

        let line = line.strip_prefix("#HttpOnly_").unwrap_or(line);
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 7 {
            continue;
        }

        let domain = fields[0];
        let name = fields[5];
        let value = fields[6];

        if domain_filters.iter().any(|f| domain.contains(f)) {
            pairs.push(format!("{}={}", name, value));
        }
    }

    Ok(pairs.join("; "))
}

/// Downloads the video at `url` (following redirects, e.g. from the
/// ytdlp2STRM proxy to the actual CDN URL) and saves it to `dest`.
/// Google's video CDN rejects requests without a browser-like User-Agent
/// (and sometimes without a Referer), so we send both explicitly instead
/// of relying on reqwest's default (non-browser) identification. Some
/// signed CDN URLs (generated via an authenticated/cookie-based request)
/// additionally require the same session cookies to be present on the
/// download request itself, so we forward those too when configured.
async fn download_video(
    url: &str,
    dest: &Path,
    user_agent: &str,
    cookies_file: Option<&str>,
) -> Result<()> {
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let client = reqwest::Client::builder()
        .user_agent(user_agent)
        .build()?;

    let mut request = client
        .get(url)
        .header("Referer", "https://www.youtube.com/")
        .header("Origin", "https://www.youtube.com")
        .header("Accept", "*/*")
        .header("Accept-Language", "en-US,en;q=0.9")
        .header("Sec-Fetch-Mode", "cors")
        .header("Sec-Fetch-Site", "cross-site")
        .header("Sec-Fetch-Dest", "empty");

    if let Some(cookies_path) = cookies_file {
        match parse_netscape_cookies(Path::new(cookies_path), &["youtube.com", "google.com"]) {
            Ok(cookie_header) if !cookie_header.is_empty() => {
                request = request.header("Cookie", cookie_header);
            }
            Ok(_) => {}
            Err(e) => {
                warn!("Could not read/parse cookies file {}: {:?}", cookies_path, e);
            }
        }
    }

    let response = request.send().await?.error_for_status()?;

    let bytes = response.bytes().await?;
    tokio::fs::write(dest, &bytes).await?;

    Ok(())
}

#[async_trait]
impl TrailerScraper for TmdbTrailerScraper {
    async fn scan_and_refresh_trailers(self: Arc<Self>, config: &Arc<AppConfig>) -> Result<()> {
        self.perform_scan_and_refresh_tmdb(config).await
    }

    async fn process_path(&self, path: PathBuf, config: Arc<AppConfig>, folder_type: FolderType) {
        self.process_path_internal(path, config, folder_type).await;
    }
}

impl TmdbTrailerScraper {
    pub async fn perform_scan_and_refresh_tmdb(self: Arc<Self>, app_config: &Arc<AppConfig>) -> Result<()> {
        process_media_folders(app_config, self as Arc<dyn TrailerScraper>).await
    }

    async fn process_path_internal(
        &self,
        path: PathBuf,
        config: Arc<AppConfig>,
        folder_type: FolderType,
    ) {
        if let Some(path_str) = path.to_str() {
            if let Some(cap) = (&*TMDB_ID_REGEX).captures(path_str) {
                let tmdb_id = cap[1].to_string();
                let backdrops_path = path.join(BACKDROPS_FOLDER);
                let strm_path = backdrops_path.join(&config.video_filename);

                // If a ytdlp2STRM-compatible proxy is configured, use TMDB's official
                // /videos endpoint to resolve the YouTube trailer key directly. This
                // avoids scraping IMDb's website entirely, which is now blocked by
                // IMDb's bot protection (AWS WAF challenge) for automated clients.
                if let Some(proxy_base) = &config.ytdlp_proxy_url {
                    // In download mode we save an actual .mp4 file instead of a .strm
                    // link, so the "already done" check targets that file instead.
                    let download_path = backdrops_path.join("trailer.mp4");
                    let target_exists = match config.mode {
                        TrailerMode::Link => strm_path.exists(),
                        TrailerMode::Download => download_path.exists(),
                    };

                    if target_exists {
                        info!("Trailer already present for {} in {:?}", tmdb_id, path);
                        return;
                    }

                    info!("Refreshing trailer for {} in {:?}", tmdb_id, path);

                    match self.get_youtube_trailer_key(
                        &tmdb_id,
                        folder_type,
                        config.preferred_language.as_deref(),
                        &config.channel_whitelist,
                        &config.channel_blacklist,
                    ).await {
                        Ok(Some(key)) => {
                            let stream_url = format!(
                                "{}/youtube/direct/{}",
                                proxy_base.trim_end_matches('/'),
                                key
                            );

                            match config.mode {
                                TrailerMode::Link => {
                                    if let Err(e) = self.imdb_trailer_scraper.create_or_update_strm_file(
                                        &path,
                                        &config,
                                        &stream_url,
                                    ) {
                                        warn!("Failed to write .strm file for {}: {:?}", tmdb_id, e);
                                    }
                                }
                                TrailerMode::Download => {
                                    if let Err(e) = download_video(
                                        &stream_url,
                                        &download_path,
                                        &config.user_agent,
                                        config.download_cookies_file.as_deref(),
                                    ).await {
                                        warn!("Failed to download trailer for {}: {:?}", tmdb_id, e);
                                    } else {
                                        info!("Downloaded trailer for {} to {:?}", tmdb_id, download_path);
                                    }
                                }
                            }
                        }
                        Ok(None) => {
                            warn!("No official YouTube trailer found via TMDB for {}", tmdb_id);
                        }
                        Err(e) => {
                            warn!("Failed to fetch TMDB videos for {}: {:?}", tmdb_id, e);
                        }
                    }
                    return;
                }

                // Legacy fallback: resolve via IMDb ID + scrape IMDb's website directly.
                // Kept for backwards compatibility, but currently non-functional for most
                // users since IMDb blocks automated requests with a bot challenge.
                if let Ok(expired) = self.imdb_trailer_scraper.is_strm_expired(&strm_path) {
                    if !expired {
                        info!("Trailer still valid for {} in {:?}", tmdb_id, path);
                        return;
                    }
                }

                info!("Refreshing trailer for {} in {:?}", tmdb_id, path);

                let imdb_id = match self.get_imdb_id(&tmdb_id, folder_type).await {
                    Ok(id) => id,
                    Err(e) => {
                        warn!("Failed to retrieve IMDb ID for {}: {:?}", tmdb_id, e);
                        return; // just early-exit the method
                    }
                };

                self.imdb_trailer_scraper
                    .refresh_imdb_trailer(&imdb_id, path.clone(), config)
                    .await;
            } else {
                warn!("No TMDB ID found in path: {:?}", path);
            }
        }
    }

    async fn get_youtube_trailer_key(
        &self,
        tmdb_id: &str,
        folder_type: FolderType,
        preferred_language: Option<&str>,
        channel_whitelist: &[String],
        channel_blacklist: &[String],
    ) -> Result<Option<String>> {
        let client = get_tmdb_client();

        // 1. Try the preferred language first, if one is configured.
        if let Some(lang) = preferred_language {
            let videos = match folder_type {
                FolderType::Movie => {
                    client.videos().get_for_movie_with_language(tmdb_id, Some(lang)).await?
                }
                FolderType::TvShow => {
                    client.videos().get_for_tv_with_language(tmdb_id, Some(lang)).await?
                }
            };

            if let Some(key) = select_trailer_key(&videos, channel_whitelist, channel_blacklist).await {
                return Ok(Some(key));
            }

            info!(
                "No {} trailer found for TMDB ID {}, falling back to default language",
                lang, tmdb_id
            );
        }

        // 2. Fall back to TMDB's default (usually English/original).
        let videos = match folder_type {
            FolderType::Movie => client.videos().get_for_movie(tmdb_id).await?,
            FolderType::TvShow => client.videos().get_for_tv(tmdb_id).await?,
        };

        Ok(select_trailer_key(&videos, channel_whitelist, channel_blacklist).await)
    }

    async fn get_imdb_id(&self, tmdb_id: &str, folder_type: FolderType) -> Result<String> {
        if let Some(imdb_id) = self.tmdb_to_imdb_cache.try_get_imdb_id(tmdb_id)? {
            return Ok(imdb_id);
        }

        info!("No IMDB ID found in local cache for TMDB ID: {}", tmdb_id);
        let client = get_tmdb_client();

        let external_ids = match folder_type {
            FolderType::TvShow => {
                info!("Fetching IMDB ID for TV Show TMDB ID: {}", tmdb_id);
                external_ids_endpoints::ExternalIds::Tv(client.external_ids().get_for_tv(tmdb_id).await?)
            }
            FolderType::Movie => {
                info!("Fetching IMDB ID for Movie TMDB ID: {}", tmdb_id);
                external_ids_endpoints::ExternalIds::Movie(client.external_ids().get_for_movie(tmdb_id).await?)
            }
        };

        if let Some(imdb_id) = external_ids.imdb_id() {
            self.tmdb_to_imdb_cache.add(tmdb_id, imdb_id)?;
            Ok(imdb_id.to_string())
        } else {
            warn!("No IMDB ID found for TMDB ID: {}", tmdb_id);
            Err(anyhow::anyhow!("No IMDB ID found for TMDB ID: {}", tmdb_id))
        }
    }
}