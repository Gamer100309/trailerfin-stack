use crate::scrapers::traits::TrailerScraper;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::Result;
use regex::Regex;
use scraper::{Html, Selector};
use tracing::{error, info, warn};
use url::Url;
use std::fs::File;
use std::io::{Read, Write};
use std::sync::Arc;
use async_trait::async_trait;
use once_cell::sync::{Lazy};
use crate::configuration::configuration_provider::AppConfig;
use crate::request_clients::get_imdb_client;
use crate::request_clients::imdb_client::imdb_request_client::ImdbRequestClient;
use crate::request_clients::request_errors::error::Error;
use crate::scrapers::media_directories::{process_media_folders, FolderType, BACKDROPS_FOLDER};

const VIDEO_PROPS_PATH: &str = "/props/pageProps/videoPlaybackData/video/playbackURLs";
const SCRIPT_SELECTOR: &str = "script#\\__NEXT_DATA__";
const VIDEO_SELECTOR: &str = "a[href*=\"/video/vi\"]";
const VIDEO_MIME_TYPE: &str = "videoMimeType";
const HREF_ATTR: &str = "href";
const VIDEO_DEFINITION_ATTR: &str = "videoDefinition";
const TRAILER: &str = "trailer";
const URL: &str = "url";
const TYPE_QUERY: &str = "#t=8";

static PARSED_VIDEO_SELECTOR: Lazy<Selector> = Lazy::new(|| {
    Selector::parse(VIDEO_SELECTOR).expect("Invalid VIDEO_SELECTOR")
});

#[derive(Debug)]
pub struct ImdbTrailerScraper;


// Accepts any bracket style ([ ] / { } / ( )) and both "imdb-" and
// "imdbid-" as the tag prefix, for consistency with the TMDB matcher.
static IMDB_ID_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[\[\{\(]\s*imdb(?:id)?-(tt\d+)\s*[\]\}\)]")
        .expect("Failed to compile IMDB ID Regex")
});

#[async_trait]
impl TrailerScraper for ImdbTrailerScraper {
    async fn scan_and_refresh_trailers(self: Arc<Self>, config: &Arc<AppConfig>) -> Result<()> {
        self.perform_scan_and_refresh_imdb(config).await
    }

    async fn process_path(&self, path: PathBuf, config: Arc<AppConfig>, _: FolderType) {
        self.process_path_internal(path, config).await;
    }
}

impl ImdbTrailerScraper {
    pub async fn perform_scan_and_refresh_imdb(self: Arc<Self>, app_config: &Arc<AppConfig>) -> Result<()> {
        process_media_folders(app_config, self as Arc<dyn TrailerScraper>).await
    }

    pub fn is_strm_expired(&self, strm_path: &Path) -> Result<bool> {
        if !strm_path.exists() {
            return Ok(true);
        }

        let mut file = File::open(strm_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let url = Url::parse(contents.trim())?;
        if let Some(expires) = url.query_pairs().find_map(|(k, v)| {
            if k == "Expires" {
                v.parse::<i64>().ok()
            } else {
                None
            }
        }) {
            let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
            Ok(now >= expires)
        } else {
            Ok(true)
        }
    }

    pub fn create_or_update_strm_file(&self, folder: &Path, app_config: &AppConfig, video_url: &str) -> Result<()> {
        let backdrops = folder.join(BACKDROPS_FOLDER);
        fs::create_dir_all(&backdrops)?;
        let strm_path = backdrops.join(&app_config.video_filename);
        let mut f = File::create(&strm_path)?;
        f.write_all(video_url.as_bytes())?;
        info!("Updated {:?}", strm_path);
        Ok(())
    }

    pub async fn get_trailer_video_page_url(
        &self,
        client: &ImdbRequestClient,
        imdb_id: &str,
    ) -> Result<Option<String>> {
        let path = format!("/title/{}/videogallery/?sort=date,asc", imdb_id);
        let res = match client.get_raw(&path).await {
            Ok(r) => r,
            Err(e) => {
                error!("Request failed for {}: {e}", &path);
                return Err(e.into());
            }
        };

        if !res.status().is_success() {
            error!("Failed to fetch trailers for {} (status {})", imdb_id, res.status());
            return Ok(None);
        }

        let body = res.text().await?;
        let doc = Html::parse_document(&body);

        for el in doc.select(&PARSED_VIDEO_SELECTOR) {
            let text = el.text().collect::<String>().to_lowercase();
            if text.contains(TRAILER) {
                if let Some(href) = el.value().attr(HREF_ATTR) {
                    return Ok(Some(format!("{}", href)));
                }
            }
        }

        if let Some(el) = doc.select(&PARSED_VIDEO_SELECTOR).next() {
            if let Some(href) = el.value().attr(HREF_ATTR) {
                return Ok(Some(format!("{}", href)));
            }
        }

        warn!("No video found for {}", imdb_id);
        Ok(None)
    }

    async fn process_path_internal(
        &self,
        path: PathBuf,
        config: Arc<AppConfig>,
    ) {
        if let Some(path_str) = path.to_str() {
            if let Some(cap) = IMDB_ID_REGEX.captures(path_str) {
                let imdb_id = &cap[1];
                let backdrops_path = path.join(BACKDROPS_FOLDER);
                let strm_path = backdrops_path.join(&config.video_filename);

                if let Ok(expired) = self.is_strm_expired(&strm_path) {
                    if !expired {
                        info!("Trailer still valid for {imdb_id} in {:?}", path);
                        return;
                    }
                }

                info!("Refreshing trailer for {imdb_id} in {:?}", path);
                self.refresh_imdb_trailer(imdb_id, path.clone(), config).await;
                
            } else {
                warn!("No IMDB ID found in path: {:?}", path);
            }
        }
    }
    
    pub(crate) async fn refresh_imdb_trailer(&self, imdb_id: &str, path: PathBuf, config: Arc<AppConfig>) {
        let client = get_imdb_client().clone();

        if let Ok(Some(video_page_url)) = self.get_trailer_video_page_url(&client, imdb_id).await {
            if let Ok(Some(direct_url)) = self.get_direct_video_url_from_page(&client, &video_page_url).await {
                if let Err(e) = self.create_or_update_strm_file(&path, &config, &direct_url) {
                    error!("Failed to write .strm file: {:?}", e);
                }
            }
        }
    }

    pub async fn get_direct_video_url_from_page(
        &self,
        client: &ImdbRequestClient,
        video_page_path: &str,
    ) -> Result<Option<String>, Error> {
        let res = match client.get_raw(video_page_path).await {
            Ok(r) => r,
            Err(e) => {
                error!("Request failed for {}: {e}", video_page_path);
                return Err(e);
            }
        };

        if !res.status().is_success() {
            error!("Failed to fetch video page: {} (status {})", video_page_path, res.status());
            return Ok(None);
        }

        let body = res.text().await?;
        let doc = Html::parse_document(&body);

        let script_selector = Selector::parse(SCRIPT_SELECTOR).unwrap();
        let script_tag = doc.select(&script_selector).next();

        if let Some(tag) = script_tag {
            if let Some(json_text) = tag.inner_html().lines().next() {
                let data: serde_json::Value = serde_json::from_str(json_text.trim())?;

                if let Some(playbacks) = data.pointer(VIDEO_PROPS_PATH) {
                    let empty = vec![];
                    let mp4_urls: Vec<_> = playbacks
                        .as_array()
                        .unwrap_or(&empty)
                        .iter()
                        .filter(|entry| {
                            entry
                                .get(VIDEO_MIME_TYPE)
                                .and_then(|v| v.as_str())
                                == Some("MP4")
                        })
                        .collect();

                    if let Some(best_url) = mp4_urls
                        .iter()
                        .max_by_key(|e| {
                            e.get(VIDEO_DEFINITION_ATTR)
                                .and_then(|v| v.as_str())
                                .map(|s| match s {
                                    d if d.contains("1080") => 3,
                                    d if d.contains("720") => 2,
                                    d if d.contains("480") => 1,
                                    _ => 0,
                                })
                                .unwrap_or(0)
                        })
                        .and_then(|e| e.get(URL).and_then(|u| u.as_str()))
                    {
                        return Ok(Some(format!("{}{}", best_url, TYPE_QUERY)));
                    }

                    if let Some(first) = playbacks
                        .get(0)
                        .and_then(|e| e.get(URL))
                        .and_then(|u| u.as_str())
                    {
                        return Ok(Some(format!("{}{}", first, TYPE_QUERY)));
                    }
                }
            }
        }

        warn!("No JSON playback URLs found for {}", video_page_path);
        Ok(None)
    }
}